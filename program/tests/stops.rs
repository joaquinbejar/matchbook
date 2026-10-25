//! Stops and brackets: orders that wait for a trigger price, set off by
//! ordinary trades without a keeper.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    missing_docs
)]

mod common;

use common::*;
use matchbook_program::events::{FILL_LEN, OUT_LEN, REVERSED_LEN};
use matchbook_program::instruction::{build, OrderArgs, StopArgs};
use solana_instruction::AccountMeta;

const LOT: u64 = 1_000_000;
const ABOVE: u8 = 0;
const BELOW: u8 = 1;
const STOP_WOULD_TRIGGER: u32 = 26;

fn ioc(side: u8, price: u64, lots: u64) -> OrderArgs {
    OrderArgs {
        order_type: 3,
        ..limit(side, price, lots)
    }
}

/// Places a stop and returns its id.
fn stop(
    h: &mut H,
    u: &User,
    direction: u8,
    trigger: u64,
    order: OrderArgs,
    oco_with: u64,
) -> Result<u64, Option<u32>> {
    let args = StopArgs {
        order,
        trigger_price: trigger,
        oco_with,
    };
    let ix = h.ix(
        build::place_stop(h.seat(u).unwrap_or(u32::MAX), direction, &args),
        h.owner_metas(u),
    );
    let r = h.send(&[ix], &[&u.kp]);
    match &r {
        Ok(m) => Ok(u64::from_le_bytes(
            m.return_data.data[..8].try_into().unwrap(),
        )),
        Err(_) => Err(err_code(&r)),
    }
}

/// Reasons of the order-out events in a transaction's logs.
fn out_reasons(logs: &[String]) -> Vec<u8> {
    use base64::Engine as _;
    let mut v = vec![];
    for l in logs {
        let Some(data) = l.strip_prefix("Program data: ") else {
            continue;
        };
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(data)
            .unwrap();
        let mut i = 0;
        while let Some(&t) = bytes.get(i) {
            let len = match t {
                1 => FILL_LEN,
                2 => OUT_LEN,
                3 => REVERSED_LEN,
                _ => unreachable!("unknown event {t}"),
            };
            if t == 2 {
                v.push(bytes[i + len - 1]);
            }
            i += len;
        }
    }
    v
}

#[test]
fn bracket_stop_loss_fires_on_a_trade_and_cancels_the_take_profit() {
    let mut h = H::new(default_market(), false);
    let mm = h.user(0, 10_000_000_000);
    let a = h.user(10 * LOT, 0);
    let b = h.user(10 * LOT, 0);
    for u in [&mm, &a, &b] {
        h.claim_seat(u).unwrap();
    }
    h.deposit(&mm, false, 10_000_000_000).unwrap();
    h.deposit(&a, true, 10 * LOT).unwrap();
    h.deposit(&b, true, 10 * LOT).unwrap();
    h.batch(&mm, 0, &[], &[limit(0, 94_000, 10), limit(0, 93_000, 10)])
        .unwrap();

    // A protects a long position: sell 4 lots if a trade happens at 95,000
    // or lower (stop-loss), or at 110,000 or higher (take-profit).
    let sl = stop(&mut h, &a, BELOW, 95_000, ioc(1, 90_000, 4), 0).unwrap();
    stop(&mut h, &a, ABOVE, 110_000, ioc(1, 105_000, 4), sl).unwrap();
    assert_eq!(h.trader(&a).base_free, 2 * LOT);
    h.audit();

    // B's unrelated sale trades at 94,000 and sets off the stop-loss in the
    // same transaction: A sells 4 lots into the 94,000 bid and the
    // take-profit is cancelled.
    let r = h.place(&b, ioc(1, 94_000, 1)).unwrap();
    assert_eq!(out_reasons(&r.logs), vec![4, 5]);
    let t = h.trader(&a);
    assert_eq!(t.base_free, 6 * LOT);
    assert_eq!(t.quote_free, 4 * 94_000 - 376);
    assert_eq!(h.with_market(|m| m.header().stop_count), 0);
    h.audit();

    // A stop already met by the last trade is refused.
    assert_eq!(
        stop(&mut h, &a, BELOW, 95_000, ioc(1, 90_000, 1), 0),
        Err(Some(STOP_WOULD_TRIGGER))
    );
}

#[test]
fn anyone_can_trigger_stops_beyond_the_automatic_ones() {
    let mut h = H::new(default_market(), false);
    let mm = h.user(20 * LOT, 10_000_000_000);
    let b = h.user(0, 10_000_000_000);
    h.claim_seat(&mm).unwrap();
    h.claim_seat(&b).unwrap();
    h.deposit(&mm, true, 20 * LOT).unwrap();
    h.deposit(&mm, false, 10_000_000_000).unwrap();
    h.deposit(&b, false, 10_000_000_000).unwrap();
    h.place(&mm, limit(1, 100_000, 10)).unwrap();
    // Three stop buys (breakout entries) above 100,000, resting as limits.
    for k in 0..3 {
        stop(&mut h, &b, ABOVE, 100_000, limit(0, 99_000 - k, 1), 0).unwrap();
    }
    let r = h.place(&b, ioc(0, 100_000, 1)).unwrap();
    // Two trigger with the trade itself.
    assert_eq!(out_reasons(&r.logs), vec![4, 4]);
    assert_eq!(h.with_market(|m| m.header().stop_count), 1);
    // The third waits for anyone to push it through.
    let ix = h.ix(
        build::trigger_stops(10),
        vec![AccountMeta::new(h.market, false)],
    );
    let r = h.send(&[ix], &[]).unwrap();
    assert_eq!(out_reasons(&r.logs), vec![4]);
    assert_eq!(h.with_market(|m| m.header().stop_count), 0);
    assert_eq!(h.with_market(|m| m.header().order_count), 1 + 3);
    h.audit();
}
