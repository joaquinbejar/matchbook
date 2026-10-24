//! Runs the same compute-unit scenarios as `bench.rs` against OpenBook v2, built from source
//! by `scripts/build-competitors.sh`. Skipped when the competitor binary is missing.
//!
//! OpenBook differs in ways that shape the scenarios:
//! - An open-orders account holds at most 24 orders, so the deep book is spread over 40
//!   accounts, and each book side holds at most ~500 orders: the deep book has 960, not 2,000.
//! - Makers are paid later: a fill writes an event that `consume_events` (the crank) must
//!   process, so the crank's cost is measured as its own row.
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

const OPENBOOK: Address = Address::from_str_const("opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb");
const BOOKSIDE_LEN: u64 = 8 + 90_944;
const EVENT_HEAP_LEN: u64 = 8 + 91_280;
const LOT: u64 = 1_000_000;

const BID: u8 = 0;
const ASK: u8 = 1;
const LIMIT: u8 = 0;
const IOC: u8 = 1;
const POST_ONLY: u8 = 2;

/// Anchor discriminators: `sha256("global:<name>")[..8]`.
mod disc {
    pub const CREATE_MARKET: [u8; 8] = [103, 226, 97, 235, 200, 188, 251, 254];
    pub const CREATE_OPEN_ORDERS_INDEXER: [u8; 8] = [64, 64, 153, 255, 217, 71, 249, 133];
    pub const CREATE_OPEN_ORDERS_ACCOUNT: [u8; 8] = [204, 181, 175, 222, 40, 125, 188, 71];
    pub const DEPOSIT: [u8; 8] = [242, 35, 198, 137, 82, 225, 242, 182];
    pub const PLACE_ORDER: [u8; 8] = [51, 194, 155, 175, 109, 130, 96, 106];
    pub const PLACE_ORDERS: [u8; 8] = [60, 63, 50, 123, 12, 197, 60, 190];
    pub const CANCEL_ALL_AND_PLACE_ORDERS: [u8; 8] = [128, 155, 222, 60, 186, 40, 225, 50];
    pub const PLACE_TAKE_ORDER: [u8; 8] = [3, 44, 71, 3, 26, 199, 203, 85];
    pub const CONSUME_EVENTS: [u8; 8] = [221, 145, 177, 52, 31, 47, 63, 201];
    pub const CANCEL_ORDER: [u8; 8] = [95, 129, 237, 240, 8, 49, 223, 132];
    pub const SETTLE_FUNDS: [u8; 8] = [238, 64, 163, 96, 75, 171, 16, 33];
}

struct W(Vec<u8>);

