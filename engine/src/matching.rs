//! Order placement, matching, cancels and swaps.
//!
//! Every public operation validates first and mutates second, so an error
//! leaves the market untouched.

use bytemuck::cast_mut;

use crate::events::{EventSink, FillEvent, OrderOutEvent, OutReason, PlacedEvent};
use crate::market::{order_key, Market, TreeId};
use crate::node::{
    order_id, split_order_id, Node, NodeTag, OrderLeaf, FLAG_ASK, MAX_ORDER_SEQ, NIL, ORDER_INDEX_BITS,
};
use crate::rbtree::{EDGE_MAX, EDGE_MIN};
use crate::{
    EngineError, OrderParams, OrderType, PlaceOutcome, Result, SelfTradeBehavior, Side,
    SwapOutcome, SwapParams,
};

const BPS: u128 = 10_000;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Taker {
    Seat(u32),
    Anonymous,
}

#[derive(Default)]
struct MatchTotals {
    base_lots: u64,
    quote_atoms: u64,
    fee_atoms: u64,
    touched: u32,
    /// Taker lots neither filled nor removed by self-trade decrements.
    remaining: u64,
}

fn add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b).ok_or(EngineError::Overflow)
}

fn sub(a: u64, b: u64) -> Result<u64> {
    a.checked_sub(b).ok_or(EngineError::Corrupted)
}

#[inline(always)]
fn wants_placed<S: EventSink>(_: &S) -> bool {
    S::PLACED
}

#[inline(always)]
fn wants_fills<S: EventSink>(_: &S) -> bool {
    S::FILLS
}

fn side_of(o: &OrderLeaf) -> Side {
    if o.is_ask() {
        Side::Ask
    } else {
        Side::Bid
    }
}

const fn crosses(taker: Side, maker_price: u64, limit: u64) -> bool {
    match taker {
        Side::Bid => maker_price <= limit,
        Side::Ask => maker_price >= limit,
    }
}

