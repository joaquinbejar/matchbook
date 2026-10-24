//! Runs the same compute-unit scenarios as `bench.rs` against Manifest, built from source
//! by `scripts/build-competitors.sh`. Skipped when the competitor binary is missing.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, missing_docs)]

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

const MANIFEST: Address = Address::from_str_const("MNFSTqtC93rEfYHB6hF82sKdZpUDFWkViLByLd1k1Ms");
const LOT: u64 = 1_000_000;
const LIMIT: u8 = 0;
const IOC: u8 = 1;
const POST_ONLY: u8 = 2;

struct Order {
    atoms: u64,
    price: u32,
    bid: bool,
    kind: u8,
}

fn o(bid: bool, price: u64, lots: u64) -> Order {
    Order { atoms: lots * LOT, price: price as u32, bid, kind: LIMIT }
}

struct M {
    svm: LiteSVM,
    payer: Keypair,
    market: Address,
    base_mint: Address,
    quote_mint: Address,
    last_cu: u64,
}

fn vault(market: &Address, mint: &Address) -> Address {
    Address::find_program_address(&[b"vault", market.as_ref(), mint.as_ref()], &MANIFEST).0
}

impl M {
    fn new(path: &str) -> Self {
        let mut svm = LiteSVM::new();
        svm.add_program_from_file(MANIFEST, path).unwrap();
        let payer = Keypair::new();
        svm.airdrop(&payer.pubkey(), 1_000_000_000_000).unwrap();
        let base_mint = CreateMint::new(&mut svm, &payer).decimals(9).send().unwrap();
        let quote_mint = CreateMint::new(&mut svm, &payer).decimals(6).send().unwrap();
        let market_kp = Keypair::new();
        let market = market_kp.pubkey();
        let mut m = Self { svm, payer, market, base_mint, quote_mint, last_cu: 0 };
        // MarketFixed is 256 bytes; the program grows the account from there.
        let space = 256u64;
        let lamports = m.svm.minimum_balance_for_rent_exemption(space as usize);
        let mut create = vec![0u8, 0, 0, 0];
        create.extend_from_slice(&lamports.to_le_bytes());
        create.extend_from_slice(&space.to_le_bytes());
        create.extend_from_slice(MANIFEST.as_ref());
        let ix1 = Instruction::new_with_bytes(SYSTEM, &create, vec![AccountMeta::new(m.payer.pubkey(), true), AccountMeta::new(market, true)]);
        let ix2 = Instruction::new_with_bytes(MANIFEST, &[0], vec![
            AccountMeta::new(m.payer.pubkey(), true),
            AccountMeta::new(market, true),
            AccountMeta::new_readonly(SYSTEM, false),
            AccountMeta::new_readonly(base_mint, false),
            AccountMeta::new_readonly(quote_mint, false),
            AccountMeta::new(vault(&market, &base_mint), false),
            AccountMeta::new(vault(&market, &quote_mint), false),
            AccountMeta::new_readonly(TOKEN, false),
            AccountMeta::new_readonly(TOKEN_2022, false),
        ]);
        m.send(&[ix1, ix2], &[&market_kp]).expect("create market");
        m
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

    fn user(&mut self, base: u64, quote: u64) -> User {
        let kp = Keypair::new();
        self.svm.airdrop(&kp.pubkey(), 10_000_000_000).unwrap();
        let b = self.ata(&kp.pubkey(), self.base_mint, base);
        let q = self.ata(&kp.pubkey(), self.quote_mint, quote);
        User { kp, base: b, quote: q }
    }

    fn ata(&mut self, owner: &Address, mint: Address, amount: u64) -> Address {
        let a = ata(owner, &mint, &TOKEN);
        let create = Instruction::new_with_bytes(ATA, &[1], vec![
            AccountMeta::new(self.payer.pubkey(), true),
            AccountMeta::new(a, false),
            AccountMeta::new_readonly(*owner, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(SYSTEM, false),
            AccountMeta::new_readonly(TOKEN, false),
        ]);
        let mut d = vec![7u8];
        d.extend_from_slice(&amount.to_le_bytes());
        let mint_to = Instruction::new_with_bytes(TOKEN, &d, vec![
            AccountMeta::new(mint, false),
            AccountMeta::new(a, false),
            AccountMeta::new_readonly(self.payer.pubkey(), true),
        ]);
        self.send(&[create, mint_to], &[]).unwrap();
        a
    }

    fn expand(&mut self, blocks: u32) {
        let mut d = vec![5u8];
        d.extend_from_slice(&blocks.to_le_bytes());
        let ix = Instruction::new_with_bytes(MANIFEST, &d, vec![
            AccountMeta::new(self.payer.pubkey(), true),
            AccountMeta::new(self.market, false),
            AccountMeta::new_readonly(SYSTEM, false),
        ]);
        self.send(&[ix], &[]).unwrap();
    }

    fn claim_seat(&mut self, u: &User) {
        let ix = Instruction::new_with_bytes(MANIFEST, &[1], vec![
            AccountMeta::new(u.key(), true),
            AccountMeta::new(self.market, false),
            AccountMeta::new_readonly(SYSTEM, false),
        ]);
        self.send(&[ix], &[&u.kp]).unwrap();
    }

    fn transfer(&mut self, u: &User, base: bool, amount: u64, tag: u8) {
        let (mint, acct) = if base { (self.base_mint, u.base) } else { (self.quote_mint, u.quote) };
        let mut d = vec![tag];
        d.extend_from_slice(&amount.to_le_bytes());
        d.push(0);
        let ix = Instruction::new_with_bytes(MANIFEST, &d, vec![
            AccountMeta::new(u.key(), true),
            AccountMeta::new(self.market, false),
            AccountMeta::new(acct, false),
            AccountMeta::new(vault(&self.market, &mint), false),
            AccountMeta::new_readonly(TOKEN, false),
            AccountMeta::new_readonly(mint, false),
        ]);
        self.send(&[ix], &[&u.kp]).unwrap();
    }

    /// Returns `(sequence_number, index)` for every order the batch placed.
    fn batch(&mut self, u: &User, cancels: &[(u64, Option<u32>)], orders: &[Order]) -> Vec<(u64, u32)> {
        let mut d = vec![6u8, 0];
        d.extend_from_slice(&(cancels.len() as u32).to_le_bytes());
        for (seq, hint) in cancels {
            d.extend_from_slice(&seq.to_le_bytes());
            match hint {
                Some(i) => {
                    d.push(1);
                    d.extend_from_slice(&i.to_le_bytes());
                }
                None => d.push(0),
            }
        }
        d.extend_from_slice(&(orders.len() as u32).to_le_bytes());
        for x in orders {
            d.extend_from_slice(&x.atoms.to_le_bytes());
            d.extend_from_slice(&x.price.to_le_bytes());
            d.push((-6i8) as u8);
            d.push(x.bid as u8);
            d.extend_from_slice(&0u32.to_le_bytes());
            d.push(x.kind);
        }
        let ix = Instruction::new_with_bytes(MANIFEST, &d, vec![
            AccountMeta::new(u.key(), true),
            AccountMeta::new(self.market, false),
            AccountMeta::new_readonly(SYSTEM, false),
        ]);
        let ret = self.send(&[ix], &[&u.kp]).unwrap();
        ret.get(4..).unwrap_or(&[]).chunks_exact(12)
            .map(|c| (u64::from_le_bytes(c[..8].try_into().unwrap()), u32::from_le_bytes(c[8..].try_into().unwrap())))
            .collect()
    }

    fn swap(&mut self, u: &User, in_atoms: u64, base_in: bool) {
        let mut d = vec![4u8];
        d.extend_from_slice(&in_atoms.to_le_bytes());
        d.extend_from_slice(&0u64.to_le_bytes());
        d.push(base_in as u8);
        d.push(1);
        let ix = Instruction::new_with_bytes(MANIFEST, &d, vec![
            AccountMeta::new(u.key(), true),
            AccountMeta::new(self.market, false),
            AccountMeta::new_readonly(SYSTEM, false),
            AccountMeta::new(u.base, false),
            AccountMeta::new(u.quote, false),
            AccountMeta::new(vault(&self.market, &self.base_mint), false),
            AccountMeta::new(vault(&self.market, &self.quote_mint), false),
            AccountMeta::new_readonly(TOKEN, false),
        ]);
        self.send(&[ix], &[&u.kp]).unwrap();
    }
}

#[test]
fn manifest_compute_units() {
    let path = format!("{}/../target/competitors/manifest.so", env!("CARGO_MANIFEST_DIR"));
    if !std::path::Path::new(&path).exists() {
        eprintln!("skipping: {path} not found, run scripts/build-competitors.sh");
        return;
    }
    let mut rows: Vec<(String, u64)> = vec![];
    let mut h = M::new(&path);
    // Preallocate 4,000 blocks so no measured instruction pays for growth.
    for n in (100..=4_100).step_by(100) {
        h.expand(n);
    }
    let mm = h.user(100_000 * LOT, 100_000_000_000_000);
    let taker = h.user(100_000 * LOT, 100_000_000_000_000);
    h.claim_seat(&mm);
    rows.push(("claim_seat".into(), h.last_cu));
    h.claim_seat(&taker);
    h.transfer(&mm, true, 100_000 * LOT, 2);
    rows.push(("deposit".into(), h.last_cu));
    h.transfer(&mm, false, 100_000_000_000_000, 2);
    h.transfer(&taker, true, 100_000 * LOT, 2);
    h.transfer(&taker, false, 100_000_000_000_000, 2);

    h.batch(&mm, &[], &[o(false, 1_000_000, 10)]);
    rows.push(("place_order: rest, empty book".into(), h.last_cu));
    for i in 0..125u64 {
        let asks: Vec<Order> = (0..8).map(|k| o(false, 1_000_100 + i * 8 + k, 10)).collect();
        h.batch(&mm, &[], &asks);
        let bids: Vec<Order> = (0..8).map(|k| o(true, 999_900 - i * 8 - k, 10)).collect();
        h.batch(&mm, &[], &bids);
    }
    h.batch(&mm, &[], &[o(false, 1_500_000, 10)]);
    rows.push(("place_order: rest, book with 2,000 orders".into(), h.last_cu));
    let placed = h.batch(&mm, &[], &[Order { kind: POST_ONLY, ..o(false, 1_400_000, 10) }]);
    rows.push(("place_order: post-only, 2,000 orders".into(), h.last_cu));
    let (seq, idx) = placed[0];
    h.batch(&mm, &[(seq, Some(idx))], &[]);
    rows.push(("cancel_order (newest order, with index hint)".into(), h.last_cu));
    let placed = h.batch(&mm, &[], &[o(false, 1_400_000, 10)]);
    h.batch(&mm, &[(placed[0].0, None)], &[]);
    rows.push(("cancel_order (newest order, no hint: scans the book)".into(), h.last_cu));

    h.batch(&taker, &[], &[Order { kind: IOC, ..o(true, 1_000_000, 5) }]);
    rows.push(("take: IOC, 1 fill".into(), h.last_cu));
    h.batch(&taker, &[], &[Order { kind: IOC, ..o(true, 2_000_000, 50) }]);
    rows.push(("take: IOC, sweep 5 orders".into(), h.last_cu));
    h.batch(&taker, &[], &[Order { kind: IOC, ..o(true, 2_000_000, 200) }]);
    rows.push(("take: IOC, sweep 20 orders".into(), h.last_cu));
    h.batch(&taker, &[], &[o(false, 999_000, 15)]);
    rows.push(("take: limit ask, 2 fills + rest remainder".into(), h.last_cu));

    let swapper = h.user(1_000 * LOT, 1_000_000_000_000);
    // Quote in for roughly 5 lots at the touch: one fill.
    h.swap(&swapper, 5 * LOT * 1_002_000 / 1_000_000, false);
    rows.push(("swap (no seat): 1 fill".into(), h.last_cu));
    h.swap(&swapper, 50 * LOT, true);
    rows.push(("swap (no seat): 5 fills".into(), h.last_cu));

    let quotes: Vec<Order> = (0..5).map(|k| o(false, 3_000_000 + k, 10)).chain((0..5).map(|k| o(true, 10_000 + k, 10))).collect();
    let fresh = h.batch(&mm, &[], &quotes);
    let cancels: Vec<(u64, Option<u32>)> = fresh.iter().map(|&(s, i)| (s, Some(i))).collect();
    h.batch(&mm, &cancels, &quotes);
    rows.push(("batch_update: cancel 10 + place 10 (with hints)".into(), h.last_cu));

    h.transfer(&mm, true, LOT, 3);
    rows.push(("withdraw".into(), h.last_cu));

    println!("\n| Operation (Manifest) | Compute units |\n| --- | --- |");
    for (k, v) in &rows {
        println!("| {k} | {v} |");
    }
}