impl W {
    fn new(d: [u8; 8]) -> Self {
        Self(d.to_vec())
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
    fn i64(self, v: i64) -> Self {
        self.u64(v as u64)
    }
    fn str(mut self, s: &str) -> Self {
        self = self.u32(s.len() as u32);
        self.0.extend_from_slice(s.as_bytes());
        self
    }
}

fn ro(a: Address) -> AccountMeta {
    AccountMeta::new_readonly(a, false)
}

fn rw(a: Address) -> AccountMeta {
    AccountMeta::new(a, false)
}

/// An absent optional Anchor account is passed as the program id.
fn none() -> AccountMeta {
    ro(OPENBOOK)
}

/// Setup-only: raises the compute limit for instructions too large for the default.
fn cu_limit(units: u32) -> Instruction {
    let mut d = vec![2u8];
    d.extend_from_slice(&units.to_le_bytes());
    Instruction::new_with_bytes(
        Address::from_str_const("ComputeBudget111111111111111111111111111111"),
        &d,
        vec![],
    )
}

struct Trader {
    kp: Keypair,
    base: Address,
    quote: Address,
    indexer: Address,
    accounts: Vec<Address>,
}

impl Trader {
    fn key(&self) -> Address {
        self.kp.pubkey()
    }
}

struct O {
    svm: LiteSVM,
    payer: Keypair,
    market: Address,
    authority: Address,
    bids: Address,
    asks: Address,
    heap: Address,
    base_vault: Address,
    quote_vault: Address,
    base_mint: Address,
    quote_mint: Address,
    last_cu: u64,
}

impl O {
    fn new(path: &str) -> Self {
        let mut svm = LiteSVM::new();
        svm.add_program_from_file(OPENBOOK, path).unwrap();
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
        let (market_kp, bids_kp, asks_kp, heap_kp) = (
            Keypair::new(),
            Keypair::new(),
            Keypair::new(),
            Keypair::new(),
        );
        let market = market_kp.pubkey();
        let authority = Address::find_program_address(&[b"Market", market.as_ref()], &OPENBOOK).0;
        let mut o = Self {
            svm,
            payer,
            market,
            authority,
            bids: bids_kp.pubkey(),
            asks: asks_kp.pubkey(),
            heap: heap_kp.pubkey(),
            base_vault: ata(&authority, &base_mint, &TOKEN),
            quote_vault: ata(&authority, &quote_mint, &TOKEN),
            base_mint,
            quote_mint,
            last_cu: 0,
        };
        let mut ixs: Vec<Instruction> = [
            (o.bids, BOOKSIDE_LEN),
            (o.asks, BOOKSIDE_LEN),
            (o.heap, EVENT_HEAP_LEN),
        ]
        .iter()
        .map(|&(a, len)| o.create_account_ix(a, len))
        .collect();
        // Base lot = 1e6 atoms, quote lot = 1 atom, so prices read the same as in the other
        // benches. Fees: 5 bps taker, no maker rebate.
        let d = W::new(disc::CREATE_MARKET)
            .str("BENCH")
            .u32(0f32.to_bits())
            .u8(0)
            .i64(1)
            .i64(LOT as i64)
            .i64(0)
            .i64(500)
            .i64(0)
            .0;
        let event_authority = Address::find_program_address(&[b"__event_authority"], &OPENBOOK).0;
        ixs.push(Instruction::new_with_bytes(
            OPENBOOK,
            &d,
            vec![
                AccountMeta::new(market, true),
                ro(authority),
                rw(o.bids),
                rw(o.asks),
                rw(o.heap),
                AccountMeta::new(o.payer.pubkey(), true),
                rw(o.base_vault),
                rw(o.quote_vault),
                ro(base_mint),
                ro(quote_mint),
                ro(SYSTEM),
                ro(TOKEN),
                ro(ATA),
                none(),
                none(),
                ro(o.payer.pubkey()),
                none(),
                none(),
                none(),
                ro(event_authority),
                ro(OPENBOOK),
            ],
        ));
        o.send(&ixs, &[&market_kp, &bids_kp, &asks_kp, &heap_kp])
            .expect("create market");
        o
    }

    fn create_account_ix(&self, a: Address, space: u64) -> Instruction {
        let lamports = self.svm.minimum_balance_for_rent_exemption(space as usize);
        let mut d = vec![0u8, 0, 0, 0];
        d.extend_from_slice(&lamports.to_le_bytes());
        d.extend_from_slice(&space.to_le_bytes());
        d.extend_from_slice(OPENBOOK.as_ref());
        Instruction::new_with_bytes(
            SYSTEM,
            &d,
            vec![
                AccountMeta::new(self.payer.pubkey(), true),
                AccountMeta::new(a, true),
            ],
        )
    }

    fn send(&mut self, ixs: &[Instruction], extra: &[&Keypair]) -> Result<Vec<u8>, String> {
        self.svm.expire_blockhash();
        let mut signers: Vec<&Keypair> = vec![&self.payer];
        signers.extend_from_slice(extra);
        let msg = Message::new(ixs, Some(&self.payer.pubkey()));
        let tx = Transaction::new(&signers, msg, self.svm.latest_blockhash());
        match self.svm.send_transaction(tx) {
            Ok(meta) => {
                self.last_cu = meta.compute_units_consumed;
                Ok(meta.return_data.data)
            }
            Err(e) => Err(format!("{:?}\n{}", e.err, e.meta.logs.join("\n"))),
        }
    }

    fn ata(&mut self, owner: &Address, mint: Address, amount: u64) -> Address {
        let a = ata(owner, &mint, &TOKEN);
        let create = Instruction::new_with_bytes(
            ATA,
            &[1],
            vec![
                AccountMeta::new(self.payer.pubkey(), true),
                rw(a),
                ro(*owner),
                ro(mint),
                ro(SYSTEM),
                ro(TOKEN),
            ],
        );
        let mut d = vec![7u8];
        d.extend_from_slice(&amount.to_le_bytes());
        let mint_to = Instruction::new_with_bytes(
            TOKEN,
            &d,
            vec![
                rw(mint),
                rw(a),
                AccountMeta::new_readonly(self.payer.pubkey(), true),
            ],
        );
        self.send(&[create, mint_to], &[]).unwrap();
        a
    }

