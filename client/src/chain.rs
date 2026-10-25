//! RPC access: sending transactions, reading markets, wallets and tokens.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use base64::Engine as _;
use matchbook_sdk::events::{self, Event};
use matchbook_sdk::ix::MarketAccounts;
use matchbook_sdk::{pda, MarketSnapshot, TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID};
use solana_commitment_config::CommitmentConfig;
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_keypair::{read_keypair_file, Keypair};
use solana_message::{Instruction, Message};
use solana_rpc_client::rpc_client::RpcClient;
use solana_rpc_client_api::config::RpcTransactionConfig;
use solana_signature::Signature;
use solana_signer::Signer;
use solana_transaction::Transaction;
use solana_transaction_status_client_types::option_serializer::OptionSerializer;
use solana_transaction_status_client_types::UiTransactionEncoding;

use crate::units::Units;

pub use solana_pubkey::Pubkey;

/// Largest serialized transaction Solana accepts, in bytes.
pub const MAX_TX_SIZE: usize = 1_232;

/// Resolves a cluster name to an RPC URL. Anything else is taken as a URL.
#[must_use]
pub fn cluster_url(cluster: &str) -> String {
    match cluster {
        "localnet" | "localhost" | "l" => "http://127.0.0.1:8899".into(),
        "devnet" | "d" => "https://api.devnet.solana.com".into(),
        "mainnet" | "mainnet-beta" | "m" => "https://api.mainnet-beta.solana.com".into(),
        "testnet" | "t" => "https://api.testnet.solana.com".into(),
        url => url.into(),
    }
}

/// The Solana CLI default wallet, `~/.config/solana/id.json`.
#[must_use]
pub fn default_keypair_path() -> PathBuf {
    let home = std::env::var_os("HOME").unwrap_or_default();
    Path::new(&home).join(".config/solana/id.json")
}

/// Reads a keypair file (a JSON array of 64 bytes, as `solana-keygen` writes).
pub fn load_keypair(path: &Path) -> Result<Keypair> {
    read_keypair_file(path)
        .map_err(|e| anyhow::anyhow!("cannot read keypair {}: {e}", path.display()))
}

