//! Runs the same compute-unit scenarios as `bench.rs` against Phoenix v1, built from source
//! by `scripts/build-competitors.sh`. Skipped when the competitor binary is missing.
//!
//! Phoenix seats are permissioned: the market authority approves each trader, so the seat
//! row measures the trader's own `RequestSeat` and leaves the approval out.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    missing_docs
)]

mod common;

use common::*;
use litesvm::LiteSVM;
use litesvm_token::CreateMint;
use solana_address::Address;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;

const PHOENIX: Address = Address::from_str_const("PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY");
/// `size_of::<MarketHeader>()` + the `FIFOMarket<Pubkey, 2048, 2048, 128>` it dispatches to.
const HEADER_LEN: usize = 576;
const MARKET_LEN: usize = 280_976;
/// `order_sequence_number` sits after the market's 32-word padding and two u64 fields.
const SEQ_OFFSET: usize = HEADER_LEN + 32 * 8 + 16;
const LOT: u64 = 1_000_000;

const BID: u8 = 0;
const ASK: u8 = 1;

/// Tags of the instructions used here.
mod ix {
    pub const SWAP: u8 = 0;
    pub const SWAP_WITH_FREE_FUNDS: u8 = 1;
    pub const PLACE_LIMIT_WITH_FREE_FUNDS: u8 = 3;
    pub const CANCEL_BY_ID_WITH_FREE_FUNDS: u8 = 11;
    pub const WITHDRAW: u8 = 12;
    pub const DEPOSIT: u8 = 13;
    pub const REQUEST_SEAT: u8 = 14;
    pub const PLACE_MULTIPLE_POST_ONLY_WITH_FREE_FUNDS: u8 = 17;
    pub const INITIALIZE_MARKET: u8 = 100;
    pub const CHANGE_MARKET_STATUS: u8 = 103;
    pub const CHANGE_SEAT_STATUS: u8 = 104;
}

/// Borsh writer for the handful of shapes Phoenix uses.
#[derive(Default)]
struct W(Vec<u8>);

impl W {
    fn new(tag: u8) -> Self {
        Self(vec![tag])
    }
    fn u8(mut self, v: u8) -> Self {
        self.0.push(v);
        self
    }
    fn u32(mut self, v: u32) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn u64(mut self, v: u64) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn u128(mut self, v: u128) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn opt_u64(self, v: Option<u64>) -> Self {
        match v {
            Some(x) => self.u8(1).u64(x),
            None => self.u8(0),
        }
    }
    fn bytes(mut self, v: &[u8]) -> Self {
        self.0.extend_from_slice(v);
        self
    }
}

/// `OrderPacket::Limit` resting with deposited funds.
fn limit(side: u8, price: u64, lots: u64) -> Vec<u8> {
    W::new(1)
        .u8(side)
        .u64(price)
        .u64(lots)
        .u8(1) // SelfTradeBehavior::CancelProvide
        .opt_u64(None)
        .u128(0)
        .u8(1) // use_only_deposited_funds
        .opt_u64(None)
        .opt_u64(None)
        .u8(0)
        .0
}

/// `OrderPacket::PostOnly` resting with deposited funds.
fn post_only(side: u8, price: u64, lots: u64) -> Vec<u8> {
    W::new(0)
        .u8(side)
        .u64(price)
        .u64(lots)
        .u128(0)
        .u8(1) // reject_post_only
        .u8(1) // use_only_deposited_funds
        .opt_u64(None)
        .opt_u64(None)
        .u8(0)
        .0
}

/// `OrderPacket::ImmediateOrCancel`.
fn ioc(side: u8, price: Option<u64>, base_lots: u64, quote_lots: u64, deposited: bool) -> Vec<u8> {
    W::new(2)
        .u8(side)
        .opt_u64(price)
        .u64(base_lots)
        .u64(quote_lots)
        .u64(0)
        .u64(0)
        .u8(1)
        .opt_u64(None)
        .u128(0)
        .u8(deposited as u8)
        .opt_u64(None)
        .opt_u64(None)
        .0
}