    fn trader(&mut self, base: u64, quote: u64) -> Trader {
        let kp = Keypair::new();
        self.svm.airdrop(&kp.pubkey(), 100_000_000_000).unwrap();
        let b = self.ata(&kp.pubkey(), self.base_mint, base);
        let q = self.ata(&kp.pubkey(), self.quote_mint, quote);
        let indexer =
            Address::find_program_address(&[b"OpenOrdersIndexer", kp.pubkey().as_ref()], &OPENBOOK)
                .0;
        Trader {
            kp,
            base: b,
            quote: q,
            indexer,
            accounts: vec![],
        }
    }

    /// Creates the indexer on first use, then one open-orders account. Returns the CU of the
    /// account creation alone.
    fn open_account(&mut self, t: &mut Trader) -> u64 {
        if t.accounts.is_empty() {
            let ix = Instruction::new_with_bytes(
                OPENBOOK,
                &disc::CREATE_OPEN_ORDERS_INDEXER,
                vec![
                    AccountMeta::new(t.key(), true),
                    AccountMeta::new_readonly(t.key(), true),
                    rw(t.indexer),
                    ro(SYSTEM),
                ],
            );
            self.send(&[ix], &[&t.kp]).unwrap();
        }
        let n = t.accounts.len() as u32 + 1;
        let oo = Address::find_program_address(
            &[b"OpenOrders", t.key().as_ref(), &n.to_le_bytes()],
            &OPENBOOK,
        )
        .0;
        let d = W::new(disc::CREATE_OPEN_ORDERS_ACCOUNT).str("bench").0;
        let ix = Instruction::new_with_bytes(
            OPENBOOK,
            &d,
            vec![
                AccountMeta::new(t.key(), true),
                AccountMeta::new_readonly(t.key(), true),
                none(),
                rw(t.indexer),
                rw(oo),
                ro(self.market),
                ro(SYSTEM),
            ],
        );
        self.send(&[ix], &[&t.kp]).unwrap();
        t.accounts.push(oo);
        self.last_cu
    }

    fn deposit(&mut self, t: &Trader, oo: usize, base: u64, quote: u64) {
        let d = W::new(disc::DEPOSIT).u64(base).u64(quote).0;
        let ix = Instruction::new_with_bytes(
            OPENBOOK,
            &d,
            vec![
                AccountMeta::new_readonly(t.key(), true),
                rw(t.base),
                rw(t.quote),
                rw(t.accounts[oo]),
                rw(self.market),
                rw(self.base_vault),
                rw(self.quote_vault),
                ro(TOKEN),
            ],
        );
        self.send(&[ix], &[&t.kp]).unwrap();
    }

    /// Places one order and returns its id when it rests.
    fn place(
        &mut self,
        t: &Trader,
        oo: usize,
        side: u8,
        kind: u8,
        price: u64,
        lots: u64,
    ) -> Option<u128> {
        let d = W::new(disc::PLACE_ORDER)
            .u8(side)
            .i64(price as i64)
            .i64(lots as i64)
            .i64(i64::MAX)
            .u64(0)
            .u8(kind)
            .u64(0)
            .u8(1) // CancelProvide
            .u8(50)
            .0;
        let (user, vault) = if side == BID {
            (t.quote, self.quote_vault)
        } else {
            (t.base, self.base_vault)
        };
        let ix = Instruction::new_with_bytes(
            OPENBOOK,
            &d,
            vec![
                AccountMeta::new_readonly(t.key(), true),
                rw(t.accounts[oo]),
                none(),
                rw(user),
                rw(self.market),
                rw(self.bids),
                rw(self.asks),
                rw(self.heap),
                rw(vault),
                none(),
                none(),
                ro(TOKEN),
            ],
        );
        let ret = self.send(&[ix], &[&t.kp]).unwrap();
        match ret.first() {
            Some(1) => Some(u128::from_le_bytes(ret[1..17].try_into().unwrap())),
            _ => None,
        }
    }

