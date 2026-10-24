//! LiteSVM harness for the Matchbook program.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    missing_docs
)]

use litesvm::{
    types::{FailedTransactionMetadata, TransactionMetadata},
    LiteSVM,
};
use litesvm_token::CreateMint;
use matchbook_program::engine::{Market, MarketTotals, TraderLeaf};
use matchbook_program::instruction::{build, CreateMarketArgs, OrderArgs, SwapArgs};

/// Swap request as tests write it (the side travels in the instruction header).
#[derive(Clone, Copy)]
pub struct Swap {
    pub side: u8,
    pub base_lots: u64,
    pub quote_budget: u64,
    pub limit_price: u64,
    pub min_out: u64,
    pub match_limit: u32,
}
use solana_address::Address;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;

pub const TOKEN: Address = Address::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
pub const TOKEN_2022: Address =
    Address::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
pub const ATA: Address = Address::from_str_const("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
pub const SYSTEM: Address = Address::from_str_const("11111111111111111111111111111111");

pub fn program_id() -> Address {
    Address::new_from_array(matchbook_program::ID.to_bytes())
}

pub fn ata(owner: &Address, mint: &Address, tp: &Address) -> Address {
    Address::find_program_address(&[owner.as_ref(), tp.as_ref(), mint.as_ref()], &ATA).0
}

/// Creates a Token-2022 mint with no extensions (82 bytes).
pub fn create_mint_2022(svm: &mut LiteSVM, payer: &Keypair, decimals: u8) -> Address {
    let mint = Keypair::new();
    let lamports = svm.minimum_balance_for_rent_exemption(82);
    let mut create = vec![0u8, 0, 0, 0];
    create.extend_from_slice(&lamports.to_le_bytes());
    create.extend_from_slice(&82u64.to_le_bytes());
    create.extend_from_slice(TOKEN_2022.as_ref());
    let ix1 = Instruction::new_with_bytes(
        SYSTEM,
        &create,
        vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(mint.pubkey(), true),
        ],
    );
    let mut init = vec![20u8, decimals];
    init.extend_from_slice(payer.pubkey().as_ref());
    init.push(0);
    let ix2 = Instruction::new_with_bytes(
        TOKEN_2022,
        &init,
        vec![AccountMeta::new(mint.pubkey(), false)],
    );
    let msg = Message::new(&[ix1, ix2], Some(&payer.pubkey()));
    let tx = Transaction::new(&[payer, &mint], msg, svm.latest_blockhash());
    svm.send_transaction(tx).unwrap();
    mint.pubkey()
}

pub struct User {
    pub kp: Keypair,
    pub base: Address,
    pub quote: Address,
}

impl User {
    pub fn key(&self) -> Address {
        self.kp.pubkey()
    }
}

pub struct H {
    pub svm: LiteSVM,
    pub payer: Keypair,
    pub authority: Keypair,
    pub market: Address,
    pub base_mint: Address,
    pub quote_mint: Address,
    pub base_tp: Address,
    pub quote_tp: Address,
    pub vault_auth: Address,
    pub base_vault: Address,
    pub quote_vault: Address,
    pub last_cu: u64,
}

pub fn so_path() -> String {
    format!(
        "{}/../target/deploy/matchbook_program.so",
        env!("CARGO_MANIFEST_DIR")
    )
}

