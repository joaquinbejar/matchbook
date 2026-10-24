//! Hand-written scenarios for every order type and edge case.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    missing_docs
)]

use matchbook_engine::*;

const BASE_LOT: u64 = 1_000;

struct Fx {
    data: Vec<u8>,
}

struct Fill {
    maker_order_id: u64,
    maker_remaining: u64,
}

#[derive(Default)]
struct Rec {
    fills: Vec<Fill>,
    outs: Vec<OrderOutEvent>,
    placed: Vec<PlacedEvent>,
}

impl EventSink for Rec {
    fn on_fill(&mut self, e: &FillEvent<'_>) {
        self.fills.push(Fill {
            maker_order_id: e.maker_order_id,
            maker_remaining: e.maker_remaining,
        });
    }
    fn on_out(&mut self, e: &OrderOutEvent) {
        self.outs.push(*e);
    }
    fn on_placed(&mut self, e: &PlacedEvent) {
        self.placed.push(*e);
    }
}

impl Fx {
    fn new(nodes: usize, taker_bps: u32, rebate_bps: u32) -> Self {
        let mut data = vec![0u8; market_size(nodes)];
        Market::init(
            &mut data,
            &MarketParams {
                base_lot_size: BASE_LOT,
                quote_lot_size: 10,
                min_base_lots: 1,
                taker_fee_bps: taker_bps,
                maker_rebate_bps: rebate_bps,
                ..MarketParams::default()
            },
        )
        .unwrap();
        Self { data }
    }
    fn m(&mut self) -> Market<'_> {
        Market::load_mut(&mut self.data).unwrap()
    }
    fn seat(&mut self, id: u8, base: u64, quote: u64) -> u32 {
        let mut m = self.m();
        let t = m.claim_seat(&[id; 32]).unwrap();
        m.deposit(t, base, quote).unwrap();
        t
    }
    fn check(&mut self) -> MarketTotals {
        self.m().check_invariants().unwrap()
    }
}

#[test]
fn limit_rests_and_crosses_at_maker_price() {
    let mut fx = Fx::new(32, 0, 0);
    let a = fx.seat(1, 10 * BASE_LOT, 0);
    let b = fx.seat(2, 0, 1_000_000);
    let mut ev = Rec::default();
    let mut m = fx.m();
    let r = m
        .place_order(a, &OrderParams::limit(Side::Ask, 100, 10), 0, &mut ev)
        .unwrap();
    assert_eq!(r.rested_base_lots, 10);
    let r = m
        .place_order(b, &OrderParams::limit(Side::Bid, 120, 4), 0, &mut ev)
        .unwrap();
    assert_eq!(r.filled_base_lots, 4);
    assert_eq!(r.filled_quote_atoms, 4 * 100 * 10);
    assert_eq!(r.order_id, None);
    assert_eq!(m.trader(a).unwrap().quote_free, 4_000);
    assert_eq!(m.trader(b).unwrap().base_free, 4 * BASE_LOT);
    assert_eq!(m.trader(b).unwrap().quote_free, 1_000_000 - 4_000);
    assert_eq!(ev.fills.len(), 1);
    assert_eq!(ev.fills[0].maker_remaining, 6);
    let t = fx.check();
    assert_eq!(t.base_atoms, 10 * u128::from(BASE_LOT));
    assert_eq!(t.quote_atoms, 1_000_000);
}

#[test]
fn price_time_priority_and_sweep() {
    let mut fx = Fx::new(64, 0, 0);
    let a = fx.seat(1, 100 * BASE_LOT, 0);
    let c = fx.seat(3, 100 * BASE_LOT, 0);
    let b = fx.seat(2, 0, 10_000_000);
    let mut ev = Rec::default();
    let mut m = fx.m();
    let first = m
        .place_order(a, &OrderParams::limit(Side::Ask, 101, 5), 0, &mut ev)
        .unwrap()
        .order_id
        .unwrap();
    let second = m
        .place_order(c, &OrderParams::limit(Side::Ask, 101, 5), 0, &mut ev)
        .unwrap()
        .order_id
        .unwrap();
    let cheap = m
        .place_order(c, &OrderParams::limit(Side::Ask, 100, 2), 0, &mut ev)
        .unwrap()
        .order_id
        .unwrap();
    let r = m
        .place_order(b, &OrderParams::limit(Side::Bid, 101, 9), 0, &mut ev)
        .unwrap();
    assert_eq!(r.filled_base_lots, 9);
    let ids: Vec<u64> = ev.fills.iter().map(|f| f.maker_order_id).collect();
    assert_eq!(ids, vec![cheap, first, second]);
    assert_eq!(r.filled_quote_atoms, (2 * 100 + 7 * 101) * 10);
    fx.check();
}

