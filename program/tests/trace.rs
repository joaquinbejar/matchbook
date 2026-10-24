//! Prints a compute-unit trace of one batch update. Build the program with
//! `cargo build-sbf --features cu-trace` first.
#![allow(
    clippy::chunks_exact_to_as_chunks,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    missing_docs
)]
mod common;
use common::*;
use matchbook_program::instruction::OrderArgs;
const LOT: u64 = 1_000_000;

#[test]
#[ignore = "profiling aid"]
fn trace_batch() {
    let mut h = H::new(
        CreateMarketArgsLite {
            initial_nodes: 4_000,
            ..default_market()
        },
        false,
    );
    let mm = h.user(100_000 * LOT, 100_000_000_000_000);
    h.claim_seat(&mm).unwrap();
    h.deposit(&mm, true, 100_000 * LOT).unwrap();
    h.deposit(&mm, false, 100_000_000_000_000).unwrap();
    for i in 0..125u64 {
        let asks: Vec<OrderArgs> = (0..8)
            .map(|k| limit(1, 1_000_100 + i * 8 + k, 10))
            .collect();
        h.batch(&mm, 0, &[], &asks).unwrap();
        let bids: Vec<OrderArgs> = (0..8).map(|k| limit(0, 999_900 - i * 8 - k, 10)).collect();
        h.batch(&mm, 0, &[], &bids).unwrap();
    }
    let quotes: Vec<OrderArgs> = (0..3)
        .map(|k| limit(1, 3_000_000 + k, 10))
        .chain((0..3).map(|k| limit(0, 10_000 + k, 10)))
        .collect();
    let r = h.batch(&mm, 0, &[], &quotes).unwrap();
    let ids: Vec<u64> = r
        .return_data
        .data
        .chunks_exact(8)
        .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
        .collect();
    let r = h.batch(&mm, 0, &ids, &quotes).unwrap();
    let mut prev = 0u64;
    for l in &r.logs {
        if let Some(v) = l
            .strip_prefix("Program consumption: ")
            .and_then(|x| x.split(' ').next())
            .and_then(|x| x.parse::<u64>().ok())
        {
            if prev != 0 {
                println!("{:>6}", prev - v - 100);
            }
            prev = v;
        } else {
            println!("{l}");
        }
    }
}

#[test]
#[ignore = "profiling aid"]
fn trace_sweep() {
    let mut h = H::new(
        CreateMarketArgsLite {
            initial_nodes: 4_000,
            ..default_market()
        },
        false,
    );
    let mm = h.user(100_000 * LOT, 100_000_000_000_000);
    let taker = h.user(100_000 * LOT, 100_000_000_000_000);
    h.claim_seat(&mm).unwrap();
    h.claim_seat(&taker).unwrap();
    h.deposit(&mm, true, 100_000 * LOT).unwrap();
    h.deposit(&taker, false, 100_000_000_000_000).unwrap();
    for i in 0..125u64 {
        let asks: Vec<OrderArgs> = (0..8)
            .map(|k| limit(1, 1_000_100 + i * 8 + k, 10))
            .collect();
        h.batch(&mm, 0, &[], &asks).unwrap();
    }
    let r = h
        .place(
            &taker,
            OrderArgs {
                order_type: 3,
                ..limit(0, 2_000_000, 40)
            },
        )
        .unwrap();
    let mut prev = 0u64;
    for l in &r.logs {
        if let Some(v) = l
            .strip_prefix("Program consumption: ")
            .and_then(|x| x.split(' ').next())
            .and_then(|x| x.parse::<u64>().ok())
        {
            if prev != 0 {
                print!("{:>5}", prev - v - 100);
            }
            prev = v;
        } else if !l.starts_with("Program data") {
            println!("\n{l}");
        }
    }
    println!();
}
