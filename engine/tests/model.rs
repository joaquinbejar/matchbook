//! Differential testing: random operation sequences run against the engine
//! and a naive Vec-based reference model must produce identical results,
//! and the engine's invariants must hold after every step.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    missing_docs
)]

use matchbook_engine::*;
use proptest::prelude::*;

const BLS: u64 = 100;
const QLS: u64 = 3;
const TAKER_BPS: u32 = 37;
const REBATE_BPS: u32 = 11;
const TRADERS: usize = 3;

#[derive(Clone, Debug)]
struct MOrder {
    seq: u64,
    trader: usize,
    side: Side,
    price: u64,
    lots: u64,
    coid: u64,
    exp: u64,
}

#[derive(Clone, Debug, Default)]
struct Model {
    base: [u64; TRADERS],
    quote: [u64; TRADERS],
    orders: Vec<MOrder>,
    seq: u64,
    fees: u64,
    capacity: u32,
}

fn fee(q: u64) -> u64 {
    (u128::from(q) * u128::from(TAKER_BPS)).div_ceil(10_000) as u64
}
fn rebate(q: u64) -> u64 {
    (u128::from(q) * u128::from(REBATE_BPS) / 10_000) as u64
}
fn crosses(taker: Side, mp: u64, limit: u64) -> bool {
    match taker {
        Side::Bid => mp <= limit,
        Side::Ask => mp >= limit,
    }
}

impl Model {
    fn free_nodes(&self) -> u32 {
        self.capacity - TRADERS as u32 - self.orders.len() as u32
    }
    /// Indices of `side` orders in priority order.
    fn book(&self, side: Side) -> Vec<usize> {
        let mut v: Vec<usize> = (0..self.orders.len())
            .filter(|&i| self.orders[i].side == side)
            .collect();
        v.sort_by(|&a, &b| {
            let (x, y) = (&self.orders[a], &self.orders[b]);
            let pc = match side {
                Side::Ask => x.price.cmp(&y.price),
                Side::Bid => y.price.cmp(&x.price),
            };
            pc.then(x.seq.cmp(&y.seq))
        });
        v
    }
    fn unlock(&mut self, o: &MOrder) {
        match o.side {
            Side::Ask => self.base[o.trader] += o.lots * BLS,
            Side::Bid => self.quote[o.trader] += o.lots * o.price * QLS,
        }
    }
    fn remove(&mut self, seq: u64) -> MOrder {
        let i = self.orders.iter().position(|o| o.seq == seq).unwrap();
        let o = self.orders.remove(i);
        self.unlock(&o);
        o
    }
    fn affordable(budget: u64, price: u64) -> u64 {
        let mut l = 0u64;
        while {
            let q = (l + 1) * price * QLS;
            q + fee(q) <= budget
        } {
            l += 1;
        }
        l
    }

    #[allow(clippy::too_many_arguments)]
    fn match_book(
        &mut self,
        taker: Option<usize>,
        side: Side,
        limit: u64,
        mut lots: u64,
        mut budget: Option<u64>,
        stp: SelfTradeBehavior,
        ml: u32,
        now: u64,
    ) -> Result<(u64, u64, u64, u32, u64)> {
        let (mut fl, mut fq, mut ff, mut touched) = (0, 0, 0, 0u32);
        while lots > 0 && touched < ml {
            let Some(&i) = self.book(side.opposite()).first() else {
                break;
            };
            let o = self.orders[i].clone();
            if !crosses(side, o.price, limit) {
                break;
            }
            if o.exp != 0 && now >= o.exp {
                touched += 1;
                self.remove(o.seq);
                continue;
            }
            if taker == Some(o.trader) {
                touched += 1;
                match stp {
                    SelfTradeBehavior::Abort => return Err(EngineError::SelfTrade),
                    SelfTradeBehavior::CancelProvide => {
                        self.remove(o.seq);
                    }
                    SelfTradeBehavior::DecrementTake => {
                        let d = lots.min(o.lots);
                        lots -= d;
                        if d == o.lots {
                            self.remove(o.seq);
                        } else {
                            self.orders[i].lots -= d;
                            let part = MOrder {
                                lots: d,
                                ..o.clone()
                            };
                            self.unlock(&part);
                        }
                    }
                }
                continue;
            }
            let mut f = lots.min(o.lots);
            if let Some(b) = budget {
                f = f.min(Self::affordable(b, o.price));
            }
            if f == 0 {
                break;
            }
            touched += 1;
            let q = f * o.price * QLS;
            let (tf, rb) = (fee(q), rebate(q));
            match o.side {
                Side::Ask => self.quote[o.trader] += q + rb,
                Side::Bid => {
                    self.base[o.trader] += f * BLS;
                    self.quote[o.trader] += rb;
                }
            }
            self.fees += tf - rb;
            if f == o.lots {
                self.orders.remove(i);
            } else {
                self.orders[i].lots -= f;
            }
            lots -= f;
            fl += f;
            fq += q;
            ff += tf;
            if let Some(b) = budget.as_mut() {
                *b -= q + tf;
            }
        }
        Ok((fl, fq, ff, touched, lots))
    }

