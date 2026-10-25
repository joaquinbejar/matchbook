//! Differential testing: random operation sequences run against the engine
//! and a naive Vec-based reference model must produce identical results,
//! and the engine's invariants must hold after every step.
#![allow(
    clippy::type_complexity,
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
    rev: u32,
    /// The reverse order this one flips into, by sequence number.
    partner: Option<u64>,
}

#[derive(Clone, Debug)]
struct MStop {
    seq: u64,
    trader: usize,
    above: bool,
    trigger: u64,
    side: Side,
    price: u64,
    lots: u64,
    ty: OrderType,
    stp: SelfTradeBehavior,
    ml: u32,
    coid: u64,
    exp: u64,
    partner: Option<u64>,
}

#[derive(Clone, Debug, Default)]
struct Model {
    stops: Vec<MStop>,
    last: u64,
    /// Highest and lowest trade prices since the stops last caught up.
    high: Option<u64>,
    low: Option<u64>,
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
    /// Free nodes, less one held for each reverse order without a partner.
    fn free_nodes(&self) -> u32 {
        let reserved = self
            .orders
            .iter()
            .filter(|o| o.rev != 0 && o.partner.is_none())
            .count();
        self.capacity
            - TRADERS as u32
            - self.orders.len() as u32
            - self.stops.len() as u32
            - reserved as u32
    }
    fn stop_lock(side: Side, price: u64, lots: u64, ml: u32) -> u64 {
        match side {
            Side::Ask => lots * BLS,
            Side::Bid => {
                let n = lots * price * QLS;
                let slack = if TAKER_BPS == 0 {
                    0
                } else {
                    u64::from(ml).min(lots)
                };
                n + fee(n) + slack
            }
        }
    }
    fn credit(&mut self, t: usize, side: Side, amount: u64) {
        match side {
            Side::Ask => self.base[t] += amount,
            Side::Bid => self.quote[t] += amount,
        }
    }
    fn met(above: bool, trigger: u64, last: u64) -> bool {
        last != 0
            && if above {
                last >= trigger
            } else {
                last <= trigger
            }
    }
    fn place_stop(&mut self, t: usize, sp: &StopParams, now: u64) -> Result<u64> {
        let o = &sp.order;
        if o.price == 0 {
            return Err(EngineError::InvalidPrice);
        }
        if o.base_lots == 0 {
            return Err(EngineError::InvalidQuantity);
        }
        if o.expires_at != 0 && o.expires_at <= now {
            return Err(EngineError::AlreadyExpired);
        }
        if sp.trigger_price == 0 || o.quote_budget.is_some() || o.reverse_spread != 0 {
            return Err(EngineError::InvalidStop);
        }
        let above = sp.direction == TriggerDirection::Above;
        if !self.stops.iter().any(|s| s.above == above) {
            if above {
                self.high = None;
            } else {
                self.low = None;
            }
        }
        let reach = if above { self.high } else { self.low };
        if Self::met(above, sp.trigger_price, self.last)
            || Self::met(above, sp.trigger_price, reach.unwrap_or(0))
        {
            return Err(EngineError::StopWouldTrigger);
        }
        let partner = if sp.oco_with == 0 {
            None
        } else {
            match self
                .stops
                .iter()
                .find(|s| s.seq == sp.oco_with && s.trader == t)
            {
                Some(s) if s.partner.is_none() => Some(s.seq),
                _ => return Err(EngineError::InvalidStop),
            }
        };
        let ml = o.match_limit.clamp(1, MAX_STOP_MATCH_LIMIT);
        let amount = Self::stop_lock(o.side, o.price, o.base_lots, ml);
        let free = match o.side {
            Side::Ask => self.base[t],
            Side::Bid => self.quote[t],
        };
        if free < amount {
            return Err(EngineError::InsufficientFunds);
        }
        if self.free_nodes() == 0 {
            return Err(EngineError::OutOfSpace);
        }
        match o.side {
            Side::Ask => self.base[t] -= amount,
            Side::Bid => self.quote[t] -= amount,
        }
        self.seq += 1;
        if let Some(ps) = partner {
            self.stops.iter_mut().find(|s| s.seq == ps).unwrap().partner = Some(self.seq);
        }
        self.stops.push(MStop {
            seq: self.seq,
            trader: t,
            above,
            trigger: sp.trigger_price,
            side: o.side,
            price: o.price,
            lots: o.base_lots,
            ty: o.order_type,
            stp: o.self_trade,
            ml,
            coid: o.client_order_id,
            exp: o.expires_at,
            partner,
        });
        Ok(self.seq)
    }
    fn remove_stop(&mut self, seq: u64) -> MStop {
        let i = self.stops.iter().position(|s| s.seq == seq).unwrap();
        let s = self.stops.remove(i);
        if let Some(ps) = s.partner {
            self.stops.iter_mut().find(|q| q.seq == ps).unwrap().partner = None;
        }
        self.credit(
            s.trader,
            s.side,
            Self::stop_lock(s.side, s.price, s.lots, s.ml),
        );
        s
    }
    fn run_stops(&mut self, now: u64, limit: u32) -> Result<u32> {
        let mut n = 0;
        while n < limit {
            let high = self.high.map_or(self.last, |h| h.max(self.last));
            let low = self.low.map_or(self.last, |l| l.min(self.last));
            let above = self
                .stops
                .iter()
                .filter(|s| s.above && Self::met(true, s.trigger, high))
                .min_by_key(|s| (s.trigger, s.seq))
                .map(|s| s.seq);
            let below = || {
                self.stops
                    .iter()
                    .filter(|s| !s.above && Self::met(false, s.trigger, low))
                    .min_by_key(|s| (std::cmp::Reverse(s.trigger), s.seq))
                    .map(|s| s.seq)
            };
            let Some(seq) = above.or_else(below) else {
                self.high = None;
                self.low = None;
                break;
            };
            n += 1;
            let s = self.remove_stop(seq);
            if s.exp != 0 && now >= s.exp {
                continue;
            }
            if let Some(ps) = s.partner {
                self.remove_stop(ps);
            }
            let p = OrderParams {
                side: s.side,
                price: s.price,
                base_lots: s.lots,
                order_type: s.ty,
                self_trade: s.stp,
                client_order_id: s.coid,
                expires_at: s.exp,
                quote_budget: None,
                match_limit: s.ml,
                reverse_spread: 0,
            };
            match self.place_only(s.trader, &p, now) {
                Ok(_)
                | Err(
                    EngineError::PostOnlyWouldCross
                    | EngineError::FillOrKillNotFilled
                    | EngineError::SelfTrade
                    | EngineError::OutOfSpace
                    | EngineError::InvalidQuantity
                    | EngineError::InsufficientFunds,
                ) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(n)
    }
    fn place(&mut self, t: usize, p: &OrderParams, now: u64) -> Result<PlaceOutcome> {
        // The engine skips stops for an order that cannot reach the book.
        let reaches = self
            .book(p.side.opposite())
            .first()
            .is_some_and(|&i| crosses(p.side, self.orders[i].price, p.price));
        let out = self.place_only(t, p, now)?;
        if reaches {
            self.run_stops(now, AUTO_TRIGGER_LIMIT)?;
        }
        Ok(out)
    }
    fn swap(&mut self, sp: &SwapParams, now: u64) -> Result<(u64, u64, u64, u32, u64)> {
        let side = sp.side;
        let r = self.match_book(
            None,
            side,
            sp.limit_price,
            sp.base_lots,
            (side == Side::Bid).then_some(sp.quote_budget),
            SelfTradeBehavior::CancelProvide,
            sp.match_limit.max(1),
            now,
        )?;
        self.run_stops(now, AUTO_TRIGGER_LIMIT)?;
        Ok(r)
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
    /// Takes an order off the book without unlocking it, unlinking its partner.
    fn take(&mut self, seq: u64) -> MOrder {
        let i = self.orders.iter().position(|o| o.seq == seq).unwrap();
        let o = self.orders.remove(i);
        if let Some(ps) = o.partner {
            self.orders
                .iter_mut()
                .find(|q| q.seq == ps)
                .unwrap()
                .partner = None;
        }
        o
    }
    fn remove(&mut self, seq: u64) -> MOrder {
        let o = self.take(seq);
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
        let mut last = 0;
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
            if o.rev != 0 {
                let rside = o.side.opposite();
                let lock = |md: &mut Self, price: u64| match rside {
                    Side::Bid => md.quote[o.trader] -= f * price * QLS,
                    Side::Ask => md.base[o.trader] -= f * BLS,
                };
                if let Some(ps) = o.partner {
                    // Grow the existing partner, keeping its price and place.
                    let j = self.orders.iter().position(|q| q.seq == ps).unwrap();
                    let pp = self.orders[j].price;
                    lock(self, pp);
                    self.orders[j].lots += f;
                } else {
                    let scale = 1_000_000u64;
                    let np = match rside {
                        Side::Bid => o.price * (scale - u64::from(o.rev)) / scale,
                        Side::Ask => (o.price * (scale + u64::from(o.rev))).div_ceil(scale),
                    };
                    if np != 0 {
                        lock(self, np);
                        self.seq += 1;
                        self.orders.push(MOrder {
                            seq: self.seq,
                            side: rside,
                            price: np,
                            lots: f,
                            partner: Some(o.seq),
                            ..o.clone()
                        });
                        self.orders[i].partner = Some(self.seq);
                    }
                }
            }
            if f == o.lots {
                self.take(o.seq);
            } else {
                self.orders[i].lots -= f;
            }
            lots -= f;
            last = o.price;
            fl += f;
            fq += q;
            ff += tf;
            if let Some(b) = budget.as_mut() {
                *b -= q + tf;
            }
        }
        if last != 0 {
            self.last = last;
            if !self.stops.is_empty() {
                self.high = Some(self.high.map_or(last, |h| h.max(last)));
                self.low = Some(self.low.map_or(last, |l| l.min(last)));
            }
        }
        Ok((fl, fq, ff, touched, lots))
    }

    fn place_only(&mut self, t: usize, p: &OrderParams, now: u64) -> Result<PlaceOutcome> {
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
        let rests = matches!(
            p.order_type,
            OrderType::Limit | OrderType::PostOnly | OrderType::PostOnlySlide
        );
        if p.reverse_spread != 0 && (p.reverse_spread >= 1_000_000 || !rests) {
            return Err(EngineError::InvalidReverseSpread);
        }
        let takes = !matches!(p.order_type, OrderType::PostOnly | OrderType::PostOnlySlide);
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
                    // The fee buffer is needed only if the order reaches the best ask.
                    let reaches = self
                        .book(Side::Ask)
                        .first()
                        .is_some_and(|&i| self.orders[i].price <= p.price);
                    let need = if takes && reaches {
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
        if rests && self.free_nodes() < 1 + u32::from(p.reverse_spread != 0) {
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
                rev: p.reverse_spread,
                partner: None,
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
        rev: u32,
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
    PlaceStop {
        t: usize,
        side: bool,
        price: u64,
        lots: u64,
        ty: u8,
        stp: u8,
        exp: Option<u64>,
        ml: u32,
        coid: u64,
        trigger: u64,
        above: bool,
        oco: Option<usize>,
        bad: u8,
    },
    TriggerStops(u32),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        8 => (0..TRADERS, any::<bool>(), 1u64..30, 1u64..12, 0u8..5, 0u8..3, proptest::option::of(1u64..40), proptest::option::of(0u64..20_000), 1u32..8, 0u64..4, prop_oneof![3 => Just(0u32), 2 => 1u32..400_000, 1 => 999_000u32..1_000_001])
            .prop_map(|(t, side, price, lots, ty, stp, exp, budget, ml, coid, rev)| Op::Place { t, side, price, lots, ty, stp, exp, budget, ml, coid, rev }),
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
        4 => (0..TRADERS, any::<bool>(), 1u64..30, 1u64..12, 0u8..5, 0u8..3, proptest::option::of(1u64..40), (0u32..40, 0u64..4, 1u64..30, any::<bool>(), proptest::option::of(0usize..4), prop_oneof![12 => Just(0u8), 1 => 1u8..3]))
            .prop_map(|(t, side, price, lots, ty, stp, exp, (ml, coid, trigger, above, oco, bad))| Op::PlaceStop { t, side, price, lots, ty, stp, exp, ml, coid, trigger, above, oco, bad }),
        1 => (0u32..4).prop_map(Op::TriggerStops),
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
    #![proptest_config(ProptestConfig { cases: std::env::var("MODEL_CASES").ok().and_then(|v| v.parse().ok()).unwrap_or(2_000), .. ProptestConfig::default() })]

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
                Op::Place { t, side, price, lots, ty, stp, exp, budget, ml, coid, rev } => {
                    let side = if side { Side::Bid } else { Side::Ask };
                    let p = OrderParams {
                        side, price, base_lots: lots,
                        order_type: OrderType::from_u8(ty).unwrap(),
                        self_trade: SelfTradeBehavior::from_u8(stp).unwrap(),
                        client_order_id: coid,
                        expires_at: exp.map_or(0, |e| now + e),
                        quote_budget: budget,
                        match_limit: ml,
                        reverse_spread: rev,
                    };
                    // Hosts skip the clock when nothing can expire; results must not change.
                    let eng_now = if m.needs_clock(p.expires_at != 0) { now } else { 0 };
                    let mut a = m.place_order(seats[t], &p, eng_now, &mut NoopSink);
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
                    let mine: Vec<u64> = md.orders.iter().filter(|o| o.trader == t).map(|o| o.seq)
                        .chain(md.stops.iter().filter(|s| s.trader == t).map(|s| s.seq)).collect();
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
                    let b = if mine.contains(&id) {
                        if md.orders.iter().any(|o| o.seq == id) { md.remove(id); } else { md.remove_stop(id); }
                        stale.push(eid);
                        Ok(())
                    } else { Err(EngineError::OrderNotFound) };
                    prop_assert_eq!(a, b);
                }
                Op::CancelCoid { t, coid } => {
                    let ids: Vec<u64> = md.orders.iter().filter(|o| o.trader == t && o.coid == coid).map(|o| o.seq).collect();
                    for id in &ids { md.remove(*id); }
                    let sids: Vec<u64> = md.stops.iter().filter(|s| s.trader == t && s.coid == coid).map(|s| s.seq).collect();
                    for id in &sids { md.remove_stop(*id); }
                    prop_assert_eq!(m.cancel_by_client_id(seats[t], coid, &mut NoopSink), Ok((ids.len() + sids.len()) as u32));
                }
                Op::CancelAll { t, side } => {
                    let side = side.map(|b| if b { Side::Bid } else { Side::Ask });
                    let ids: Vec<u64> = md.orders.iter().filter(|o| o.trader == t && side.is_none_or(|s| s == o.side)).map(|o| o.seq).collect();
                    for id in &ids { md.remove(*id); }
                    let sids: Vec<u64> = md.stops.iter().filter(|o| o.trader == t && side.is_none_or(|s| s == o.side)).map(|o| o.seq).collect();
                    for id in &sids { md.remove_stop(*id); }
                    prop_assert_eq!(m.cancel_all(seats[t], side, &mut NoopSink), Ok((ids.len() + sids.len()) as u32));
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
                    let eng_now = if m.needs_clock(false) { now } else { 0 };
                    let a = m.swap(&sp, eng_now, &mut NoopSink);
                    let b = md.swap(&sp, now);
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
                Op::PlaceStop { t, side, price, lots, ty, stp, exp, ml, coid, trigger, above, oco, bad } => {
                    let side = if side { Side::Bid } else { Side::Ask };
                    let oco_with = oco.and_then(|k| md.stops.iter().filter(|s| s.trader == t).nth(k)).map_or(0, |s| ids[&s.seq]);
                    let mut order = OrderParams {
                        side, price, base_lots: lots,
                        order_type: OrderType::from_u8(ty).unwrap(),
                        self_trade: SelfTradeBehavior::from_u8(stp).unwrap(),
                        client_order_id: coid,
                        expires_at: exp.map_or(0, |e| now + e),
                        quote_budget: None,
                        match_limit: ml,
                        reverse_spread: 0,
                    };
                    match bad { 1 => order.quote_budget = Some(5), 2 => order.reverse_spread = 7, _ => {} }
                    let sp = StopParams {
                        trigger_price: trigger,
                        direction: if above { TriggerDirection::Above } else { TriggerDirection::Below },
                        order,
                        oco_with,
                    };
                    let mut msp = sp;
                    msp.oco_with = oco.and_then(|k| md.stops.iter().filter(|s| s.trader == t).nth(k)).map_or(0, |s| s.seq);
                    let eng_now = if m.needs_clock(order.expires_at != 0) { now } else { 0 };
                    let a = m.place_stop(seats[t], &sp, eng_now);
                    let b = md.place_stop(t, &msp, now);
                    let a = a.map(|id| { let (seq, _) = split_order_id(id); ids.insert(seq, id); seq });
                    prop_assert_eq!(a, b, "stop {:?}", sp);
                }
                Op::TriggerStops(limit) => {
                    let eng_now = if m.needs_clock(false) { now } else { 0 };
                    let a = m.trigger_stops(eng_now, limit, &mut NoopSink);
                    let b = md.run_stops(now, limit);
                    prop_assert_eq!(a, b);
                }
            }
            // Learn the ids of orders the engine placed on its own (reverse orders).
            for side in [Side::Bid, Side::Ask] {
                let mut i = if side == Side::Bid { m.best_bid() } else { m.best_ask() };
                while i != NIL {
                    ids.entry(m.order(i).unwrap().seq).or_insert(order_id(m.order(i).unwrap().seq, i));
                    i = m.next_order(side, i).unwrap();
                }
            }
            let totals = m.check_invariants();
            prop_assert!(totals.is_ok(), "invariants: {:?}", totals);
            let totals = totals.unwrap();
            prop_assert_eq!(totals.base_atoms, vault_base);
            prop_assert_eq!(totals.quote_atoms, vault_quote);
            prop_assert_eq!(snapshot(&m, &seats, now), model_snapshot(&md));
            prop_assert_eq!(m.header().last_price, md.last);
            let mut es = vec![];
            for (t, &seat) in seats.iter().enumerate() {
                m.for_each_trader_stop(seat, |_, s| {
                    let partner = (s.partner() != NIL).then(|| m.stop(s.partner()).unwrap().seq());
                    es.push((t, s.seq(), s.trigger_price(), s.base_lots, partner));
                }).unwrap();
            }
            es.sort_by_key(|e| e.1);
            let ms: Vec<_> = md.stops.iter().map(|s| (s.trader, s.seq, s.trigger, s.lots, s.partner)).collect();
            prop_assert_eq!(es, ms);
            // Live book is never crossed.
            let mut bb = None; let mut ba = None;
            m.for_each_order(Side::Bid, now, |o| { bb = Some(o.price); false }).unwrap();
            m.for_each_order(Side::Ask, now, |o| { ba = Some(o.price); false }).unwrap();
            if let (Some(b), Some(a)) = (bb, ba) { prop_assert!(b < a, "crossed {} {}", b, a); }
        }
    }
}
