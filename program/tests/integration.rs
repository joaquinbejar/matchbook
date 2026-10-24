//! End-to-end tests against the compiled program in LiteSVM.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    missing_docs
)]

mod common;

use common::*;
use matchbook_program::engine::order_id;
use matchbook_program::instruction::{build, OrderArgs};
use solana_signer::Signer;

const LOT: u64 = 1_000_000;

#[test]
fn lifecycle_place_cross_withdraw() {
    let mut h = H::new(default_market(), false);
    let maker = h.user(100 * LOT, 0);
    let taker = h.user(0, 1_000_000_000);
    h.claim_seat(&maker).unwrap();
    h.claim_seat(&taker).unwrap();
    h.deposit(&maker, true, 100 * LOT).unwrap();
    h.deposit(&taker, false, 1_000_000_000).unwrap();
    h.audit();

    // Ask 50 lots at 150_000 quote atoms per lot (150 USDC per SOL).
    h.place(&maker, limit(1, 150_000, 50)).unwrap();
    // Bid crosses 20 lots and fills immediately: no crank.
    h.place(&taker, limit(0, 151_000, 20)).unwrap();
    let t = h.trader(&taker);
    assert_eq!(t.base_free, 20 * LOT);
    let notional = 20 * 150_000;
    let fee = (notional * 10u64).div_ceil(10_000);
    assert_eq!(t.quote_free, 1_000_000_000 - notional - fee);
    let m = h.trader(&maker);
    assert_eq!(m.quote_free, notional + notional * 2 / 10_000);
    h.audit();

    h.withdraw(&taker, true, 20 * LOT).unwrap();
    h.withdraw(&maker, false, notional).unwrap();
    assert_eq!(h.token(&taker.base), 20 * LOT);
    h.audit();
    // Over-withdraw fails.
    assert!(h.withdraw(&taker, true, 1).is_err());
}

#[test]
fn swap_fees_and_authority() {
    let mut h = H::new(default_market(), false);
    let maker = h.user(100 * LOT, 100_000_000);
    let swapper = h.user(10 * LOT, 100_000_000);
    h.claim_seat(&maker).unwrap();
    h.deposit(&maker, true, 100 * LOT).unwrap();
    h.deposit(&maker, false, 100_000_000).unwrap();
    h.place(&maker, limit(1, 200_000, 10)).unwrap();
    h.place(&maker, limit(0, 190_000, 10)).unwrap();

    // Buy up to 5 lots, too-strict min_out fails atomically.
    let buy = Swap {
        side: 0,
        base_lots: 5,
        quote_budget: 10_000_000,
        limit_price: 200_000,
        min_out: 6 * LOT,
        match_limit: 0,
    };
    assert_eq!(err_code(&h.swap(&swapper, buy)), Some(107));
    h.swap(
        &swapper,
        Swap {
            min_out: 5 * LOT,
            ..buy
        },
    )
    .unwrap();
    assert_eq!(h.token(&swapper.base), 15 * LOT);
    let paid = 5 * 200_000 + (5u64 * 200_000 * 10).div_ceil(10_000);
    assert_eq!(h.token(&swapper.quote), 100_000_000 - paid);
    // Sell 3 lots into the bid.
    h.swap(
        &swapper,
        Swap {
            side: 1,
            base_lots: 3,
            quote_budget: 0,
            limit_price: 1,
            min_out: 0,
            match_limit: 0,
        },
    )
    .unwrap();
    h.audit();

    let fees = h.with_market(|m| m.header().fees_accrued);
    assert!(fees > 0);
    let stranger = solana_keypair::Keypair::new();
    h.svm.airdrop(&stranger.pubkey(), 1_000_000_000).unwrap();
    let dest = swapper.quote;
    assert_eq!(err_code(&h.collect_fees(&stranger, dest)), Some(109));
    let auth = h.authority.insecure_clone();
    let before = h.token(&dest);
    h.collect_fees(&auth, dest).unwrap();
    assert_eq!(h.token(&dest), before + fees);
    h.audit();
}