fn vault(market: &Address, mint: &Address) -> Address {
    Address::find_program_address(&[b"vault", market.as_ref(), mint.as_ref()], &PHOENIX).0
}

fn seat(market: &Address, trader: &Address) -> Address {
    Address::find_program_address(&[b"seat", market.as_ref(), trader.as_ref()], &PHOENIX).0
}

struct P {
    svm: LiteSVM,
    payer: Keypair,
    market: Address,
    log_authority: Address,
    base_mint: Address,
    quote_mint: Address,
    last_cu: u64,
}

impl P {
    fn new(path: &str) -> Self {
        let mut svm = LiteSVM::new();
        svm.add_program_from_file(PHOENIX, path).unwrap();
        let payer = Keypair::new();
        svm.airdrop(&payer.pubkey(), 1_000_000_000_000).unwrap();
        let base_mint = CreateMint::new(&mut svm, &payer)
            .decimals(9)
            .send()
            .unwrap();
        let quote_mint = CreateMint::new(&mut svm, &payer)
            .decimals(6)
            .send()
            .unwrap();
        let market_kp = Keypair::new();
        let market = market_kp.pubkey();
        let mut p = Self {
            svm,
            payer,
            market,
            log_authority: Address::find_program_address(&[b"log"], &PHOENIX).0,
            base_mint,
            quote_mint,
            last_cu: 0,
        };
        // Phoenix markets are allocated at full size up front.
        let space = (HEADER_LEN + MARKET_LEN) as u64;
        let lamports = p.svm.minimum_balance_for_rent_exemption(space as usize);
        let mut create = vec![0u8, 0, 0, 0];
        create.extend_from_slice(&lamports.to_le_bytes());
        create.extend_from_slice(&space.to_le_bytes());
        create.extend_from_slice(PHOENIX.as_ref());
        let ix1 = Instruction::new_with_bytes(
            SYSTEM,
            &create,
            vec![
                AccountMeta::new(p.payer.pubkey(), true),
                AccountMeta::new(market, true),
            ],
        );
        // Base lot = 1e6 atoms (1,000 per SOL), quote lot = 1 atom, and one tick is one quote
        // lot per base lot, so prices read the same as in the other benches. Fee: 5 bps.
        let params = W::new(ix::INITIALIZE_MARKET)
            .u64(2048)
            .u64(2048)
            .u64(128)
            .u64(1_000_000)
            .u64(1_000)
            .u64(1_000)
            .bytes(&5u16.to_le_bytes())
            .bytes(p.payer.pubkey().as_ref())
            .u8(0)
            .0;
        let ix2 = Instruction::new_with_bytes(
            PHOENIX,
            &params,
            vec![
                AccountMeta::new_readonly(PHOENIX, false),
                AccountMeta::new_readonly(p.log_authority, false),
                AccountMeta::new(market, false),
                AccountMeta::new(p.payer.pubkey(), true),
                AccountMeta::new_readonly(base_mint, false),
                AccountMeta::new_readonly(quote_mint, false),
                AccountMeta::new(vault(&market, &base_mint), false),
                AccountMeta::new(vault(&market, &quote_mint), false),
                AccountMeta::new_readonly(SYSTEM, false),
                AccountMeta::new_readonly(TOKEN, false),
            ],
        );
        p.send(&[ix1, ix2], &[&market_kp]).expect("create market");
        // Markets start post-only; open them to takers.
        let ix = Instruction::new_with_bytes(
            PHOENIX,
            &[ix::CHANGE_MARKET_STATUS, 1],
            p.head(p.payer.pubkey()),
        );
        p.send(&[ix], &[]).expect("activate market");
        p
    }

