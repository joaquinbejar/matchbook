//! Reverse orders: liquidity that re-posts itself on the other side.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    missing_docs
)]

mod common;

use common::*;
use matchbook_engine::Side;
use matchbook_program::instruction::OrderArgs;

const LOT: u64 = 1_000_000;

/// `(owner seat, price, lots)` for every order on `side`, best first.
fn book(h: &H, side: Side) -> Vec<(u32, u64, u64)> {
    h.with_market(|m| {
        let mut v = vec![];
        let mut i = if side == Side::Bid {
            m.best_bid()
        } else {
            m.best_ask()
        };
        while i != matchbook_engine::NIL {
            let o = m.order(i).unwrap();
            v.push((o.trader, o.price(), o.base_lots));
            i = m.next_order(side, i).unwrap();
        }
        v
    })
}

#[test]
fn fills_flip_to_the_other_side() {
    let mut h = H::new(default_market(), false);
    let lp = h.user(10 * LOT, 0);
    let taker = h.user(10 * LOT, 10_000_000_000);
    h.claim_seat(&lp).unwrap();
    h.claim_seat(&taker).unwrap();
    h.deposit(&lp, true, 10 * LOT).unwrap();
    h.deposit(&taker, true, 10 * LOT).unwrap();
    h.deposit(&taker, false, 10_000_000_000).unwrap();
    let s = h.seat(&lp).unwrap();

    // Sell 10 lots at 100,000 and buy back 1% lower whatever gets filled.
    h.place(
        &lp,
        OrderArgs {
            order_type: 1, // post-only
            reverse_spread: 10_000,
            ..limit(1, 100_000, 10)
        },
    )
    .unwrap();
    let r = h
        .place(
            &taker,
            OrderArgs {
                order_type: 3,
                ..limit(0, 100_000, 4)
            },
        )
        .unwrap();
    assert_eq!(book(&h, Side::Ask), vec![(s, 100_000, 6)]);
    assert_eq!(book(&h, Side::Bid), vec![(s, 99_000, 4)]);
    // The new order is announced in the event log (tag 3).
    assert!(r
        .logs
        .iter()
        .any(|l| l.starts_with("Program data: ") && events_have_tag(l, 3)));
    h.audit();

    // Another buy grows the same bid instead of adding a second one.
    h.place(
        &taker,
        OrderArgs {
            order_type: 3,
            ..limit(0, 100_000, 2)
        },
    )
    .unwrap();
    assert_eq!(book(&h, Side::Ask), vec![(s, 100_000, 4)]);
    assert_eq!(book(&h, Side::Bid), vec![(s, 99_000, 6)]);

    // Selling into the bid flips the lots back onto the original ask.
    h.place(
        &taker,
        OrderArgs {
            order_type: 3,
            ..limit(1, 99_000, 6)
        },
    )
    .unwrap();
    assert!(book(&h, Side::Bid).is_empty());
    assert_eq!(book(&h, Side::Ask), vec![(s, 100_000, 10)]);
    // The LP made the spread: all its base is back in the ask, plus quote.
    let t = h.trader(&lp);
    assert_eq!(t.base_free, 0);
    assert!(t.quote_free > 0);
    h.audit();
}

#[test]
fn reverse_needs_a_resting_order_and_a_spread_below_100_percent() {
    let mut h = H::new(default_market(), false);
    let a = h.user(10 * LOT, 0);
    h.claim_seat(&a).unwrap();
    h.deposit(&a, true, 10 * LOT).unwrap();
    let bad = |spread, ty| OrderArgs {
        order_type: ty,
        reverse_spread: spread,
        ..limit(1, 100_000, 1)
    };
    assert_eq!(err_code(&h.place(&a, bad(1_000_000, 0))), Some(24));
    assert_eq!(err_code(&h.place(&a, bad(1_000, 3))), Some(24));
    h.place(&a, bad(999_999, 0)).unwrap();
    h.audit();
}

fn events_have_tag(log: &str, tag: u8) -> bool {
    use base64::Engine as _;
    let data = log.trim_start_matches("Program data: ");
    let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(data) else {
        return false;
    };
    // Walk the packed events by their fixed sizes.
    let mut i = 0;
    while let Some(&t) = bytes.get(i) {
        if t == tag {
            return true;
        }
        i += match t {
            1 => matchbook_program::events::FILL_LEN,
            2 => matchbook_program::events::OUT_LEN,
            3 => matchbook_program::events::REVERSED_LEN,
            _ => return false,
        };
    }
    false
}
