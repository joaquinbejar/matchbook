//! Compute-unit benchmarks. Run with `cargo test --test bench -- --nocapture`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    missing_docs
)]

mod common;

use common::*;
use matchbook_program::instruction::{build, OrderArgs};

const LOT: u64 = 1_000_000;

fn ids(h: &H, u: &User) -> Vec<u64> {
    let s = h.seat(u).unwrap();
    let mut v = vec![];
    h.with_market(|m| {
        m.for_each_trader_order(s, |_, o| v.push(o.order_id))
            .unwrap()
    });
    v
}

#[test]
fn compute_units() {
    let mut rows: Vec<(String, u64)> = vec![];
    let mut h = H::new(
        CreateMarketArgsLite {
            initial_nodes: 4_000,
            ..default_market()
        },
        false,
    );
    rows.push(("create_market (4,000 nodes preallocated)".into(), h.last_cu));
    let mm = h.user(100_000 * LOT, 100_000_000_000_000);
    let taker = h.user(100_000 * LOT, 100_000_000_000_000);
    h.claim_seat(&mm).unwrap();
    rows.push(("claim_seat".into(), h.last_cu));
    h.claim_seat(&taker).unwrap();
    h.deposit(&mm, true, 100_000 * LOT).unwrap();
    rows.push(("deposit".into(), h.last_cu));
    h.deposit(&mm, false, 100_000_000_000_000).unwrap();
    h.deposit(&taker, true, 100_000 * LOT).unwrap();
    h.deposit(&taker, false, 100_000_000_000_000).unwrap();

    h.place(&mm, limit(1, 1_000_000, 10)).unwrap();
    rows.push(("place_order: rest, empty book".into(), h.last_cu));
    // Deep book: 1,000 asks and 1,000 bids.
    for i in 0..125u64 {
        let asks: Vec<OrderArgs> = (0..8)
            .map(|k| limit(1, 1_000_100 + i * 8 + k, 10))
            .collect();
        h.batch(&mm, 0, &[], &asks).unwrap();
        let bids: Vec<OrderArgs> = (0..8).map(|k| limit(0, 999_900 - i * 8 - k, 10)).collect();
        h.batch(&mm, 0, &[], &bids).unwrap();
    }
    h.place(&mm, limit(1, 1_500_000, 10)).unwrap();
    rows.push((
        "place_order: rest, book with 2,000 orders".into(),
        h.last_cu,
    ));
    h.place(
        &mm,
        OrderArgs {
            order_type: 1,
            ..limit(1, 1_400_000, 10)
        },
    )
    .unwrap();
    rows.push(("place_order: post-only, 2,000 orders".into(), h.last_cu));
    let last = *ids(&h, &mm).first().unwrap();
    let ix = h.cancel_ix(&mm, build::cancel_order(h.seat(&mm).unwrap(), last));
    h.send(&[ix], &[&mm.kp]).unwrap();
    rows.push(("cancel_order (newest order)".into(), h.last_cu));

    h.place(
        &taker,
        OrderArgs {
            order_type: 3,
            ..limit(0, 1_000_000, 5)
        },
    )
    .unwrap();
    rows.push(("take: IOC, 1 fill".into(), h.last_cu));
    h.place(
        &taker,
        OrderArgs {
            order_type: 3,
            ..limit(0, 2_000_000, 50)
        },
    )
    .unwrap();
    rows.push(("take: IOC, sweep 5 orders".into(), h.last_cu));
    h.place(
        &taker,
        OrderArgs {
            order_type: 3,
            ..limit(0, 2_000_000, 200)
        },
    )
    .unwrap();
    rows.push(("take: IOC, sweep 20 orders".into(), h.last_cu));
    h.place(&taker, limit(1, 999_000, 15)).unwrap();
    rows.push((
        "take: limit ask, 2 fills + rest remainder".into(),
        h.last_cu,
    ));

    let swapper = h.user(1_000 * LOT, 1_000_000_000_000);
    h.swap(
        &swapper,
        Swap {
            side: 0,
            base_lots: 5,
            quote_budget: u64::MAX / 2,
            limit_price: 5_000_000,
            min_out: 0,
            match_limit: 0,
        },
    )
    .unwrap();
    rows.push(("swap (no seat): 1 fill".into(), h.last_cu));
    h.swap(
        &swapper,
        Swap {
            side: 1,
            base_lots: 50,
            quote_budget: 0,
            limit_price: 1,
            min_out: 0,
            match_limit: 0,
        },
    )
    .unwrap();
    rows.push(("swap (no seat): 5 fills".into(), h.last_cu));

    // Market-maker requote: cancel 10 fresh quotes and place 10 new ones.
    let quotes: Vec<OrderArgs> = (0..5)
        .map(|k| limit(1, 3_000_000 + k, 10))
        .chain((0..5).map(|k| limit(0, 10_000 + k, 10)))
        .collect();
    let r = h.batch(&mm, 0, &[], &quotes).unwrap();
    let fresh: Vec<u64> = r
        .return_data
        .data
        .chunks_exact(8)
        .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
        .collect();
    h.batch(&mm, 0, &fresh, &quotes).unwrap();
    rows.push(("batch_update: cancel 10 + place 10".into(), h.last_cu));

    h.withdraw(&mm, true, LOT).unwrap();
    rows.push(("withdraw".into(), h.last_cu));
    h.audit();

    let mut g = H::new(default_market(), false);
    let u = g.user(10 * LOT, 0);
    g.claim_seat(&u).unwrap();
    g.deposit(&u, true, 10 * LOT).unwrap();
    g.place(&u, limit(1, 1_000_000, 1)).unwrap();
    rows.push((
        "place_order: rest, grows account by one node".into(),
        g.last_cu,
    ));

    println!("\n| Operation | Compute units |\n| --- | --- |");
    for (k, v) in &rows {
        println!("| {k} | {v} |");
    }
}
