#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, missing_docs)]
mod common;
use common::*;
use matchbook_program::instruction::OrderArgs;
const LOT: u64 = 1_000_000;
#[test]
fn trace() {
    let mut h = H::new(CreateMarketArgsLite { initial_nodes: 4_000, ..default_market() }, false);
    let mm = h.user(100_000 * LOT, 100_000_000_000_000);
    let taker = h.user(100_000 * LOT, 100_000_000_000_000);
    h.claim_seat(&mm).unwrap();
    h.claim_seat(&taker).unwrap();
    h.deposit(&mm, true, 100_000 * LOT).unwrap();
    h.deposit(&taker, false, 100_000_000_000_000).unwrap();
    for i in 0..125u64 {
        let asks: Vec<OrderArgs> = (0..8).map(|k| limit(1, 1_000_100 + i * 8 + k, 10)).collect();
        h.batch(&mm, 0, &[], &asks).unwrap();
    }
    let r = h.place(&taker, OrderArgs { order_type: 3, ..limit(0, 2_000_000, 30) }).unwrap();
    for l in &r.logs { println!("{l}"); }
}