    fn place(&mut self, t: usize, p: &OrderParams, now: u64) -> Result<PlaceOutcome> {
        if p.price == 0 {
            return Err(EngineError::InvalidPrice);
        }
        if p.base_lots == 0 {
            return Err(EngineError::InvalidQuantity);
        }
        if p.expires_at != 0 && p.expires_at <= now {
            return Err(EngineError::AlreadyExpired);
        }
        if p.quote_budget.is_some()
            && (p.order_type != OrderType::ImmediateOrCancel || p.side != Side::Bid)
        {
            return Err(EngineError::BudgetRequiresIoc);
        }
        let ml = p.match_limit.max(1);
        let takes = !matches!(p.order_type, OrderType::PostOnly | OrderType::PostOnlySlide);
        let rests = matches!(
            p.order_type,
            OrderType::Limit | OrderType::PostOnly | OrderType::PostOnlySlide
        );
        let budget = match p.side {
            Side::Ask => {
                if self.base[t] < p.base_lots * BLS {
                    return Err(EngineError::InsufficientFunds);
                }
                None
            }
            Side::Bid => match p.quote_budget {
                Some(b) => {
                    if self.quote[t] < b {
                        return Err(EngineError::InsufficientFunds);
                    }
                    Some(b)
                }
                None => {
                    let n = p.base_lots * p.price * QLS;
                    let need = if takes {
                        n + fee(n) + u64::from(ml).min(p.base_lots)
                    } else {
                        n
                    };
                    if self.quote[t] < need {
                        return Err(EngineError::InsufficientFunds);
                    }
                    Some(self.quote[t])
                }
            },
        };
        if rests && self.free_nodes() < 1 {
            return Err(EngineError::OutOfSpace);
        }
        let mut price = p.price;
        if !takes {
            let live = self.book(p.side.opposite()).into_iter().find(|&i| {
                let o = &self.orders[i];
                !(o.exp != 0 && now >= o.exp)
            });
            if let Some(i) = live {
                let opp = self.orders[i].price;
                if crosses(p.side, opp, price) {
                    if p.order_type == OrderType::PostOnly {
                        return Err(EngineError::PostOnlyWouldCross);
                    }
                    price = match p.side {
                        Side::Bid => opp.checked_sub(1).filter(|v| *v > 0),
                        Side::Ask => opp.checked_add(1),
                    }
                    .ok_or(EngineError::PostOnlyWouldCross)?;
                }
            }
        }
        if p.order_type == OrderType::FillOrKill || p.self_trade == SelfTradeBehavior::Abort {
            let mut sim = self.clone();
            let r = sim.match_book(
                Some(t),
                p.side,
                price,
                p.base_lots,
                None,
                p.self_trade,
                ml,
                now,
            );
            match r {
                Err(e) => return Err(e),
                Ok((fl, ..)) if p.order_type == OrderType::FillOrKill && fl < p.base_lots => {
                    return Err(EngineError::FillOrKillNotFilled)
                }
                _ => {}
            }
        }
        let mut out = PlaceOutcome::default();
        let mut remaining = p.base_lots;
        if takes {
            let (fl, fq, ff, touched, rem) = self.match_book(
                Some(t),
                p.side,
                price,
                p.base_lots,
                budget,
                p.self_trade,
                ml,
                now,
            )?;
            match p.side {
                Side::Bid => {
                    self.quote[t] -= fq + ff;
                    self.base[t] += fl * BLS;
                }
                Side::Ask => {
                    self.base[t] -= fl * BLS;
                    self.quote[t] += fq - ff;
                }
            }
            out.filled_base_lots = fl;
            out.filled_quote_atoms = fq;
            out.fee_atoms = ff;
            out.makers_touched = touched;
            remaining = rem;
        }
        let can_rest = self
            .book(p.side.opposite())
            .into_iter()
            .map(|i| &self.orders[i])
            .take_while(|o| crosses(p.side, o.price, price))
            .all(|o| o.exp != 0 && now >= o.exp);
        if rests && remaining > 0 && can_rest {
            match p.side {
                Side::Ask => self.base[t] -= remaining * BLS,
                Side::Bid => self.quote[t] -= remaining * price * QLS,
            }
            self.seq += 1;
            self.orders.push(MOrder {
                seq: self.seq,
                trader: t,
                side: p.side,
                price,
                lots: remaining,
                coid: p.client_order_id,
                exp: p.expires_at,
            });
            out.order_id = Some(self.seq);
            out.rested_price = price;
            out.rested_base_lots = remaining;
        }
        Ok(out)
    }
}