/// Writes a keypair file readable only by its owner.
pub fn write_keypair(kp: &Keypair, path: &Path) -> Result<()> {
    let bytes: Vec<u8> = kp.to_bytes().to_vec();
    std::fs::write(path, serde_json::to_string(&bytes)?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// Unix time in seconds.
#[must_use]
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Compute budget settings added to every transaction.
#[derive(Debug, Clone, Copy, Default)]
pub struct Fees {
    /// Compute unit limit (0 = the runtime default).
    pub cu_limit: u32,
    /// Priority fee in micro-lamports per compute unit.
    pub priority_micro_lamports: u64,
}

/// What a confirmed transaction did.
#[derive(Debug, Clone)]
pub struct Executed {
    /// Its signature.
    pub signature: Signature,
    /// The program's return data (an order or stop id when it places one).
    pub return_data: Option<Vec<u8>>,
    /// MatchBook events it logged.
    pub events: Vec<Event>,
    /// Compute units it used.
    pub compute_units: Option<u64>,
}

impl Executed {
    /// The u64 the program returned, such as a new order or stop id.
    #[must_use]
    pub fn returned_u64(&self) -> Option<u64> {
        let d = self.return_data.as_ref()?;
        Some(u64::from_le_bytes(d.get(..8)?.try_into().ok()?))
    }
}

/// A connection to a cluster with a fee payer.
pub struct Chain {
    /// The RPC client.
    pub rpc: RpcClient,
    /// Pays fees and rent, and signs as the default owner.
    pub payer: Keypair,
    /// Compute budget settings.
    pub fees: Fees,
}

/// A market with its accounts and units, read in one call.
#[derive(Debug, Clone)]
pub struct MarketCtx {
    /// The decoded account.
    pub snap: MarketSnapshot,
    /// Mints, vaults and token programs.
    pub accounts: MarketAccounts,
    /// Lot sizes and decimals.
    pub units: Units,
}

impl MarketCtx {
    /// Seat index of `owner`, if it has one.
    pub fn seat_of(&self, owner: &Pubkey) -> Result<Option<u32>> {
        Ok(self.snap.seat(owner)?.map(|(i, _)| i))
    }

    /// Seat index of `owner`, or an error telling how to claim one.
    pub fn require_seat(&self, owner: &Pubkey) -> Result<u32> {
        self.seat_of(owner)?.with_context(|| {
            format!(
                "{owner} has no seat on {}; run `matchbook seat claim {}` first",
                self.snap.key(),
                self.snap.key()
            )
        })
    }
}

impl Chain {
    /// Connects to `url` with `payer`.
    #[must_use]
    pub fn new(url: &str, payer: Keypair, fees: Fees) -> Self {
        Self {
            rpc: RpcClient::new_with_commitment(url.to_string(), CommitmentConfig::confirmed()),
            payer,
            fees,
        }
    }

    /// The fee payer's address.
    #[must_use]
    pub fn payer_key(&self) -> Pubkey {
        self.payer.pubkey()
    }

    fn budget(&self) -> Vec<Instruction> {
        let mut out = Vec::new();
        if self.fees.cu_limit > 0 {
            out.push(ComputeBudgetInstruction::set_compute_unit_limit(
                self.fees.cu_limit,
            ));
        }
        if self.fees.priority_micro_lamports > 0 {
            out.push(ComputeBudgetInstruction::set_compute_unit_price(
                self.fees.priority_micro_lamports,
            ));
        }
        out
    }

    /// Size in bytes of the transaction [`Chain::send_from`] would build
    /// for `ixs` with `fee_payer` paying, compute budget included. Compare
    /// it with [`MAX_TX_SIZE`].
    #[must_use]
    pub fn tx_size(&self, fee_payer: &Pubkey, ixs: &[Instruction]) -> usize {
        let mut all = self.budget();
        all.extend_from_slice(ixs);
        let msg = Message::new(&all, Some(fee_payer));
        let sigs = usize::from(msg.header.num_required_signatures);
        // A short-vec length (one byte below 128) and 64 bytes a signature.
        1 + 64 * sigs + msg.serialize().len()
    }

    /// Sends `ixs` signed by the payer and `signers`, and waits for
    /// confirmation.
    pub fn send(&self, ixs: &[Instruction], signers: &[&Keypair]) -> Result<Signature> {
        self.send_from(&self.payer, ixs, signers)
    }

    /// Like [`Chain::send`] with another fee payer (a session key).
    pub fn send_from(
        &self,
        fee_payer: &Keypair,
        ixs: &[Instruction],
        signers: &[&Keypair],
    ) -> Result<Signature> {
        let mut all = self.budget();
        all.extend_from_slice(ixs);
        let mut keys: Vec<&Keypair> = vec![fee_payer];
        for s in signers {
            if s.pubkey() != fee_payer.pubkey() && !keys.iter().any(|k| k.pubkey() == s.pubkey()) {
                keys.push(s);
            }
        }
        let blockhash = self.rpc.get_latest_blockhash()?;
        let msg = Message::new(&all, Some(&fee_payer.pubkey()));
        let mut tx = Transaction::new_unsigned(msg);
        tx.try_sign(&keys, blockhash)?;
        self.rpc
            .send_and_confirm_transaction(&tx)
            .map_err(|e| explain(e.into()))
    }

    /// [`Chain::send`], then reads the transaction's return data and events.
    pub fn execute(&self, ixs: &[Instruction], signers: &[&Keypair]) -> Result<Executed> {
        let sig = self.send(ixs, signers)?;
        self.executed(sig)
    }

    /// [`Chain::send_from`], then reads the return data and events.
    pub fn execute_from(
        &self,
        fee_payer: &Keypair,
        ixs: &[Instruction],
        signers: &[&Keypair],
    ) -> Result<Executed> {
        let sig = self.send_from(fee_payer, ixs, signers)?;
        self.executed(sig)
    }

    /// Reads the return data and events of a confirmed transaction.
    pub fn executed(&self, signature: Signature) -> Result<Executed> {
        let tx = self.rpc.get_transaction_with_config(
            &signature,
            RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::Json),
                commitment: Some(CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
            },
        )?;
        let meta = tx.transaction.meta.context("transaction has no metadata")?;
        let logs: Vec<String> = match meta.log_messages {
            OptionSerializer::Some(l) => l,
            _ => Vec::new(),
        };
        let return_data = match meta.return_data {
            OptionSerializer::Some(r) => {
                Some(base64::engine::general_purpose::STANDARD.decode(r.data.0)?)
            }
            _ => None,
        };
        let compute_units = match meta.compute_units_consumed {
            OptionSerializer::Some(c) => Some(c),
            _ => None,
        };
        Ok(Executed {
            signature,
            return_data,
            events: events_from_logs(&logs),
            compute_units,
        })
    }

    /// Reads and decodes a market.
    pub fn market(&self, market: &Pubkey) -> Result<MarketCtx> {
        let acc = self
            .rpc
            .get_account(market)
            .with_context(|| format!("market {market} not found"))?;
        let snap = MarketSnapshot::new(*market, &acc.data)
            .with_context(|| format!("{market} is not a MatchBook market"))?;
        let accounts = snap.accounts()?;
        let mints = self
            .rpc
            .get_multiple_accounts(&[accounts.base_mint, accounts.quote_mint])?;
        let mut dec = mints.iter().map(|a| -> Result<u8> {
            let a = a.as_ref().context("mint not found")?;
            a.data.get(44).copied().context("not a mint")
        });
        let base_decimals = dec.next().context("mint not found")??;
        let quote_decimals = dec.next().context("mint not found")??;
        let units = Units::new(&snap.header(), base_decimals, quote_decimals);
        Ok(MarketCtx {
            snap,
            accounts,
            units,
        })
    }

    /// Re-reads only the market account, keeping the accounts and units.
    pub fn refresh(&self, ctx: &mut MarketCtx) -> Result<()> {
        let acc = self.rpc.get_account(&ctx.snap.key())?;
        ctx.snap.update(&acc.data)?;
        Ok(())
    }

    /// The token program owning `mint`.
    pub fn token_program_of(&self, mint: &Pubkey) -> Result<Pubkey> {
        let owner = self
            .rpc
            .get_account(mint)
            .with_context(|| format!("mint {mint} not found"))?
            .owner;
        if owner == TOKEN_PROGRAM_ID || owner == TOKEN_2022_PROGRAM_ID {
            Ok(owner)
        } else {
            bail!("{mint} is not a token mint (owner {owner})")
        }
    }

    /// Token balance in atoms of an account, 0 if it does not exist.
    pub fn token_balance(&self, token_account: &Pubkey) -> Result<u64> {
        match self.rpc.get_account(token_account) {
            Ok(a) => Ok(a
                .data
                .get(64..72)
                .and_then(|b| b.try_into().ok())
                .map_or(0, u64::from_le_bytes)),
            Err(_) => Ok(0),
        }
    }

    /// Lamports of an account.
    pub fn sol_balance(&self, key: &Pubkey) -> Result<u64> {
        Ok(self.rpc.get_balance(key)?)
    }

    /// Requests an airdrop and waits for it.
    pub fn airdrop(&self, to: &Pubkey, lamports: u64) -> Result<Signature> {
        let sig = self.rpc.request_airdrop(to, lamports)?;
        for _ in 0..60 {
            if self.rpc.confirm_transaction(&sig)? {
                return Ok(sig);
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        bail!("airdrop {sig} was not confirmed")
    }

    /// Creates a mint with `decimals`, the payer as mint authority.
    pub fn create_mint(&self, decimals: u8, token_program: &Pubkey) -> Result<Pubkey> {
        let mint = Keypair::new();
        // A mint without extensions is 82 bytes under both token programs.
        let space = 82;
        let rent = self.rpc.get_minimum_balance_for_rent_exemption(space)?;
        let ixs = [
            solana_system_interface::instruction::create_account(
                &self.payer_key(),
                &mint.pubkey(),
                rent,
                space as u64,
                token_program,
            ),
            spl_token_interface::instruction::initialize_mint2(
                token_program,
                &mint.pubkey(),
                &self.payer_key(),
                None,
                decimals,
            )?,
        ];
        self.send(&ixs, &[&mint])?;
        Ok(mint.pubkey())
    }

    /// Instruction creating `owner`'s associated token account if missing.
    #[must_use]
    pub fn ensure_ata_ix(
        &self,
        owner: &Pubkey,
        mint: &Pubkey,
        token_program: &Pubkey,
    ) -> Instruction {
        spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent(
            &self.payer_key(),
            owner,
            mint,
            token_program,
        )
    }

    /// Mints `atoms` to `owner`'s associated token account, creating it.
    pub fn mint_to(&self, mint: &Pubkey, owner: &Pubkey, atoms: u64) -> Result<Pubkey> {
        let tp = self.token_program_of(mint)?;
        let ata = pda::associated_token_address(owner, mint, &tp);
        let ixs = [
            self.ensure_ata_ix(owner, mint, &tp),
            spl_token_interface::instruction::mint_to(
                &tp,
                mint,
                &ata,
                &self.payer_key(),
                &[],
                atoms,
            )?,
        ];
        self.send(&ixs, &[])?;
        Ok(ata)
    }

    /// Transfers lamports from the payer.
    pub fn transfer_sol(&self, to: &Pubkey, lamports: u64) -> Result<Signature> {
        let ix = solana_system_interface::instruction::transfer(&self.payer_key(), to, lamports);
        self.send(&[ix], &[])
    }
}

/// Decodes the MatchBook events in a transaction's logs.
#[must_use]
pub fn events_from_logs(logs: &[String]) -> Vec<Event> {
    logs.iter()
        .filter_map(|l| l.strip_prefix("Program data: "))
        .filter_map(|b| {
            base64::engine::general_purpose::STANDARD
                .decode(b.trim())
                .ok()
        })
        .filter_map(|d| events::decode(&d).ok())
        .flatten()
        .collect()
}

/// Replaces a failed MatchBook transaction's RPC error with the program's error name.
fn explain(e: anyhow::Error) -> anyhow::Error {
    let msg = format!("{e:#}");
    if let Some(code) = msg.split("custom program error: 0x").nth(1).and_then(|s| {
        u32::from_str_radix(s.split(|c: char| !c.is_ascii_hexdigit()).next()?, 16).ok()
    }) {
        // The RPC error repeats the simulation logs; the name says it all.
        return anyhow::anyhow!("MatchBook error {code}: {}", error_name(code));
    }
    e
}

/// Name of a MatchBook program error code.
#[must_use]
pub fn error_name(code: u32) -> String {
    matchbook_sdk::error_name(code).map_or_else(|| format!("unknown ({code})"), str::to_string)
}