#[test]
fn fees_and_rebates_balance() {
    let mut fx = Fx::new(32, 30, 10);
    let a = fx.seat(1, 10 * BASE_LOT, 0);
    let b = fx.seat(2, 0, 1_000_000);
    let mut m = fx.m();
    m.place_order(
        a,
        &OrderParams::limit(Side::Ask, 1_000, 10),
        0,
        &mut NoopSink,
    )
    .unwrap();
    let r = m
        .place_order(
            b,
            &OrderParams::limit(Side::Bid, 1_000, 10),
            0,
            &mut NoopSink,
        )
        .unwrap();
    let notional = 10 * 1_000 * 10;
    assert_eq!(r.fee_atoms, notional * 30 / 10_000);
    assert_eq!(
        m.trader(a).unwrap().quote_free,
        notional + notional * 10 / 10_000
    );
    assert_eq!(m.header().fees_accrued, notional * 20 / 10_000);
    let t = m.check_invariants().unwrap();
    assert_eq!(t.quote_atoms, 1_000_000);
    assert_eq!(m.collect_fees(), 200);
}

#[test]
fn post_only_rejects_and_slides() {
    let mut fx = Fx::new(32, 0, 0);
    let a = fx.seat(1, 10 * BASE_LOT, 0);
    let b = fx.seat(2, 0, 1_000_000);
    let mut m = fx.m();
    m.place_order(a, &OrderParams::limit(Side::Ask, 100, 5), 0, &mut NoopSink)
        .unwrap();
    let e = m.place_order(
        b,
        &OrderParams::limit(Side::Bid, 100, 1).with_type(OrderType::PostOnly),
        0,
        &mut NoopSink,
    );
    assert_eq!(e, Err(EngineError::PostOnlyWouldCross));
    let r = m
        .place_order(
            b,
            &OrderParams::limit(Side::Bid, 105, 1).with_type(OrderType::PostOnlySlide),
            0,
            &mut NoopSink,
        )
        .unwrap();
    assert_eq!(r.rested_price, 99);
    assert_eq!(r.filled_base_lots, 0);
    m.check_invariants().unwrap();
}

#[test]
fn ioc_fok_and_budget() {
    let mut fx = Fx::new(32, 0, 0);
    let a = fx.seat(1, 10 * BASE_LOT, 0);
    let b = fx.seat(2, 0, 1_000_000);
    let mut m = fx.m();
    m.place_order(a, &OrderParams::limit(Side::Ask, 100, 3), 0, &mut NoopSink)
        .unwrap();
    m.place_order(a, &OrderParams::limit(Side::Ask, 110, 3), 0, &mut NoopSink)
        .unwrap();
    let e = m.place_order(
        b,
        &OrderParams::limit(Side::Bid, 105, 4).with_type(OrderType::FillOrKill),
        0,
        &mut NoopSink,
    );
    assert_eq!(e, Err(EngineError::FillOrKillNotFilled));
    m.check_invariants().unwrap();
    let r = m
        .place_order(
            b,
            &OrderParams::limit(Side::Bid, 105, 4).with_type(OrderType::ImmediateOrCancel),
            0,
            &mut NoopSink,
        )
        .unwrap();
    assert_eq!((r.filled_base_lots, r.order_id), (3, None));
    // Market buy with a budget of 2 lots at 110 plus a bit.
    let r = m
        .place_order(
            b,
            &OrderParams::limit(Side::Bid, u64::MAX / 1_000, 100)
                .with_type(OrderType::ImmediateOrCancel)
                .with_budget(2 * 1_100 + 50),
            0,
            &mut NoopSink,
        )
        .unwrap();
    assert_eq!(r.filled_base_lots, 2);
    m.check_invariants().unwrap();
}