#[derive(Clone, Debug)]
enum Op {
    Place {
        t: usize,
        side: bool,
        price: u64,
        lots: u64,
        ty: u8,
        stp: u8,
        exp: Option<u64>,
        budget: Option<u64>,
        ml: u32,
        coid: u64,
    },
    Cancel {
        t: usize,
        nth: usize,
    },
    CancelCoid {
        t: usize,
        coid: u64,
    },
    CancelAll {
        t: usize,
        side: Option<bool>,
    },
    Reduce {
        t: usize,
        nth: usize,
        lots: u64,
    },
    Swap {
        side: bool,
        lots: u64,
        budget: u64,
        limit: u64,
        ml: u32,
    },
    Withdraw {
        t: usize,
        base: u64,
        quote: u64,
    },
    Deposit {
        t: usize,
        base: u64,
        quote: u64,
    },
    Tick(u64),
    Prune(u32),
    Grow(u32),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        8 => (0..TRADERS, any::<bool>(), 1u64..30, 1u64..12, 0u8..5, 0u8..3, proptest::option::of(1u64..40), proptest::option::of(0u64..20_000), 1u32..8, 0u64..4)
            .prop_map(|(t, side, price, lots, ty, stp, exp, budget, ml, coid)| Op::Place { t, side, price, lots, ty, stp, exp, budget, ml, coid }),
        2 => (0..TRADERS, 0usize..6).prop_map(|(t, nth)| Op::Cancel { t, nth }),
        1 => (0..TRADERS, 0u64..4).prop_map(|(t, coid)| Op::CancelCoid { t, coid }),
        1 => (0..TRADERS, proptest::option::of(any::<bool>())).prop_map(|(t, side)| Op::CancelAll { t, side }),
        1 => (0..TRADERS, 0usize..6, 0u64..10).prop_map(|(t, nth, lots)| Op::Reduce { t, nth, lots }),
        2 => (any::<bool>(), 1u64..15, 0u64..30_000, 1u64..30, 1u32..8).prop_map(|(side, lots, budget, limit, ml)| Op::Swap { side, lots, budget, limit, ml }),
        1 => (0..TRADERS, 0u64..3_000, 0u64..20_000).prop_map(|(t, base, quote)| Op::Withdraw { t, base, quote }),
        1 => (0..TRADERS, 0u64..3_000, 0u64..20_000).prop_map(|(t, base, quote)| Op::Deposit { t, base, quote }),
        1 => (1u64..15).prop_map(Op::Tick),
        1 => (1u32..5).prop_map(Op::Prune),
        1 => (1u32..6).prop_map(Op::Grow),
    ]
}

fn snapshot(
    m: &Market<'_>,
    seats: &[u32],
    now: u64,
) -> (
    Vec<(u64, u64)>,
    Vec<(u64, u64, u64)>,
    Vec<(u64, u64, u64)>,
    u64,
) {
    let bal = seats
        .iter()
        .map(|&s| {
            let t = m.trader(s).unwrap();
            (t.base_free, t.quote_free)
        })
        .collect();
    let mut sides = [vec![], vec![]];
    for (k, side) in [Side::Bid, Side::Ask].into_iter().enumerate() {
        let mut i = match side {
            Side::Bid => m.best_bid(),
            Side::Ask => m.best_ask(),
        };
        while i != NIL {
            let o = m.order(i).unwrap();
            sides[k].push((o.seq, o.price(), o.base_lots));
            i = m.next_order(side, i).unwrap();
        }
    }
    let _ = now;
    let [b, a] = sides;
    (bal, b, a, m.header().fees_accrued)
}