    fn multiple_ix(
        &self,
        t: &Trader,
        oo: usize,
        cancel_all: bool,
        bids: &[u64],
        asks: &[u64],
        lots: u64,
    ) -> Instruction {
        let mut w = W::new(if cancel_all {
            disc::CANCEL_ALL_AND_PLACE_ORDERS
        } else {
            disc::PLACE_ORDERS
        })
        .u8(LIMIT);
        for list in [bids, asks] {
            w = w.u32(list.len() as u32);
            for &price in list {
                // Each order is sized by its quote budget: price * lots, plus the taker fee.
                w = w
                    .i64(price as i64)
                    .i64((price * lots + price * lots / 1_000) as i64)
                    .u64(0);
            }
        }
        let d = w.u8(10).0;
        Instruction::new_with_bytes(
            OPENBOOK,
            &d,
            vec![
                AccountMeta::new_readonly(t.key(), true),
                rw(t.accounts[oo]),
                none(),
                rw(t.quote),
                rw(t.base),
                rw(self.market),
                rw(self.bids),
                rw(self.asks),
                rw(self.heap),
                rw(self.quote_vault),
                rw(self.base_vault),
                none(),
                none(),
                ro(TOKEN),
            ],
        )
    }

    fn cancel(&mut self, t: &Trader, oo: usize, id: u128) {
        let mut d = disc::CANCEL_ORDER.to_vec();
        d.extend_from_slice(&id.to_le_bytes());
        let ix = Instruction::new_with_bytes(
            OPENBOOK,
            &d,
            vec![
                AccountMeta::new_readonly(t.key(), true),
                rw(t.accounts[oo]),
                ro(self.market),
                rw(self.bids),
                rw(self.asks),
            ],
        );
        self.send(&[ix], &[&t.kp]).unwrap();
    }

    fn take(&mut self, t: &Trader, side: u8, price: u64, lots: u64) {
        let d = W::new(disc::PLACE_TAKE_ORDER)
            .u8(side)
            .i64(price as i64)
            .i64(lots as i64)
            .i64(i64::MAX)
            .u8(IOC)
            .u8(50)
            .0;
        let ix = Instruction::new_with_bytes(
            OPENBOOK,
            &d,
            vec![
                AccountMeta::new(t.key(), true),
                AccountMeta::new(t.key(), true),
                rw(self.market),
                ro(self.authority),
                rw(self.bids),
                rw(self.asks),
                rw(self.base_vault),
                rw(self.quote_vault),
                rw(self.heap),
                rw(t.base),
                rw(t.quote),
                none(),
                none(),
                ro(TOKEN),
                ro(SYSTEM),
                none(),
            ],
        );
        self.send(&[ix], &[&t.kp]).unwrap();
    }

    fn consume_events(&mut self, makers: &[Address]) {
        let d = W::new(disc::CONSUME_EVENTS).u64(64).0;
        let mut accts = vec![none(), rw(self.market), rw(self.heap)];
        accts.extend(makers.iter().map(|&a| rw(a)));
        let ix = Instruction::new_with_bytes(OPENBOOK, &d, accts);
        self.send(&[ix], &[]).unwrap();
    }

    fn settle(&mut self, t: &Trader, oo: usize) {
        let ix = Instruction::new_with_bytes(
            OPENBOOK,
            &disc::SETTLE_FUNDS,
            vec![
                AccountMeta::new_readonly(t.key(), true),
                AccountMeta::new(t.key(), true),
                rw(t.accounts[oo]),
                rw(self.market),
                ro(self.authority),
                rw(self.base_vault),
                rw(self.quote_vault),
                rw(t.base),
                rw(t.quote),
                none(),
                ro(TOKEN),
                ro(SYSTEM),
            ],
        );
        self.send(&[ix], &[&t.kp]).unwrap();
    }