#[test]
fn token_2022_quote_mint() {
    let mut h = H::new(default_market(), true);
    let a = h.user(10 * LOT, 0);
    let b = h.user(0, 10_000_000);
    h.claim_seat(&a).unwrap();
    h.claim_seat(&b).unwrap();
    h.deposit(&a, true, 10 * LOT).unwrap();
    h.deposit(&b, false, 10_000_000).unwrap();
    h.place(&a, limit(1, 100_000, 10)).unwrap();
    h.place(&b, limit(0, 100_000, 10)).unwrap();
    h.withdraw(&a, false, 1_000_000).unwrap();
    assert_eq!(h.token(&a.quote), 1_000_000);
    h.audit();
}

#[test]
fn market_grows_from_zero_nodes() {
    let mut h = H::new(default_market(), false);
    let len0 = h.data().len();
    let a = h.user(1_000 * LOT, 0);
    h.claim_seat(&a).unwrap();
    h.deposit(&a, true, 1_000 * LOT).unwrap();
    for i in 0..40 {
        h.place(&a, limit(1, 100_000 + i, 1)).unwrap();
    }
    let len1 = h.data().len();
    assert_eq!(len1 - len0, 41 * matchbook_program::engine::NODE_SIZE);
    h.audit();
}

#[test]
fn others_cannot_touch_my_orders_or_funds() {
    let mut h = H::new(default_market(), false);
    let a = h.user(10 * LOT, 0);
    let mallory = h.user(0, 0);
    h.claim_seat(&a).unwrap();
    h.claim_seat(&mallory).unwrap();
    h.deposit(&a, true, 10 * LOT).unwrap();
    h.place(&a, limit(1, 100_000, 5)).unwrap();
    let id = h.with_market(|m| {
        let i = m.best_ask();
        order_id(m.order(i).unwrap().seq, i)
    });
    let ix = h.cancel_ix(&mallory, build::cancel_order(u32::MAX, id));
    assert_eq!(err_code(&h.send(&[ix], &[&mallory.kp])), Some(13)); // OrderNotFound
                                                                    // Mallory cannot withdraw a's base: her own seat is empty.
    assert!(h.withdraw(&mallory, true, 1).is_err());
    // Seat hint pointing at a's seat is ignored for mallory.
    let hint = h.seat(&a).unwrap();
    let ix = h.cancel_ix(&mallory, build::cancel_order(hint, id));
    assert!(h.send(&[ix], &[&mallory.kp]).is_err());
    let ix = h.cancel_ix(&a, build::cancel_order(hint, id));
    h.send(&[ix], &[&a.kp]).unwrap();
    h.audit();
}

#[test]
fn expiry_uses_cluster_clock() {
    let mut h = H::new(default_market(), false);
    let a = h.user(10 * LOT, 0);
    let b = h.user(0, 10_000_000);
    h.claim_seat(&a).unwrap();
    h.claim_seat(&b).unwrap();
    h.deposit(&a, true, 10 * LOT).unwrap();
    h.deposit(&b, false, 10_000_000).unwrap();
    let mut clock: solana_clock_shim::Clock = h.svm.get_sysvar();
    let now = clock.unix_timestamp as u64;
    h.place(
        &a,
        OrderArgs {
            expires_at: now + 60,
            ..limit(1, 100_000, 5)
        },
    )
    .unwrap();
    clock.unix_timestamp += 120;
    h.svm.set_sysvar(&clock);
    h.place(&b, limit(0, 100_000, 5)).unwrap();
    // The expired ask was removed, the bid rests instead of filling.
    assert_eq!(h.trader(&b).base_free, 0);
    assert_eq!(h.trader(&a).base_free, 10 * LOT);
    h.audit();
}

mod solana_clock_shim {
    pub use solana_clock::Clock;
}