impl H {
    pub fn new(args: CreateMarketArgsLite, quote_2022: bool) -> Self {
        let mut svm = LiteSVM::new();
        svm.add_program_from_file(program_id(), so_path())
            .expect("run `cargo build-sbf` first");
        let payer = Keypair::new();
        let authority = Keypair::new();
        svm.airdrop(&payer.pubkey(), 100_000_000_000).unwrap();
        svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();
        let base_tp = TOKEN;
        let quote_tp = if quote_2022 { TOKEN_2022 } else { TOKEN };
        let base_mint = CreateMint::new(&mut svm, &payer)
            .decimals(9)
            .token_program_id(&base_tp)
            .send()
            .unwrap();
        let quote_mint = if quote_2022 {
            create_mint_2022(&mut svm, &payer, 6)
        } else {
            CreateMint::new(&mut svm, &payer)
                .decimals(6)
                .token_program_id(&quote_tp)
                .send()
                .unwrap()
        };
        let market_kp = Keypair::new();
        let market = market_kp.pubkey();
        let vault_auth = Address::find_program_address(
            &[matchbook_program::VAULT_AUTHORITY_SEED, market.as_ref()],
            &program_id(),
        )
        .0;
        let base_vault = ata(&vault_auth, &base_mint, &base_tp);
        let quote_vault = ata(&vault_auth, &quote_mint, &quote_tp);
        let mut h = Self {
            svm,
            payer,
            authority,
            market,
            base_mint,
            quote_mint,
            base_tp,
            quote_tp,
            vault_auth,
            base_vault,
            quote_vault,
            last_cu: 0,
        };
        let data = build::create_market(&CreateMarketArgs {
            base_lot_size: args.base_lot_size,
            quote_lot_size: args.quote_lot_size,
            min_base_lots: 1,
            taker_fee_bps: args.taker_fee_bps,
            maker_rebate_bps: args.maker_rebate_bps,
            initial_nodes: args.initial_nodes,
            authority: h.authority.pubkey().to_bytes(),
            reserved: 0,
        });
        let mut ixs = vec![];
        if args.initial_nodes > 100 {
            let space = matchbook_program::engine::market_size(args.initial_nodes as usize) as u64;
            let lamports = h.svm.minimum_balance_for_rent_exemption(space as usize);
            let mut create = vec![0u8, 0, 0, 0];
            create.extend_from_slice(&lamports.to_le_bytes());
            create.extend_from_slice(&space.to_le_bytes());
            create.extend_from_slice(program_id().as_ref());
            ixs.push(Instruction::new_with_bytes(
                SYSTEM,
                &create,
                vec![
                    AccountMeta::new(h.payer.pubkey(), true),
                    AccountMeta::new(market, true),
                ],
            ));
        }
        let ix = Instruction::new_with_bytes(
            program_id(),
            &data,
            vec![
                AccountMeta::new(h.payer.pubkey(), true),
                AccountMeta::new(market, true),
                AccountMeta::new_readonly(base_mint, false),
                AccountMeta::new_readonly(quote_mint, false),
                AccountMeta::new(base_vault, false),
                AccountMeta::new(quote_vault, false),
                AccountMeta::new_readonly(vault_auth, false),
                AccountMeta::new_readonly(base_tp, false),
                AccountMeta::new_readonly(quote_tp, false),
                AccountMeta::new_readonly(ATA, false),
                AccountMeta::new_readonly(SYSTEM, false),
            ],
        );
        ixs.push(ix);
        h.send(&ixs, &[&market_kp]).expect("create market");
        h
    }

    pub fn send(
        &mut self,
        ixs: &[Instruction],
        extra: &[&Keypair],
    ) -> Result<TransactionMetadata, FailedTransactionMetadata> {
        self.svm.expire_blockhash();
        let mut signers: Vec<&Keypair> = vec![&self.payer];
        signers.extend_from_slice(extra);
        let msg = Message::new(ixs, Some(&self.payer.pubkey()));
        let tx = Transaction::new(&signers, msg, self.svm.latest_blockhash());
        let r = self.svm.send_transaction(tx);
        if let Ok(m) = &r {
            self.last_cu = m.compute_units_consumed;
        }
        r
    }

    pub fn user(&mut self, base: u64, quote: u64) -> User {
        let kp = Keypair::new();
        self.svm.airdrop(&kp.pubkey(), 10_000_000_000).unwrap();
        let b = self.create_ata(&kp.pubkey(), self.base_mint, self.base_tp);
        let q = self.create_ata(&kp.pubkey(), self.quote_mint, self.quote_tp);
        self.mint_to(self.base_mint, b, self.base_tp, base);
        self.mint_to(self.quote_mint, q, self.quote_tp, quote);
        User {
            kp,
            base: b,
            quote: q,
        }
    }

    pub fn create_ata(&mut self, owner: &Address, mint: Address, tp: Address) -> Address {
        let a = ata(owner, &mint, &tp);
        let ix = Instruction::new_with_bytes(
            ATA,
            &[1],
            vec![
                AccountMeta::new(self.payer.pubkey(), true),
                AccountMeta::new(a, false),
                AccountMeta::new_readonly(*owner, false),
                AccountMeta::new_readonly(mint, false),
                AccountMeta::new_readonly(SYSTEM, false),
                AccountMeta::new_readonly(tp, false),
            ],
        );
        self.send(&[ix], &[]).unwrap();
        a
    }