    /// Phoenix program, log authority, market, signer.
    fn head(&self, signer: Address) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new_readonly(PHOENIX, false),
            AccountMeta::new_readonly(self.log_authority, false),
            AccountMeta::new(self.market, false),
            AccountMeta::new_readonly(signer, true),
        ]
    }

    /// `head` followed by token accounts, vaults and the token program.
    fn with_tokens(&self, u: &User, seat_acct: bool) -> Vec<AccountMeta> {
        let mut v = self.head(u.key());
        if seat_acct {
            v.push(AccountMeta::new_readonly(
                seat(&self.market, &u.key()),
                false,
            ));
        }
        v.extend([
            AccountMeta::new(u.base, false),
            AccountMeta::new(u.quote, false),
            AccountMeta::new(vault(&self.market, &self.base_mint), false),
            AccountMeta::new(vault(&self.market, &self.quote_mint), false),
            AccountMeta::new_readonly(TOKEN, false),
        ]);
        v
    }

    fn send(&mut self, ixs: &[Instruction], extra: &[&Keypair]) -> Result<(), String> {
        self.svm.expire_blockhash();
        let mut signers: Vec<&Keypair> = vec![&self.payer];
        signers.extend_from_slice(extra);
        let msg = Message::new(ixs, Some(&self.payer.pubkey()));
        let tx = Transaction::new(&signers, msg, self.svm.latest_blockhash());
        match self.svm.send_transaction(tx) {
            Ok(meta) => {
                self.last_cu = meta.compute_units_consumed;
                Ok(())
            }
            Err(e) => Err(format!("{:?}\n{}", e.err, e.meta.logs.join("\n"))),
        }
    }

    fn user(&mut self, base: u64, quote: u64) -> User {
        let kp = Keypair::new();
        self.svm.airdrop(&kp.pubkey(), 10_000_000_000).unwrap();
        let b = self.ata(&kp.pubkey(), self.base_mint, base);
        let q = self.ata(&kp.pubkey(), self.quote_mint, quote);
        User {
            kp,
            base: b,
            quote: q,
        }
    }

    fn ata(&mut self, owner: &Address, mint: Address, amount: u64) -> Address {
        let a = ata(owner, &mint, &TOKEN);
        let create = Instruction::new_with_bytes(
            ATA,
            &[1],
            vec![
                AccountMeta::new(self.payer.pubkey(), true),
                AccountMeta::new(a, false),
                AccountMeta::new_readonly(*owner, false),
                AccountMeta::new_readonly(mint, false),
                AccountMeta::new_readonly(SYSTEM, false),
                AccountMeta::new_readonly(TOKEN, false),
            ],
        );
        let mut d = vec![7u8];
        d.extend_from_slice(&amount.to_le_bytes());
        let mint_to = Instruction::new_with_bytes(
            TOKEN,
            &d,
            vec![
                AccountMeta::new(mint, false),
                AccountMeta::new(a, false),
                AccountMeta::new_readonly(self.payer.pubkey(), true),
            ],
        );
        self.send(&[create, mint_to], &[]).unwrap();
        a
    }

    /// Requests a seat as the trader (measured), then approves it as the authority.
    fn claim_seat(&mut self, u: &User) -> u64 {
        let s = seat(&self.market, &u.key());
        let mut accts = self.head(u.key());
        accts[3] = AccountMeta::new(u.key(), true);
        accts.push(AccountMeta::new(s, false));
        accts.push(AccountMeta::new_readonly(SYSTEM, false));
        let ix = Instruction::new_with_bytes(PHOENIX, &[ix::REQUEST_SEAT], accts);
        self.send(&[ix], &[&u.kp]).unwrap();
        let cu = self.last_cu;
        let mut accts = self.head(self.payer.pubkey());
        accts.push(AccountMeta::new(s, false));
        let ix = Instruction::new_with_bytes(PHOENIX, &[ix::CHANGE_SEAT_STATUS, 1], accts);
        self.send(&[ix], &[]).unwrap();
        cu
    }

    fn deposit(&mut self, u: &User, base_lots: u64, quote_lots: u64) {
        let d = W::new(ix::DEPOSIT).u64(quote_lots).u64(base_lots).0;
        let ix = Instruction::new_with_bytes(PHOENIX, &d, self.with_tokens(u, true));
        self.send(&[ix], &[&u.kp]).unwrap();
    }

    fn withdraw(&mut self, u: &User, base_lots: u64) {
        let d = W::new(ix::WITHDRAW)
            .opt_u64(Some(0))
            .opt_u64(Some(base_lots))
            .0;
        let ix = Instruction::new_with_bytes(PHOENIX, &d, self.with_tokens(u, false));
        self.send(&[ix], &[&u.kp]).unwrap();
    }

    fn seat_ix(&self, u: &User, data: Vec<u8>) -> Instruction {
        let mut accts = self.head(u.key());
        accts.push(AccountMeta::new_readonly(
            seat(&self.market, &u.key()),
            false,
        ));
        Instruction::new_with_bytes(PHOENIX, &data, accts)
    }

    fn sequence_number(&self) -> u64 {
        let a = self.svm.get_account(&self.market).unwrap();
        u64::from_le_bytes(a.data[SEQ_OFFSET..SEQ_OFFSET + 8].try_into().unwrap())
    }

    /// Places a limit or post-only order from deposited funds.
    fn place(&mut self, u: &User, packet: Vec<u8>) {
        let mut d = vec![ix::PLACE_LIMIT_WITH_FREE_FUNDS];
        d.extend(packet);
        let ix = self.seat_ix(u, d);
        self.send(&[ix], &[&u.kp]).unwrap();
    }

    /// IOC from deposited funds.
    fn take(&mut self, u: &User, packet: Vec<u8>) {
        let mut d = vec![ix::SWAP_WITH_FREE_FUNDS];
        d.extend(packet);
        let ix = self.seat_ix(u, d);
        self.send(&[ix], &[&u.kp]).unwrap();
    }

    /// IOC against the trader's token accounts, no seat.
    fn swap(&mut self, u: &User, packet: Vec<u8>) {
        let mut d = vec![ix::SWAP];
        d.extend(packet);
        let ix = Instruction::new_with_bytes(PHOENIX, &d, self.with_tokens(u, false));
        self.send(&[ix], &[&u.kp]).unwrap();
    }

    fn multiple_ix(&self, u: &User, bids: &[(u64, u64)], asks: &[(u64, u64)]) -> Instruction {
        let mut w = W::new(ix::PLACE_MULTIPLE_POST_ONLY_WITH_FREE_FUNDS);
        for list in [bids, asks] {
            w = w.u32(list.len() as u32);
            for &(price, lots) in list {
                w = w.u64(price).u64(lots).opt_u64(None).opt_u64(None);
            }
        }
        // client_order_id: None, FailOnInsufficientFundsAndFailOnCross.
        let d = w.u8(0).u8(1).0;
        self.seat_ix(u, d)
    }

    /// Places post-only orders and returns `(side, price, sequence number)` for each, in the
    /// order Phoenix assigns them (bids first).
    fn multiple(
        &mut self,
        u: &User,
        bids: &[(u64, u64)],
        asks: &[(u64, u64)],
    ) -> Vec<(u8, u64, u64)> {
        let seq = self.sequence_number();
        let ix = self.multiple_ix(u, bids, asks);
        self.send(&[ix], &[&u.kp]).unwrap();
        bids.iter()
            .map(|&(p, _)| (BID, p))
            .chain(asks.iter().map(|&(p, _)| (ASK, p)))
            .zip(seq..)
            .map(|((side, p), s)| (side, p, if side == BID { !s } else { s }))
            .collect()
    }

    fn cancel_ix(&self, u: &User, orders: &[(u8, u64, u64)]) -> Instruction {
        let mut w = W::new(ix::CANCEL_BY_ID_WITH_FREE_FUNDS).u32(orders.len() as u32);
        for &(side, price, seq) in orders {
            w = w.u8(side).u64(price).u64(seq);
        }
        let mut accts = self.head(u.key());
        accts.truncate(4);
        Instruction::new_with_bytes(PHOENIX, &w.0, accts)
    }
}

