//! Referral fees for frontends and aggregators, and permissioned markets.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    missing_docs
)]

mod common;

use common::*;
use matchbook_program::instruction::{build, OrderArgs};
use solana_instruction::AccountMeta;
use solana_signer::Signer;

const LOT: u64 = 1_000_000;
const PERMISSIONED: u32 = 27;

#[test]
fn referrers_earn_a_share_of_the_taker_fee() {
    // 10 bps taker fee, 2 bps maker rebate, half the taker fee to referrers.
    let mut h = H::new(
        CreateMarketArgsLite {
            referral_share_bps: 5_000,
            ..default_market()
        },
        false,
    );
    let mm = h.user(20 * LOT, 0);
    let taker = h.user(0, 10_000_000_000);
    let app = h.user(0, 0);
    for u in [&mm, &taker, &app] {
        h.claim_seat(u).unwrap();
    }
    h.deposit(&mm, true, 20 * LOT).unwrap();
    h.deposit(&taker, false, 10_000_000_000).unwrap();
    h.place(&mm, limit(1, 100_000, 20)).unwrap();
    let app_seat = h.seat(&app).unwrap();

    // A seated taker referred by the app: 4 lots at 100,000 pay a 400 fee.
    let mut ix = h.ix(
        build::place_order(
            h.seat(&taker).unwrap(),
            &OrderArgs {
                order_type: 3,
                referrer_seat: app_seat,
                ..limit(0, 100_000, 4)
            },
        ),
        h.owner_metas(&taker),
    );
    ix.accounts
        .push(AccountMeta::new_readonly(app.key(), false));
    h.send(&[ix], &[&taker.kp]).unwrap();
    assert_eq!(h.trader(&app).quote_free, 200);

    // A seatless swap from a wallet, referred the same way.
    let user = h.user(0, 1_000_000_000);
    let s = Swap {
        side: 0,
        base_lots: 2,
        quote_budget: 1_000_000_000,
        limit_price: 100_000,
        min_out: 0,
        match_limit: 0,
    };
    h.swap_with(&user, s, Some((app.key(), app_seat))).unwrap();
    assert_eq!(h.trader(&app).quote_free, 300);
    h.audit();
}

#[test]
fn permissioned_markets_need_the_authority_to_admit_traders() {
    let mut h = H::new(
        CreateMarketArgsLite {
            flags: 1,
            ..default_market()
        },
        false,
    );
    let a = h.user(10 * LOT, 0);
    // Without the authority's signature the seat is refused.
    assert!(h.claim_seat(&a).is_err());
    let ix = h.ix(
        build::claim_seat(),
        vec![
            AccountMeta::new(a.key(), true),
            AccountMeta::new_readonly(a.key(), true),
            AccountMeta::new(h.market, false),
            AccountMeta::new_readonly(SYSTEM, false),
            AccountMeta::new_readonly(h.authority.pubkey(), true),
        ],
    );
    let authority = h.authority.insecure_clone();
    h.send(&[ix], &[&a.kp, &authority]).unwrap();
    h.deposit(&a, true, 10 * LOT).unwrap();
    h.place(&a, limit(1, 100_000, 5)).unwrap();

    // Wallets without a seat cannot trade through swaps.
    let user = h.user(0, 1_000_000_000);
    let s = Swap {
        side: 0,
        base_lots: 1,
        quote_budget: 1_000_000_000,
        limit_price: 100_000,
        min_out: 0,
        match_limit: 0,
    };
    assert_eq!(err_code(&h.swap(&user, s)), Some(PERMISSIONED));
    h.audit();
}