    pub fn mint_to(&mut self, mint: Address, dest: Address, tp: Address, amount: u64) {
        if amount == 0 {
            return;
        }
        let mut d = vec![7u8];
        d.extend_from_slice(&amount.to_le_bytes());
        let ix = Instruction::new_with_bytes(
            tp,
            &d,
            vec![
                AccountMeta::new(mint, false),
                AccountMeta::new(dest, false),
                AccountMeta::new_readonly(self.payer.pubkey(), true),
            ],
        );
        self.send(&[ix], &[]).unwrap();
    }

    pub fn ix(&self, data: Vec<u8>, accounts: Vec<AccountMeta>) -> Instruction {
        Instruction::new_with_bytes(program_id(), &data, accounts)
    }

    pub fn claim_seat(
        &mut self,
        u: &User,
    ) -> Result<TransactionMetadata, FailedTransactionMetadata> {
        let ix = self.ix(
            build::claim_seat(),
            vec![
                AccountMeta::new(u.key(), true),
                AccountMeta::new_readonly(u.key(), true),
                AccountMeta::new(self.market, false),
                AccountMeta::new_readonly(SYSTEM, false),
            ],
        );
        self.send(&[ix], &[&u.kp])
    }

    pub fn deposit(
        &mut self,
        u: &User,
        base: bool,
        amount: u64,
    ) -> Result<TransactionMetadata, FailedTransactionMetadata> {
        let (tok, vault, mint, tp) = self.leg(u, base);
        let ix = self.ix(
            build::deposit(u32::MAX, amount),
            vec![
                AccountMeta::new_readonly(u.key(), true),
                AccountMeta::new(self.market, false),
                AccountMeta::new(tok, false),
                AccountMeta::new(vault, false),
                AccountMeta::new_readonly(mint, false),
                AccountMeta::new_readonly(tp, false),
            ],
        );
        self.send(&[ix], &[&u.kp])
    }

    pub fn withdraw(
        &mut self,
        u: &User,
        base: bool,
        amount: u64,
    ) -> Result<TransactionMetadata, FailedTransactionMetadata> {
        let (tok, vault, mint, tp) = self.leg(u, base);
        let ix = self.ix(
            build::withdraw(u32::MAX, amount),
            vec![
                AccountMeta::new_readonly(u.key(), true),
                AccountMeta::new(self.market, false),
                AccountMeta::new(tok, false),
                AccountMeta::new(vault, false),
                AccountMeta::new_readonly(mint, false),
                AccountMeta::new_readonly(self.vault_auth, false),
                AccountMeta::new_readonly(tp, false),
            ],
        );
        self.send(&[ix], &[&u.kp])
    }

    fn leg(&self, u: &User, base: bool) -> (Address, Address, Address, Address) {
        if base {
            (u.base, self.base_vault, self.base_mint, self.base_tp)
        } else {
            (u.quote, self.quote_vault, self.quote_mint, self.quote_tp)
        }
    }