#[test]
fn phoenix_compute_units() {
    let path = format!(
        "{}/../target/competitors/phoenix.so",
        env!("CARGO_MANIFEST_DIR")
    );
    if !std::path::Path::new(&path).exists() {
        eprintln!("skipping: {path} not found, run scripts/build-competitors.sh");
        return;
    }
    let mut rows: Vec<(String, u64)> = vec![];
    let mut h = P::new(&path);
    let mm = h.user(100_000 * LOT, 100_000_000_000_000);
    let taker = h.user(100_000 * LOT, 100_000_000_000_000);
    let cu = h.claim_seat(&mm);
    rows.push((
        "claim_seat (request only; authority must approve)".into(),
        cu,
    ));
    h.claim_seat(&taker);
    h.deposit(&mm, 100_000, 0);
    rows.push(("deposit".into(), h.last_cu));
    h.deposit(&mm, 0, 100_000_000_000_000);
    h.deposit(&taker, 100_000, 100_000_000_000_000);

    h.place(&mm, limit(ASK, 1_000_000, 10));
    rows.push(("place_order: rest, empty book".into(), h.last_cu));
    for i in 0..125u64 {
        let asks: Vec<(u64, u64)> = (0..8).map(|k| (1_000_100 + i * 8 + k, 10)).collect();
        h.multiple(&mm, &[], &asks);
        let bids: Vec<(u64, u64)> = (0..8).map(|k| (999_900 - i * 8 - k, 10)).collect();
        h.multiple(&mm, &bids, &[]);
    }
    h.place(&mm, limit(ASK, 1_500_000, 10));
    rows.push((
        "place_order: rest, book with 2,000 orders".into(),
        h.last_cu,
    ));
    let seq = h.sequence_number();
    h.place(&mm, post_only(ASK, 1_400_000, 10));
    rows.push(("place_order: post-only, 2,000 orders".into(), h.last_cu));
    let ix = h.cancel_ix(&mm, &[(ASK, 1_400_000, seq)]);
    h.send(&[ix], &[&mm.kp]).unwrap();
    rows.push(("cancel_order (newest order)".into(), h.last_cu));

    h.take(&taker, ioc(BID, Some(1_000_000), 5, 0, true));
    rows.push(("take: IOC, 1 fill".into(), h.last_cu));
    h.take(&taker, ioc(BID, Some(2_000_000), 50, 0, true));
    rows.push(("take: IOC, sweep 5 orders".into(), h.last_cu));
    h.take(&taker, ioc(BID, Some(2_000_000), 200, 0, true));
    rows.push(("take: IOC, sweep 20 orders".into(), h.last_cu));
    h.place(&taker, limit(ASK, 999_000, 15));
    rows.push((
        "take: limit ask, 2 fills + rest remainder".into(),
        h.last_cu,
    ));

    let swapper = h.user(1_000 * LOT, 1_000_000_000_000);
    h.swap(&swapper, ioc(BID, Some(5_000_000), 5, 0, false));
    rows.push(("swap (no seat): 1 fill".into(), h.last_cu));
    h.swap(&swapper, ioc(ASK, Some(1), 50, 0, false));
    rows.push(("swap (no seat): 5 fills".into(), h.last_cu));

    // Phoenix has no single cancel-and-replace instruction: both go in one transaction.
    let asks: Vec<(u64, u64)> = (0..5).map(|k| (3_000_000 + k, 10)).collect();
    let bids: Vec<(u64, u64)> = (0..5).map(|k| (10_000 + k, 10)).collect();
    let fresh = h.multiple(&mm, &bids, &asks);
    let cancel = h.cancel_ix(&mm, &fresh);
    let place = h.multiple_ix(&mm, &bids, &asks);
    h.send(&[cancel, place], &[&mm.kp]).unwrap();
    rows.push((
        "batch_update: cancel 10 + place 10 (two instructions)".into(),
        h.last_cu,
    ));

    h.withdraw(&mm, 1);
    rows.push(("withdraw".into(), h.last_cu));

    println!("\n| Operation (Phoenix) | Compute units |\n| --- | --- |");
    for (k, v) in &rows {
        println!("| {k} | {v} |");
    }
}