#[test]
fn self_trade_modes() {
    let mut fx = Fx::new(32, 0, 0);
    let a = fx.seat(1, 10 * BASE_LOT, 1_000_000);
    let mut m = fx.m();
    let id = m
        .place_order(a, &OrderParams::limit(Side::Ask, 100, 5), 0, &mut NoopSink)
        .unwrap()
        .order_id
        .unwrap();
    let e = m.place_order(
        a,
        &OrderParams::limit(Side::Bid, 100, 2).with_self_trade(SelfTradeBehavior::Abort),
        0,
        &mut NoopSink,
    );
    assert_eq!(e, Err(EngineError::SelfTrade));
    let r = m
        .place_order(
            a,
            &OrderParams::limit(Side::Bid, 100, 2)
                .with_self_trade(SelfTradeBehavior::DecrementTake),
            0,
            &mut NoopSink,
        )
        .unwrap();
    assert_eq!((r.filled_base_lots, r.order_id), (0, None));
    let mut left = 0;
    m.for_each_trader_order(a, |_, o| left = o.base_lots)
        .unwrap();
    assert_eq!(left, 3);
    let mut ev = Rec::default();
    let r = m
        .place_order(a, &OrderParams::limit(Side::Bid, 100, 2), 0, &mut ev)
        .unwrap();
    assert_eq!(ev.outs[0].order_id, id);
    assert_eq!(ev.outs[0].reason, OutReason::SelfTrade);
    assert_eq!(r.rested_base_lots, 2);
    m.check_invariants().unwrap();
}

#[test]
fn expiry_and_prune() {
    let mut fx = Fx::new(32, 0, 0);
    let a = fx.seat(1, 10 * BASE_LOT, 0);
    let b = fx.seat(2, 0, 1_000_000);
    let mut m = fx.m();
    m.place_order(
        a,
        &OrderParams::limit(Side::Ask, 100, 5).with_expiry(50),
        0,
        &mut NoopSink,
    )
    .unwrap();
    assert_eq!(
        m.place_order(
            a,
            &OrderParams::limit(Side::Ask, 100, 1).with_expiry(50),
            50,
            &mut NoopSink
        ),
        Err(EngineError::AlreadyExpired)
    );
    let r = m
        .place_order(b, &OrderParams::limit(Side::Bid, 100, 5), 60, &mut NoopSink)
        .unwrap();
    assert_eq!(r.filled_base_lots, 0);
    assert_eq!(r.rested_base_lots, 5);
    assert_eq!(m.trader(a).unwrap().base_free, 10 * BASE_LOT);
    m.place_order(
        a,
        &OrderParams::limit(Side::Ask, 200, 1).with_expiry(70),
        60,
        &mut NoopSink,
    )
    .unwrap();
    assert_eq!(m.prune_expired(100, 10, &mut NoopSink).unwrap(), 1);
    m.check_invariants().unwrap();
}

#[test]
fn cancel_reduce_and_release_seat() {
    let mut fx = Fx::new(32, 0, 0);
    let a = fx.seat(1, 10 * BASE_LOT, 100_000);
    let mut m = fx.m();
    let x = m
        .place_order(
            a,
            &OrderParams::limit(Side::Ask, 100, 5).with_client_id(7),
            0,
            &mut NoopSink,
        )
        .unwrap()
        .order_id
        .unwrap();
    m.place_order(
        a,
        &OrderParams::limit(Side::Bid, 90, 5).with_client_id(7),
        0,
        &mut NoopSink,
    )
    .unwrap();
    m.place_order(a, &OrderParams::limit(Side::Bid, 80, 5), 0, &mut NoopSink)
        .unwrap();
    m.reduce_order(a, x, 2, &mut NoopSink).unwrap();
    assert_eq!(m.locked(a).unwrap(), (2 * BASE_LOT, (90 * 5 + 80 * 5) * 10));
    assert_eq!(m.cancel_by_client_id(a, 7, &mut NoopSink).unwrap(), 2);
    assert_eq!(m.cancel_all(a, Some(Side::Bid), &mut NoopSink).unwrap(), 1);
    assert_eq!(m.release_seat(a), Err(EngineError::SeatNotEmpty));
    m.withdraw(a, 10 * BASE_LOT, 100_000).unwrap();
    m.release_seat(a).unwrap();
    let t = m.check_invariants().unwrap();
    assert_eq!(
        (t.orders, t.traders, t.base_atoms, t.quote_atoms),
        (0, 0, 0, 0)
    );
}