    pub fn owner_metas(&self, u: &User) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(u.key(), true),
            AccountMeta::new(self.market, false),
            AccountMeta::new_readonly(SYSTEM, false),
        ]
    }

    pub fn place(
        &mut self,
        u: &User,
        o: OrderArgs,
    ) -> Result<TransactionMetadata, FailedTransactionMetadata> {
        let ix = self.ix(
            build::place_order(self.seat(u).unwrap_or(u32::MAX), &o),
            self.owner_metas(u),
        );
        self.send(&[ix], &[&u.kp])
    }

    pub fn batch(
        &mut self,
        u: &User,
        cancel_all: u8,
        cancels: &[u64],
        orders: &[OrderArgs],
    ) -> Result<TransactionMetadata, FailedTransactionMetadata> {
        let ix = self.ix(
            build::batch_update(
                self.seat(u).unwrap_or(u32::MAX),
                cancel_all,
                cancels,
                orders,
            ),
            self.owner_metas(u),
        );
        self.send(&[ix], &[&u.kp])
    }

    pub fn cancel_ix(&self, u: &User, data: Vec<u8>) -> Instruction {
        self.ix(
            data,
            vec![
                AccountMeta::new_readonly(u.key(), true),
                AccountMeta::new(self.market, false),
            ],
        )
    }

    pub fn swap(
        &mut self,
        u: &User,
        s: Swap,
    ) -> Result<TransactionMetadata, FailedTransactionMetadata> {
        let a = SwapArgs {
            base_lots: s.base_lots,
            quote_budget: s.quote_budget,
            limit_price: s.limit_price,
            min_out: s.min_out,
            match_limit: s.match_limit,
            reserved: 0,
        };
        let ix = self.ix(
            build::swap(s.side, &a),
            vec![
                AccountMeta::new_readonly(u.key(), true),
                AccountMeta::new(self.market, false),
                AccountMeta::new(u.base, false),
                AccountMeta::new(u.quote, false),
                AccountMeta::new(self.base_vault, false),
                AccountMeta::new(self.quote_vault, false),
                AccountMeta::new_readonly(self.base_mint, false),
                AccountMeta::new_readonly(self.quote_mint, false),
                AccountMeta::new_readonly(self.vault_auth, false),
                AccountMeta::new_readonly(self.base_tp, false),
                AccountMeta::new_readonly(self.quote_tp, false),
            ],
        );
        self.send(&[ix], &[&u.kp])
    }

    pub fn collect_fees(
        &mut self,
        signer: &Keypair,
        dest: Address,
    ) -> Result<TransactionMetadata, FailedTransactionMetadata> {
        let ix = self.ix(
            build::collect_fees(),
            vec![
                AccountMeta::new_readonly(signer.pubkey(), true),
                AccountMeta::new(self.market, false),
                AccountMeta::new(dest, false),
                AccountMeta::new(self.quote_vault, false),
                AccountMeta::new_readonly(self.quote_mint, false),
                AccountMeta::new_readonly(self.vault_auth, false),
                AccountMeta::new_readonly(self.quote_tp, false),
            ],
        );
        let kp = signer.insecure_clone();
        self.send(&[ix], &[&kp])
    }

    pub fn data(&self) -> Vec<u8> {
        self.svm.get_account(&self.market).unwrap().data
    }

    pub fn with_market<R>(&self, f: impl FnOnce(&Market<'_>) -> R) -> R {
        let mut d = self.data();
        let m = Market::load_mut(&mut d).unwrap();
        f(&m)
    }

    pub fn seat(&self, u: &User) -> Option<u32> {
        self.with_market(|m| m.find_trader(&u.key().to_bytes()).unwrap())
    }

    pub fn trader(&self, u: &User) -> TraderLeaf {
        let s = self.seat(u).unwrap();
        self.with_market(|m| *m.trader(s).unwrap())
    }

    pub fn token(&self, a: &Address) -> u64 {
        let d = self.svm.get_account(a).unwrap().data;
        u64::from_le_bytes(d[64..72].try_into().unwrap())
    }

    /// Asserts engine invariants and that vaults hold exactly what is owed.
    pub fn audit(&self) -> MarketTotals {
        let t = self.with_market(|m| m.check_invariants().unwrap());
        assert_eq!(
            u128::from(self.token(&self.base_vault)),
            t.base_atoms,
            "base vault"
        );
        assert_eq!(
            u128::from(self.token(&self.quote_vault)),
            t.quote_atoms,
            "quote vault"
        );
        t
    }
}

#[derive(Clone, Copy)]
pub struct CreateMarketArgsLite {
    pub base_lot_size: u64,
    pub quote_lot_size: u64,
    pub taker_fee_bps: u32,
    pub maker_rebate_bps: u32,
    pub initial_nodes: u32,
}

pub fn default_market() -> CreateMarketArgsLite {
    // SOL/USDC-like: 1 lot = 0.001 SOL, 1 quote lot = 0.000001 USDC.
    CreateMarketArgsLite {
        base_lot_size: 1_000_000,
        quote_lot_size: 1,
        taker_fee_bps: 10,
        maker_rebate_bps: 2,
        initial_nodes: 0,
    }
}

pub fn limit(side: u8, price: u64, lots: u64) -> OrderArgs {
    OrderArgs::limit(side, price, lots)
}

pub fn err_code(r: &Result<TransactionMetadata, FailedTransactionMetadata>) -> Option<u32> {
    match r {
        Err(e) => match &e.err {
            solana_transaction_error_shim::TransactionError::InstructionError(
                _,
                solana_transaction_error_shim::InstructionError::Custom(c),
            ) => Some(*c),
            _ => None,
        },
        Ok(_) => None,
    }
}

pub mod solana_transaction_error_shim {
    pub use solana_instruction_error::InstructionError;
    pub use solana_transaction_error::TransactionError;
}