    fn heap_len(&self) -> u16 {
        // EventHeap: header { free_head u16, used_head u16, count u16, ... } after the
        // 8-byte discriminator.
        let a = self.svm.get_account(&self.heap).unwrap();
        u16::from_le_bytes(a.data[12..14].try_into().unwrap())
    }
}

#[test]
fn openbook_compute_units() {
    let path = format!(
        "{}/../target/competitors/openbook_v2.so",
        env!("CARGO_MANIFEST_DIR")
    );
    if !std::path::Path::new(&path).exists() {
        eprintln!("skipping: {path} not found, run scripts/build-competitors.sh");
        return;
    }
    const DEEP: usize = 40;
    let mut rows: Vec<(String, u64)> = vec![];
    let mut h = O::new(&path);
    let mut mm = h.trader(100_000 * LOT, 100_000_000_000_000);
    let mut taker = h.trader(100_000 * LOT, 100_000_000_000_000);
    h.open_account(&mut mm);
    let cu = h.open_account(&mut mm);
    rows.push(("claim_seat (open-orders account)".into(), cu));
    // Accounts 0 and 1 hold single orders, 2..2+DEEP the deep book, the next one the requote.
    for _ in 0..DEEP + 1 {
        h.open_account(&mut mm);
    }
    h.open_account(&mut taker);
    h.deposit(&mm, 0, 1_000 * LOT, 0);
    rows.push(("deposit".into(), h.last_cu));
    for i in 0..mm.accounts.len() {
        h.deposit(&mm, i, 1_000 * LOT, 1_000_000_000_000);
    }
    h.deposit(&taker, 0, 10_000 * LOT, 10_000_000_000_000);

    h.place(&mm, 0, ASK, LIMIT, 1_000_000, 10);
    rows.push(("place_order: rest, empty book".into(), h.last_cu));
    for j in 0..DEEP {
        let base = j as u64 * 12;
        let asks: Vec<u64> = (0..12).map(|k| 1_000_100 + base + k).collect();
        let bids: Vec<u64> = (0..12).map(|k| 999_900 - base - k).collect();
        let ix = h.multiple_ix(&mm, 2 + j, false, &bids, &asks, 10);
        h.send(&[cu_limit(1_400_000), ix], &[&mm.kp]).unwrap();
    }
    h.place(&mm, 1, ASK, LIMIT, 1_500_000, 10);
    rows.push(("place_order: rest, book with 961 orders".into(), h.last_cu));
    let id = h.place(&mm, 1, ASK, POST_ONLY, 1_400_000, 10).unwrap();
    rows.push(("place_order: post-only, 962 orders".into(), h.last_cu));
    h.cancel(&mm, 1, id);
    rows.push(("cancel_order (newest order)".into(), h.last_cu));

    h.place(&taker, 0, BID, IOC, 1_000_000, 5);
    rows.push(("take: IOC, 1 fill".into(), h.last_cu));
    h.place(&taker, 0, BID, IOC, 2_000_000, 50);
    rows.push(("take: IOC, sweep 5 orders".into(), h.last_cu));
    h.place(&taker, 0, BID, IOC, 2_000_000, 200);
    rows.push(("take: IOC, sweep 20 orders".into(), h.last_cu));
    let before = h.heap_len();
    let makers: Vec<Address> = mm.accounts[..5].to_vec();
    h.consume_events(&makers);
    let consumed = before - h.heap_len();
    rows.push((
        format!("crank: consume_events ({consumed} fill events, the per-call maximum)"),
        h.last_cu,
    ));
    h.place(&taker, 0, ASK, LIMIT, 999_000, 15);
    rows.push((
        "take: limit ask, 2 fills + rest remainder".into(),
        h.last_cu,
    ));

    let swapper = h.trader(1_000 * LOT, 1_000_000_000_000);
    h.take(&swapper, BID, 5_000_000, 5);
    rows.push(("swap (no seat): 1 fill".into(), h.last_cu));
    h.take(&swapper, ASK, 1, 50);
    rows.push(("swap (no seat): 5 fills".into(), h.last_cu));

    let q = DEEP + 2;
    let asks: Vec<u64> = (0..5).map(|k| 3_000_000 + k).collect();
    let bids: Vec<u64> = (0..5).map(|k| 10_000 + k).collect();
    let ix = h.multiple_ix(&mm, q, false, &bids, &asks, 10);
    h.send(&[ix], &[&mm.kp]).unwrap();
    let ix = h.multiple_ix(&mm, q, true, &bids, &asks, 10);
    h.send(&[ix], &[&mm.kp]).unwrap();
    rows.push((
        "batch_update: cancel 10 + place 10 (cancel all + place)".into(),
        h.last_cu,
    ));

    h.settle(&mm, 0);
    rows.push(("withdraw (settle_funds)".into(), h.last_cu));

    println!("\n| Operation (OpenBook v2) | Compute units |\n| --- | --- |");
    for (k, v) in &rows {
        println!("| {k} | {v} |");
    }
}