impl Market<'_> {
    /// Taker fee for `quote` atoms, rounded up.
    #[inline]
    pub fn taker_fee(&self, quote: u64) -> u64 {
        let bps = u64::from(self.header.taker_fee_bps);
        match crate::math::mul(quote, bps) {
            Some(v) => v.div_ceil(BPS as u64),
            None => (u128::from(quote) * u128::from(bps)).div_ceil(BPS) as u64,
        }
    }

    /// Maker rebate for `quote` atoms, rounded down (never above the taker fee).
    #[inline]
    pub fn maker_rebate(&self, quote: u64) -> u64 {
        let bps = u64::from(self.header.maker_rebate_bps);
        match crate::math::mul(quote, bps) {
            Some(v) => v / BPS as u64,
            None => (u128::from(quote) * u128::from(bps) / BPS) as u64,
        }
    }

    /// Largest lot count whose cost plus taker fee fits in `budget`.
    fn affordable_lots(&self, budget: u64, price: u64) -> Result<u64> {
        let unit = u128::from(price) * u128::from(self.header.quote_lot_size);
        if unit == 0 {
            return Err(EngineError::InvalidPrice);
        }
        let denom = unit * (BPS + u128::from(self.header.taker_fee_bps));
        let mut l = u128::from(budget) * BPS / denom;
        let cost = |l: u128| -> u128 {
            let q = l * unit;
            q + (q * u128::from(self.header.taker_fee_bps)).div_ceil(BPS)
        };
        while l > 0 && cost(l) > u128::from(budget) {
            l -= 1;
        }
        while cost(l + 1) <= u128::from(budget) {
            l += 1;
        }
        Ok(u64::try_from(l).unwrap_or(u64::MAX))
    }

    fn best_of(&self, side: Side) -> u32 {
        match side {
            Side::Bid => self.header.best_bid,
            Side::Ask => self.header.best_ask,
        }
    }

    /// Locks the funds of a resting order, inserts it and returns its id.
    #[allow(clippy::too_many_arguments)]
    fn insert_order(
        &mut self,
        trader: u32,
        side: Side,
        price: u64,
        base_lots: u64,
        client_order_id: u64,
        expires_at: u64,
        sink: &mut impl EventSink,
    ) -> Result<u64> {
        let seq = self
            .header
            .seq_num
            .checked_add(1)
            .ok_or(EngineError::Overflow)?;
        if seq > MAX_ORDER_SEQ {
            return Err(EngineError::Overflow);
        }
        let key = order_key(side, price, seq);
        let i = self.alloc()?;
        if i >> ORDER_INDEX_BITS != 0 {
            return Err(EngineError::OutOfSpace);
        }
        // Lock the order's funds and link it at the head of the trader's list.
        let lock_base = side == Side::Ask;
        let amount = if lock_base {
            self.base_for(base_lots)?
        } else {
            self.quote_for(base_lots, price)?
        };
        let head = {
            let t = self.trader_mut(trader)?;
            if lock_base {
                t.base_free = t
                    .base_free
                    .checked_sub(amount)
                    .ok_or(EngineError::InsufficientFunds)?;
            } else {
                t.quote_free = t
                    .quote_free
                    .checked_sub(amount)
                    .ok_or(EngineError::InsufficientFunds)?;
            }
            let head = t.order_head;
            t.order_head = i;
            head
        };
        let (key_lo, key_hi) = (key as u64, (key >> 64) as u64);
        *cast_mut::<Node, OrderLeaf>(self.node_mut(i)?) = OrderLeaf {
            tag: NodeTag::Order as u32,
            flags: if side == Side::Ask { FLAG_ASK } else { 0 },
            left: NIL,
            right: NIL,
            parent: NIL,
            extra: 0,
            key_lo,
            key_hi,
            trader,
            prev: NIL,
            next: head,
            kind: 0,
            base_lots,
            client_order_id,
            expires_at,
            seq,
        };
        // Asks sort best-first (best = minimum), bids worst-first.
        let h = &*self.header;
        let (lo, hi) = match side {
            Side::Ask => (h.best_ask, h.worst_ask),
            Side::Bid => (h.worst_bid, h.best_bid),
        };
        let edges = self.tree(TreeId::of(side)).insert_with_edges(i, lo, hi)?;
        if head != NIL {
            self.order_mut(head)?.prev = i;
        }
        let h = &mut *self.header;
        match side {
            Side::Ask => {
                if edges & EDGE_MIN != 0 {
                    h.best_ask = i;
                }
                if edges & EDGE_MAX != 0 {
                    h.worst_ask = i;
                }
            }
            Side::Bid => {
                if edges & EDGE_MIN != 0 {
                    h.worst_bid = i;
                }
                if edges & EDGE_MAX != 0 {
                    h.best_bid = i;
                }
            }
        }
        self.header.seq_num = seq;
        self.header.order_count = self.header.order_count.saturating_add(1);
        let id = order_id(seq, i);
        if wants_placed(sink) {
            let owner = self.trader(trader)?.owner;
            sink.on_placed(&PlacedEvent {
                owner,
                order_id: id,
                client_order_id,
                side,
                price,
                base_lots,
                expires_at,
            });
        }
        Ok(id)
    }

    /// Removes order `i`, returns its remaining funds to the owner's free
    /// balance and frees the node.
    fn remove_order(&mut self, i: u32, reason: OutReason, sink: &mut impl EventSink) -> Result<()> {
        let (side, prev, next, trader, price, lots) = {
            let o = self.order(i)?;
            (side_of(o), o.prev, o.next, o.trader, o.price(), o.base_lots)
        };
        let (best, worst) = match side {
            Side::Ask => (self.header.best_ask, self.header.worst_ask),
            Side::Bid => (self.header.best_bid, self.header.worst_bid),
        };
        if best == i {
            let nb = self.next_order(side, i)?;
            match side {
                Side::Ask => self.header.best_ask = nb,
                Side::Bid => self.header.best_bid = nb,
            }
        }
        if worst == i {
            // The next worst order is the neighbor on the better side.
            let nw = match side {
                Side::Ask => crate::rbtree::prev(self.nodes, i)?,
                Side::Bid => crate::rbtree::next(self.nodes, i)?,
            };
            match side {
                Side::Ask => self.header.worst_ask = nw,
                Side::Bid => self.header.worst_bid = nw,
            }
        }
        self.tree(TreeId::of(side)).remove(i)?;
        if prev != NIL {
            self.order_mut(prev)?.next = next;
        } else {
            self.trader_mut(trader)?.order_head = next;
        }
        if next != NIL {
            self.order_mut(next)?.prev = prev;
        }
        self.unlock(trader, side, price, lots)?;
        if sink.wants_out(reason) {
            let (seq, client_order_id) = {
                let o = self.order(i)?;
                (o.seq, o.client_order_id)
            };
            let owner = self.trader(trader)?.owner;
            sink.on_out(&OrderOutEvent {
                owner,
                order_id: order_id(seq, i),
                client_order_id,
                side,
                price,
                base_lots: lots,
                reason,
            });
        }
        self.release(i)?;
        self.header.order_count = self.header.order_count.saturating_sub(1);
        Ok(())
    }

    fn unlock(&mut self, trader: u32, side: Side, price: u64, lots: u64) -> Result<()> {
        if lots == 0 {
            return Ok(());
        }
        match side {
            Side::Ask => {
                let b = self.base_for(lots)?;
                let t = self.trader_mut(trader)?;
                t.base_free = add(t.base_free, b)?;
            }
            Side::Bid => {
                let q = self.quote_for(lots, price)?;
                let t = self.trader_mut(trader)?;
                t.quote_free = add(t.quote_free, q)?;
            }
        }
        Ok(())
    }

    /// Read-only walk of what a taker would meet. Returns the lots it could
    /// fill, or an error for an aborting self-trade.
    #[allow(clippy::too_many_arguments)]
    fn prescan(
        &self,
        taker: Taker,
        side: Side,
        limit: u64,
        lots: u64,
        stp: SelfTradeBehavior,
        match_limit: u32,
        now: u64,
    ) -> Result<u64> {
        let book = side.opposite();
        let mut i = self.best_of(book);
        let mut need = lots;
        let mut fillable = 0u64;
        let mut touched = 0u32;
        while i != NIL && need > 0 && touched < match_limit {
            let o = self.order(i)?;
            if !crosses(side, o.price(), limit) {
                break;
            }
            touched += 1;
            if !o.is_expired(now) {
                if taker == Taker::Seat(o.trader) {
                    match stp {
                        SelfTradeBehavior::Abort => return Err(EngineError::SelfTrade),
                        SelfTradeBehavior::CancelProvide => {}
                        SelfTradeBehavior::DecrementTake => {
                            need = need.saturating_sub(o.base_lots);
                        }
                    }
                } else {
                    let f = need.min(o.base_lots);
                    need -= f;
                    fillable += f;
                }
            }
            i = self.next_order(book, i)?;
        }
        Ok(fillable)
    }

    /// Core matching loop. The caller settles the taker's side.
    #[allow(clippy::too_many_arguments)]
    fn match_book(
        &mut self,
        taker: Taker,
        side: Side,
        limit: u64,
        mut lots: u64,
        mut budget: Option<u64>,
        stp: SelfTradeBehavior,
        match_limit: u32,
        now: u64,
        sink: &mut impl EventSink,
    ) -> Result<MatchTotals> {
        let book = side.opposite();
        let mut t = MatchTotals::default();
        let taker_owner = match taker {
            Taker::Seat(s) if wants_fills(sink) => Some(self.trader(s)?.owner),
            _ => None,
        };
        while lots > 0 && t.touched < match_limit {
            let m = self.best_of(book);
            if m == NIL {
                break;
            }
            let (price, maker_lots, maker, expired) = {
                let o = self.order(m)?;
                (o.price(), o.base_lots, o.trader, o.is_expired(now))
            };
            if !crosses(side, price, limit) {
                break;
            }
            if expired {
                t.touched += 1;
                self.remove_order(m, OutReason::Expired, sink)?;
                continue;
            }
            if taker == Taker::Seat(maker) {
                t.touched += 1;
                match stp {
                    SelfTradeBehavior::Abort => return Err(EngineError::SelfTrade),
                    SelfTradeBehavior::CancelProvide => {
                        self.remove_order(m, OutReason::SelfTrade, sink)?;
                    }
                    SelfTradeBehavior::DecrementTake => {
                        let d = lots.min(maker_lots);
                        lots -= d;
                        self.reduce_resting(m, maker_lots - d, OutReason::SelfTrade, sink)?;
                    }
                }
                continue;
            }
            let mut fill = lots.min(maker_lots);
            let mut quote = self.quote_for(fill, price).ok();
            let mut fee = quote.map_or(0, |q| self.taker_fee(q));
            if let Some(b) = budget {
                // Fast path: the whole fill fits the budget.
                let fits = quote
                    .and_then(|q| q.checked_add(fee))
                    .is_some_and(|c| c <= b);
                if !fits {
                    fill = fill.min(self.affordable_lots(b, price)?);
                    if fill == 0 {
                        break;
                    }
                    let q = self.quote_for(fill, price)?;
                    quote = Some(q);
                    fee = self.taker_fee(q);
                }
            }
            let quote = quote.ok_or(EngineError::Overflow)?;
            t.touched += 1;
            let rebate = self.maker_rebate(quote);
            {
                let base = self.base_for(fill)?;
                let mt = self.trader_mut(maker)?;
                match book {
                    Side::Ask => mt.quote_free = add(mt.quote_free, add(quote, rebate)?)?,
                    Side::Bid => {
                        mt.base_free = add(mt.base_free, base)?;
                        mt.quote_free = add(mt.quote_free, rebate)?;
                    }
                }
            }
            let h = &mut *self.header;
            h.fees_accrued = add(h.fees_accrued, sub(fee, rebate)?)?;
            h.total_base_lots_traded = h.total_base_lots_traded.saturating_add(fill);
            h.total_quote_atoms_traded = h.total_quote_atoms_traded.saturating_add(quote);
            let remaining = maker_lots - fill;
            let (seq, client_order_id) = {
                let o = self.order_mut(m)?;
                o.base_lots = remaining;
                (o.seq, o.client_order_id)
            };
            if wants_fills(sink) {
                let maker_owner = self.trader(maker)?.owner;
                sink.on_fill(&FillEvent {
                    maker: maker_owner,
                    maker_order_id: order_id(seq, m),
                    maker_client_order_id: client_order_id,
                    taker: taker_owner,
                    taker_side: side,
                    price,
                    base_lots: fill,
                    quote_atoms: quote,
                    taker_fee: fee,
                    maker_rebate: rebate,
                    maker_remaining: remaining,
                });
            }
            if remaining == 0 {
                self.remove_order(m, OutReason::Filled, sink)?;
            }
            lots -= fill;
            t.base_lots = add(t.base_lots, fill)?;
            t.quote_atoms = add(t.quote_atoms, quote)?;
            t.fee_atoms = add(t.fee_atoms, fee)?;
            if let Some(b) = budget.as_mut() {
                *b = sub(*b, add(quote, fee)?)?;
            }
        }
        t.remaining = lots;
        Ok(t)
    }

    /// Shrinks a resting order to `new_lots` (removing it at zero) and
    /// unlocks the difference. Keeps time priority.
    fn reduce_resting(
        &mut self,
        i: u32,
        new_lots: u64,
        reason: OutReason,
        sink: &mut impl EventSink,
    ) -> Result<()> {
        let o = *self.order(i)?;
        if new_lots >= o.base_lots {
            return Err(EngineError::InvalidQuantity);
        }
        if new_lots == 0 {
            return self.remove_order(i, reason, sink);
        }
        self.unlock(o.trader, side_of(&o), o.price(), o.base_lots - new_lots)?;
        self.order_mut(i)?.base_lots = new_lots;
        Ok(())
    }

    /// Places an order for seat `trader` at unix time `now`.
    pub fn place_order(
        &mut self,
        trader: u32,
        p: &OrderParams,
        now: u64,
        sink: &mut impl EventSink,
    ) -> Result<PlaceOutcome> {
        self.require_active()?;
        if p.price == 0 {
            return Err(EngineError::InvalidPrice);
        }
        if p.base_lots == 0 || p.base_lots < self.header.min_base_lots {
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
        let (base_free, quote_free) = {
            let t = self.trader(trader)?;
            (t.base_free, t.quote_free)
        };
        let match_limit = p.match_limit.max(1);
        // Whether the order reaches the opposite best at its limit price.
        // Most orders do not: they cannot match, slide or be blocked.
        let reaches = {
            let b = self.best_of(p.side.opposite());
            b != NIL && crosses(p.side, self.order(b)?.price(), p.price)
        };

        // Funds: enough for the whole order at the limit price, plus
        // worst-case fees when it can take liquidity.
        let budget = match p.side {
            Side::Ask => {
                if base_free < self.base_for(p.base_lots)? {
                    return Err(EngineError::InsufficientFunds);
                }
                None
            }
            Side::Bid => {
                if let Some(b) = p.quote_budget {
                    if quote_free < b {
                        return Err(EngineError::InsufficientFunds);
                    }
                    Some(b)
                } else {
                    let notional = self.quote_for(p.base_lots, p.price)?;
                    let need = if reaches && p.order_type.takes() {
                        let slack = u64::from(match_limit).min(p.base_lots);
                        add(add(notional, self.taker_fee(notional))?, slack)?
                    } else {
                        notional
                    };
                    if quote_free < need {
                        return Err(EngineError::InsufficientFunds);
                    }
                    Some(quote_free)
                }
            }
        };
        if p.order_type.rests() {
            self.require_space(1)?;
        }
        if !reaches {
            if p.order_type == OrderType::FillOrKill {
                return Err(EngineError::FillOrKillNotFilled);
            }
            let mut out = PlaceOutcome::default();
            if p.order_type.rests() {
                let id = self.insert_order(
                    trader,
                    p.side,
                    p.price,
                    p.base_lots,
                    p.client_order_id,
                    p.expires_at,
                    sink,
                )?;
                out.order_id = Some(id);
                out.rested_price = p.price;
                out.rested_base_lots = p.base_lots;
            }
            return Ok(out);
        }

        // Resolve post-only pricing against the first live opposite order.
        let mut price = p.price;
        if !p.order_type.takes() {
            let book = p.side.opposite();
            let mut i = self.best_of(book);
            while i != NIL && self.order(i)?.is_expired(now) {
                i = self.next_order(book, i)?;
            }
            if i != NIL {
                let opp = self.order(i)?.price();
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

        let taker = Taker::Seat(trader);
        if p.order_type == OrderType::FillOrKill || p.self_trade == SelfTradeBehavior::Abort {
            let fillable = self.prescan(
                taker,
                p.side,
                price,
                p.base_lots,
                p.self_trade,
                match_limit,
                now,
            )?;
            if p.order_type == OrderType::FillOrKill && fillable < p.base_lots {
                return Err(EngineError::FillOrKillNotFilled);
            }
        }
        let mut out = PlaceOutcome::default();
        let mut remaining = p.base_lots;
        // A post-only order may have slid off the opposite best.
        let reaches = {
            let b = self.best_of(p.side.opposite());
            b != NIL && crosses(p.side, self.order(b)?.price(), price)
        };
        if reaches && p.order_type.takes() {
            let m = self.match_book(
                taker,
                p.side,
                price,
                p.base_lots,
                budget,
                p.self_trade,
                match_limit,
                now,
                sink,
            )?;
            let base = self.base_for(m.base_lots)?;
            let tr = self.trader_mut(trader)?;
            match p.side {
                Side::Bid => {
                    tr.quote_free = sub(tr.quote_free, add(m.quote_atoms, m.fee_atoms)?)?;
                    tr.base_free = add(tr.base_free, base)?;
                }
                Side::Ask => {
                    tr.base_free = sub(tr.base_free, base)?;
                    tr.quote_free = add(tr.quote_free, sub(m.quote_atoms, m.fee_atoms)?)?;
                }
            }
            out.filled_base_lots = m.base_lots;
            out.filled_quote_atoms = m.quote_atoms;
            out.fee_atoms = m.fee_atoms;
            out.makers_touched = m.touched;
            remaining = m.remaining;
        }
        if p.order_type.rests()
            && remaining > 0
            && (!reaches || self.can_rest(p.side, price, now)?)
        {
            let id = self.insert_order(
                trader,
                p.side,
                price,
                remaining,
                p.client_order_id,
                p.expires_at,
                sink,
            )?;
            out.order_id = Some(id);
            out.rested_price = price;
            out.rested_base_lots = remaining;
        }
        Ok(out)
    }

    /// An order may rest only if it no longer crosses a live opposite order
    /// (the match limit can stop matching early).
    fn can_rest(&self, side: Side, price: u64, now: u64) -> Result<bool> {
        let book = side.opposite();
        let mut i = self.best_of(book);
        while i != NIL {
            let o = self.order(i)?;
            if !crosses(side, o.price(), price) {
                return Ok(true);
            }
            if !o.is_expired(now) {
                return Ok(false);
            }
            i = self.next_order(book, i)?;
        }
        Ok(true)
    }

    /// Finds a trader's order node by id.
    pub fn find_order(&self, trader: u32, order_id: u64) -> Result<u32> {
        let (seq, i) = split_order_id(order_id);
        match self.nodes.get(i as usize) {
            Some(n) if n.is(NodeTag::Order) => {
                let o: &OrderLeaf = bytemuck::cast_ref(n);
                if o.seq == seq && o.trader == trader {
                    return Ok(i);
                }
                Err(EngineError::OrderNotFound)
            }
            _ => Err(EngineError::OrderNotFound),
        }
    }

    /// Cancels one order by id.
    pub fn cancel_order(
        &mut self,
        trader: u32,
        order_id: u64,
        sink: &mut impl EventSink,
    ) -> Result<()> {
        self.require_cancellable()?;
        let i = self.find_order(trader, order_id)?;
        self.remove_order(i, OutReason::Cancelled, sink)
    }

    /// Cancels every order of `trader` carrying `client_order_id`. Returns the count.
    pub fn cancel_by_client_id(
        &mut self,
        trader: u32,
        client_order_id: u64,
        sink: &mut impl EventSink,
    ) -> Result<u32> {
        self.cancel_where(trader, sink, |o| o.client_order_id == client_order_id)
    }

    /// Cancels every order of `trader`, optionally on one side. Returns the count.
    pub fn cancel_all(
        &mut self,
        trader: u32,
        side: Option<Side>,
        sink: &mut impl EventSink,
    ) -> Result<u32> {
        self.cancel_where(trader, sink, |o| side.is_none_or(|s| side_of(o) == s))
    }

    fn cancel_where(
        &mut self,
        trader: u32,
        sink: &mut impl EventSink,
        pred: impl Fn(&OrderLeaf) -> bool,
    ) -> Result<u32> {
        self.require_cancellable()?;
        let mut i = self.trader(trader)?.order_head;
        let mut n = 0u32;
        while i != NIL {
            let o = *self.order(i)?;
            if pred(&o) {
                self.remove_order(i, OutReason::Cancelled, sink)?;
                n += 1;
            }
            i = o.next;
        }
        Ok(n)
    }

    /// Reduces an order's size in place, keeping its queue position.
    pub fn reduce_order(
        &mut self,
        trader: u32,
        order_id: u64,
        new_base_lots: u64,
        sink: &mut impl EventSink,
    ) -> Result<()> {
        self.require_cancellable()?;
        let i = self.find_order(trader, order_id)?;
        self.reduce_resting(i, new_base_lots, OutReason::Cancelled, sink)
    }

    /// Removes up to `limit` expired orders from the top of both sides.
    /// Permissionless housekeeping. Returns the number removed.
    pub fn prune_expired(
        &mut self,
        now: u64,
        limit: u32,
        sink: &mut impl EventSink,
    ) -> Result<u32> {
        let mut n = 0;
        for side in [Side::Bid, Side::Ask] {
            let mut i = self.best_of(side);
            while i != NIL && n < limit {
                let next = self.next_order(side, i)?;
                if self.order(i)?.is_expired(now) {
                    self.remove_order(i, OutReason::Expired, sink)?;
                    n += 1;
                }
                i = next;
            }
        }
        Ok(n)
    }

    /// Executes a seatless taker trade. The host must collect
    /// [`SwapOutcome::quote_net`] (bid) or `base_atoms` (ask) from the user
    /// and pay out the other leg.
    pub fn swap(
        &mut self,
        p: &SwapParams,
        now: u64,
        sink: &mut impl EventSink,
    ) -> Result<SwapOutcome> {
        self.require_active()?;
        if p.limit_price == 0 {
            return Err(EngineError::InvalidPrice);
        }
        if p.base_lots == 0 {
            return Err(EngineError::InvalidQuantity);
        }
        let budget = (p.side == Side::Bid).then_some(p.quote_budget);
        let m = self.match_book(
            Taker::Anonymous,
            p.side,
            p.limit_price,
            p.base_lots,
            budget,
            SelfTradeBehavior::CancelProvide,
            p.match_limit.max(1),
            now,
            sink,
        )?;
        Ok(SwapOutcome {
            base_lots: m.base_lots,
            base_atoms: self.base_for(m.base_lots)?,
            quote_atoms: m.quote_atoms,
            fee_atoms: m.fee_atoms,
            makers_touched: m.touched,
        })
    }
}
