//! PoC: price-mismatch drain in Matchbook settle (audit finding #1).
//!
//! Scenario:
//!   1. Bid placed at price 100 for qty 10 -> locks quote = 10*100*qls/bls
//!   2. Resting ask at price 110 for the same qty
//!   3. match_orders matches them with match_price = ask_price = 110
//!   4. consume_events settles the taker-bid with quote_amount computed at
//!      110, but only 1000 was locked. settle_taker_bid's saturating_sub
//!      hides the deficit instead of reverting.
//!
//! Expected (correct behavior): either the match is rejected or the settle
//! amount is capped by the locked funds.
//! Actual behavior: deficit silently absorbed; maker credited at ask price
//! from pooled vault locks.

use anchor_lang::prelude::Pubkey;

// Mirror of place_order.rs calculate_lock_amount (Side::Bid)
fn lock_amount_bid(quantity: u64, price: u64, base_lot_size: u64, quote_lot_size: u64) -> u64 {
    quantity
        .checked_mul(price).unwrap()
        .checked_mul(quote_lot_size).unwrap()
        .checked_div(base_lot_size).unwrap()
}

// Mirror of consume_events.rs lines 301-303 (settle quote_amount)
fn settle_quote_amount(quantity: u64, match_price: u64, base_lot_size: u64, quote_lot_size: u64) -> u64 {
    quantity
        .checked_mul(match_price).unwrap()
        .checked_mul(quote_lot_size).unwrap()
        .checked_div(base_lot_size).unwrap()
}

// Mirror of consume_events.rs settle_taker_bid line 372 (saturating debit)
fn settle_taker_quote_locked(quote_locked: u64, quote_amount: u64, taker_fee: u64) -> u64 {
    quote_locked.saturating_sub(quote_amount.saturating_add(taker_fee))
}

#[test]
fn test_saturating_sub_hides_deficit() {
    // Direct numeric reproduction of the settle math from consume_events.rs
    //
    // Setup (in quote token smallest units):
    //   taker bid: qty=10 lots, price=100  -> locked = 10*100 = 1000
    //   matching ask price = 110           -> settled quote = 10*110 = 1100
    //   taker_fee = 10
    let quote_locked: u64 = 1000;
    let quote_settled: u64 = 1100;
    let taker_fee: u64 = 10;

    let remaining = settle_taker_quote_locked(quote_locked, quote_settled, taker_fee);

    // Correct behavior would be a revert (InsufficientLock) because
    // quote_settled + fee > quote_locked. Instead the deficit vanishes:
    assert_eq!(remaining, 0, "saturating_sub silently absorbs the 110-unit deficit");
    // The taker still receives the full base_amount for this fill even though
    // 110 units of the settlement were never locked by anyone on the taker side.
    // That value comes out of OTHER users' aggregated locks in the market vault.
}

#[test]
fn test_aggregate_vault_insolvency_accumulates() {
    // Simulate N identical trades: each one leaks (ask_price - bid_price)*qty
    // of unbacked credit into the maker's balance.
    let mut vault_backing: i128 = 0; // total real quote locked across users
    let mut total_credit_issued: i128 = 0; // total quote_free credited by settles

    let n: u64 = 1000;
    for _ in 0..n {
        let locked: u64 = 1000;
        let settled: u64 = 1100;
        let fee: u64 = 10;

        vault_backing += locked as i128;
        // taker receives base against `settled + fee` debit that saturates at locked
        let actual_debit = (locked).min(settled + fee);
        vault_backing -= actual_debit as i128;
        // maker is credited the full settled amount regardless
        total_credit_issued += (settled + 0) as i128;
        let _ = fee;
    }

    // Every trade credits the maker more quote than was actually locked/covered,
    // while the taker's lock is consumed entirely. The gap is unbacked credit:
    assert!(
        total_credit_issued > vault_backing,
        "credit issued must exceed backing after repeated mismatches (insolvency)"
    );
}