fn model_snapshot(
    md: &Model,
) -> (
    Vec<(u64, u64)>,
    Vec<(u64, u64, u64)>,
    Vec<(u64, u64, u64)>,
    u64,
) {
    let bal = (0..TRADERS).map(|t| (md.base[t], md.quote[t])).collect();
    let f = |s| {
        md.book(s)
            .into_iter()
            .map(|i| {
                let o = &md.orders[i];
                (o.seq, o.price, o.lots)
            })
            .collect()
    };
    (bal, f(Side::Bid), f(Side::Ask), md.fees)
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 2_000, .. ProptestConfig::default() })]

    #[test]
    fn engine_matches_model(ops in proptest::collection::vec(op(), 1..120)) {
        let cap = (TRADERS + 8) as u32;
        let mut data = vec![0u8; market_size(cap as usize)];
        Market::init(&mut data, &MarketParams {
            base_lot_size: BLS, quote_lot_size: QLS, min_base_lots: 1,
            taker_fee_bps: TAKER_BPS, maker_rebate_bps: REBATE_BPS, ..MarketParams::default()
        }).unwrap();
        let mut md = Model { capacity: cap, ..Model::default() };
        let mut seats = vec![];
        let (mut vault_base, mut vault_quote) = (0u128, 0u128);
        {
            let mut m = Market::load_mut(&mut data).unwrap();
            for t in 0..TRADERS {
                let s = m.claim_seat(&[t as u8 + 1; 32]).unwrap();
                m.deposit(s, 2_000, 30_000).unwrap();
                md.base[t] = 2_000;
                md.quote[t] = 30_000;
                vault_base += 2_000;
                vault_quote += 30_000;
                seats.push(s);
            }
        }
        let mut now = 0u64;
        // Model sequence number -> engine order id (which also encodes the node index).
        let mut ids: std::collections::HashMap<u64, u64> = std::collections::HashMap::new();
        let mut stale: Vec<u64> = vec![];
        for op in ops {
            if let Op::Grow(n) = op {
                data.extend(std::iter::repeat_n(0u8, n as usize * NODE_SIZE));
                md.capacity += n;
                continue;
            }
            let mut m = Market::load_mut(&mut data).unwrap();
            match op {
                Op::Place { t, side, price, lots, ty, stp, exp, budget, ml, coid } => {
                    let side = if side { Side::Bid } else { Side::Ask };
                    let p = OrderParams {
                        side, price, base_lots: lots,
                        order_type: OrderType::from_u8(ty).unwrap(),
                        self_trade: SelfTradeBehavior::from_u8(stp).unwrap(),
                        client_order_id: coid,
                        expires_at: exp.map_or(0, |e| now + e),
                        quote_budget: budget,
                        match_limit: ml,
                    };
                    let mut a = m.place_order(seats[t], &p, now, &mut NoopSink);
                    let b = md.place(t, &p, now);
                    if let Ok(out) = a.as_mut() {
                        if let Some(id) = out.order_id {
                            let (seq, _) = split_order_id(id);
                            ids.insert(seq, id);
                            out.order_id = Some(seq);
                        }
                    }
                    prop_assert_eq!(a, b, "place {:?}", p);
                }
                Op::Cancel { t, nth } => {
                    let mine: Vec<u64> = md.orders.iter().filter(|o| o.trader == t).map(|o| o.seq).collect();
                    // Past the trader's live orders, try a stale id whose node may have been reused.
                    let id = mine.get(nth).copied().unwrap_or(u64::MAX);
                    let eid = match ids.get(&id) {
                        Some(&e) => e,
                        // Otherwise another trader's live order, then a stale id.
                        None => md.orders.iter().find(|o| o.trader != t).filter(|_| nth % 2 == 0)
                            .map(|o| ids[&o.seq])
                            .unwrap_or_else(|| stale.get(nth).copied().unwrap_or(u64::MAX)),
                    };
                    let a = m.cancel_order(seats[t], eid, &mut NoopSink);
                    let b = if mine.contains(&id) { md.remove(id); stale.push(eid); Ok(()) } else { Err(EngineError::OrderNotFound) };
                    prop_assert_eq!(a, b);
                }
                Op::CancelCoid { t, coid } => {
                    let ids: Vec<u64> = md.orders.iter().filter(|o| o.trader == t && o.coid == coid).map(|o| o.seq).collect();
                    for id in &ids { md.remove(*id); }
                    prop_assert_eq!(m.cancel_by_client_id(seats[t], coid, &mut NoopSink), Ok(ids.len() as u32));
                }
                Op::CancelAll { t, side } => {
                    let side = side.map(|b| if b { Side::Bid } else { Side::Ask });
                    let ids: Vec<u64> = md.orders.iter().filter(|o| o.trader == t && side.is_none_or(|s| s == o.side)).map(|o| o.seq).collect();
                    for id in &ids { md.remove(*id); }
                    prop_assert_eq!(m.cancel_all(seats[t], side, &mut NoopSink), Ok(ids.len() as u32));
                }
                Op::Reduce { t, nth, lots } => {
                    let mine: Vec<u64> = md.orders.iter().filter(|o| o.trader == t).map(|o| o.seq).collect();
                    let Some(&id) = mine.get(nth) else { continue };
                    let a = m.reduce_order(seats[t], ids[&id], lots, &mut NoopSink);
                    let i = md.orders.iter().position(|o| o.seq == id).unwrap();
                    let cur = md.orders[i].lots;
                    let b = if lots >= cur { Err(EngineError::InvalidQuantity) }
                        else if lots == 0 { md.remove(id); Ok(()) }
                        else { let part = MOrder { lots: cur - lots, ..md.orders[i].clone() }; md.unlock(&part); md.orders[i].lots = lots; Ok(()) };
                    prop_assert_eq!(a, b);
                }
                Op::Swap { side, lots, budget, limit, ml } => {
                    let side = if side { Side::Bid } else { Side::Ask };
                    let sp = SwapParams { side, base_lots: lots, quote_budget: budget, limit_price: limit, match_limit: ml };
                    let a = m.swap(&sp, now, &mut NoopSink);
                    let b = md.match_book(None, side, limit, lots, (side == Side::Bid).then_some(budget), SelfTradeBehavior::CancelProvide, ml, now);
                    let (fl, fq, ff, touched, _) = b.unwrap();
                    let a = a.unwrap();
                    prop_assert_eq!((a.base_lots, a.quote_atoms, a.fee_atoms, a.makers_touched), (fl, fq, ff, touched));
                    match side {
                        Side::Bid => { vault_quote += u128::from(fq + ff); vault_base -= u128::from(fl * BLS); }
                        Side::Ask => { vault_base += u128::from(fl * BLS); vault_quote -= u128::from(fq - ff); }
                    }
                }
                Op::Withdraw { t, base, quote } => {
                    let a = m.withdraw(seats[t], base, quote);
                    let b = if md.base[t] < base || md.quote[t] < quote { Err(EngineError::InsufficientFunds) } else {
                        md.base[t] -= base; md.quote[t] -= quote; vault_base -= u128::from(base); vault_quote -= u128::from(quote); Ok(())
                    };
                    prop_assert_eq!(a, b);
                }
                Op::Deposit { t, base, quote } => {
                    m.deposit(seats[t], base, quote).unwrap();
                    md.base[t] += base; md.quote[t] += quote;
                    vault_base += u128::from(base); vault_quote += u128::from(quote);
                }
                Op::Tick(d) => now += d,
                Op::Prune(limit) => {
                    let n = m.prune_expired(now, limit, &mut NoopSink).unwrap();
                    let mut removed = 0;
                    for side in [Side::Bid, Side::Ask] {
                        for i in md.book(side) {
                            if removed >= limit { break; }
                            let o = md.orders[i].clone();
                            if o.exp != 0 && now >= o.exp { removed += 1; let _ = o; }
                        }
                    }
                    // Apply removals in the same order as the engine (both sides, top down).
                    let mut left = limit;
                    for side in [Side::Bid, Side::Ask] {
                        let ids: Vec<u64> = md.book(side).into_iter().map(|i| md.orders[i].clone()).filter(|o| o.exp != 0 && now >= o.exp).map(|o| o.seq).collect();
                        for id in ids { if left == 0 { break; } md.remove(id); left -= 1; }
                    }
                    prop_assert_eq!(n, removed);
                }
                Op::Grow(_) => unreachable!(),
            }
            let totals = m.check_invariants();
            prop_assert!(totals.is_ok(), "invariants: {:?}", totals);
            let totals = totals.unwrap();
            prop_assert_eq!(totals.base_atoms, vault_base);
            prop_assert_eq!(totals.quote_atoms, vault_quote);
            prop_assert_eq!(snapshot(&m, &seats, now), model_snapshot(&md));
            // Live book is never crossed.
            let mut bb = None; let mut ba = None;
            m.for_each_order(Side::Bid, now, |o| { bb = Some(o.price); false }).unwrap();
            m.for_each_order(Side::Ask, now, |o| { ba = Some(o.price); false }).unwrap();
            if let (Some(b), Some(a)) = (bb, ba) { prop_assert!(b < a, "crossed {} {}", b, a); }
        }
    }
}