#[test]
fn swap_without_seat() {
    let mut fx = Fx::new(32, 25, 5);
    let a = fx.seat(1, 10 * BASE_LOT, 1_000_000);
    let mut m = fx.m();
    m.place_order(a, &OrderParams::limit(Side::Ask, 100, 5), 0, &mut NoopSink)
        .unwrap();
    m.place_order(a, &OrderParams::limit(Side::Bid, 90, 5), 0, &mut NoopSink)
        .unwrap();
    let r = m
        .swap(
            &SwapParams {
                side: Side::Bid,
                base_lots: u64::MAX,
                quote_budget: 3_100,
                limit_price: 1_000,
                match_limit: 8,
            },
            0,
            &mut NoopSink,
        )
        .unwrap();
    assert_eq!(r.base_lots, 3);
    assert!(r.quote_net(Side::Bid) <= 3_100);
    let r = m
        .swap(
            &SwapParams {
                side: Side::Ask,
                base_lots: 2,
                quote_budget: 0,
                limit_price: 1,
                match_limit: 8,
            },
            0,
            &mut NoopSink,
        )
        .unwrap();
    assert_eq!(r.quote_atoms, 2 * 90 * 10);
    m.check_invariants().unwrap();
}

#[test]
fn out_of_space_then_grow() {
    let mut fx = Fx::new(3, 0, 0);
    let a = fx.seat(1, 10 * BASE_LOT, 0);
    {
        let mut m = fx.m();
        m.place_order(a, &OrderParams::limit(Side::Ask, 100, 1), 0, &mut NoopSink)
            .unwrap();
        m.place_order(a, &OrderParams::limit(Side::Ask, 101, 1), 0, &mut NoopSink)
            .unwrap();
        assert_eq!(
            m.place_order(a, &OrderParams::limit(Side::Ask, 102, 1), 0, &mut NoopSink),
            Err(EngineError::OutOfSpace)
        );
        assert_eq!(m.bytes_to_grow(1), NODE_SIZE);
        m.check_invariants().unwrap();
    }
    fx.data.extend_from_slice(&[0u8; NODE_SIZE]);
    let mut m = fx.m();
    m.place_order(a, &OrderParams::limit(Side::Ask, 102, 1), 0, &mut NoopSink)
        .unwrap();
    m.check_invariants().unwrap();
}

#[test]
fn levels_view() {
    let mut fx = Fx::new(32, 0, 0);
    let a = fx.seat(1, 100 * BASE_LOT, 0);
    let mut m = fx.m();
    for (p, l) in [(101, 1), (100, 2), (101, 3), (103, 4)] {
        m.place_order(a, &OrderParams::limit(Side::Ask, p, l), 0, &mut NoopSink)
            .unwrap();
    }
    let mut lv = vec![];
    m.for_each_level(Side::Ask, 0, |l| {
        lv.push((l.price, l.base_lots, l.orders));
        true
    })
    .unwrap();
    assert_eq!(lv, vec![(100, 2, 1), (101, 4, 2), (103, 4, 1)]);
}

#[test]
fn tree_stress_thousands_of_orders() {
    let mut fx = Fx::new(5_000, 0, 0);
    let a = fx.seat(1, u64::MAX / 4, u64::MAX / 4);
    let mut m = fx.m();
    let mut ids = vec![];
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut rnd = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    for step in 0..20_000u32 {
        if ids.is_empty() || rnd() % 3 != 0 {
            let r = rnd();
            let (side, price) = if r % 2 == 0 {
                (Side::Ask, 1_000 + r % 1_000)
            } else {
                (Side::Bid, 1 + r % 999)
            };
            if m.free_nodes() == 0 {
                continue;
            }
            let id = m
                .place_order(
                    a,
                    &OrderParams::limit(side, price, 1 + rnd() % 5),
                    0,
                    &mut NoopSink,
                )
                .unwrap()
                .order_id
                .unwrap();
            ids.push(id);
        } else {
            let k = (rnd() as usize) % ids.len();
            let id = ids.swap_remove(k);
            m.cancel_order(a, id, &mut NoopSink).unwrap();
        }
        if step % 997 == 0 {
            m.check_invariants().unwrap();
        }
    }
    m.check_invariants().unwrap();
    assert_eq!(m.header().order_count as usize, ids.len());
}
