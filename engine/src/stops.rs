//! Stops: orders that wait for the last trade price to reach a trigger.
//!
//! No keeper is needed. Every trade ends by placing the stops it has set off
//! (up to [`crate::AUTO_TRIGGER_LIMIT`]), and anyone can place more with
//! [`Market::trigger_stops`]. A stop's funds are locked from the start, so
//! its order never fails for lack of them.

use bytemuck::cast_mut;

use crate::events::{EventSink, OrderOutEvent, OutReason};
use crate::market::{Market, TreeId};
use crate::node::{
    order_id, split_order_id, Node, NodeTag, StopLeaf, FLAG_ABOVE, FLAG_ASK, MAX_ORDER_SEQ, NIL,
    ORDER_INDEX_BITS,
};
use crate::{
    EngineError, OrderParams, OrderType, Result, SelfTradeBehavior, Side, StopParams,
    TriggerDirection, MAX_STOP_MATCH_LIMIT,
};

/// Whether a trade at `last` sets off a stop at `trigger`.
fn met(direction: TriggerDirection, trigger: u64, last: u64) -> bool {
    last != 0
        && match direction {
            TriggerDirection::Above => last >= trigger,
            TriggerDirection::Below => last <= trigger,
        }
}

impl Market<'_> {
    /// Atoms a stop locks: base for a sell, and for a buy the quote its
    /// order may need to take liquidity, fees and rounding included.
    pub(crate) fn stop_lock(
        &self,
        side: Side,
        price: u64,
        lots: u64,
        match_limit: u32,
    ) -> Result<u64> {
        match side {
            Side::Ask => self.base_for(lots),
            Side::Bid => {
                let notional = self.quote_for(lots, price)?;
                let slack = u64::from(match_limit.max(1)).min(lots);
                notional
                    .checked_add(self.taker_fee(notional))
                    .and_then(|v| v.checked_add(slack))
                    .ok_or(EngineError::Overflow)
            }
        }
    }

    /// Places a stop for seat `trader` at unix time `now` and returns its id.
    /// Ids of stops and orders never collide, and [`Market::cancel_order`],
    /// [`Market::cancel_all`] and [`Market::cancel_by_client_id`] cancel
    /// stops too.
    pub fn place_stop(&mut self, trader: u32, p: &StopParams, now: u64) -> Result<u64> {
        self.require_active()?;
        let o = &p.order;
        if o.price == 0 {
            return Err(EngineError::InvalidPrice);
        }
        if o.base_lots == 0 || o.base_lots < self.header.min_base_lots {
            return Err(EngineError::InvalidQuantity);
        }
        if o.expires_at != 0 && o.expires_at <= now {
            return Err(EngineError::AlreadyExpired);
        }
        if p.trigger_price == 0 || o.quote_budget.is_some() || o.reverse_spread != 0 {
            return Err(EngineError::InvalidStop);
        }
        if met(p.direction, p.trigger_price, self.header.last_price) {
            return Err(EngineError::StopWouldTrigger);
        }
        let partner = if p.oco_with == 0 {
            NIL
        } else {
            let j = self
                .find_stop(trader, p.oco_with)
                .map_err(|_| EngineError::InvalidStop)?;
            if self.stop(j)?.partner() != NIL {
                return Err(EngineError::InvalidStop);
            }
            j
        };
        let match_limit = o.match_limit.clamp(1, MAX_STOP_MATCH_LIMIT);
        let amount = self.stop_lock(o.side, o.price, o.base_lots, match_limit)?;
        {
            let t = self.trader(trader)?;
            let free = match o.side {
                Side::Ask => t.base_free,
                Side::Bid => t.quote_free,
            };
            if free < amount {
                return Err(EngineError::InsufficientFunds);
            }
        }
        let seq = self
            .header
            .seq_num
            .checked_add(1)
            .filter(|&s| s <= MAX_ORDER_SEQ)
            .ok_or(EngineError::Overflow)?;
        let i = self.alloc()?;
        if i >> ORDER_INDEX_BITS != 0 {
            return Err(EngineError::OutOfSpace);
        }

        let head = {
            let t = self.trader_mut(trader)?;
            match o.side {
                Side::Ask => t.base_free -= amount,
                Side::Bid => t.quote_free -= amount,
            }
            let head = t.order_head;
            t.order_head = i;
            head
        };
        let above = p.direction == TriggerDirection::Above;
        let mut flags = if above { FLAG_ABOVE } else { 0 };
        if o.side == Side::Ask {
            flags |= FLAG_ASK;
        }
        *cast_mut::<Node, StopLeaf>(self.node_mut(i)?) = StopLeaf {
            tag: NodeTag::Stop as u32,
            flags,
            left: NIL,
            right: NIL,
            parent: NIL,
            extra: partner.wrapping_add(1),
            key_lo: if above { seq } else { !seq },
            key_hi: p.trigger_price,
            trader,
            prev: NIL,
            next: head,
            order_type: o.order_type as u8,
            self_trade: o.self_trade as u8,
            match_limit: match_limit as u16,
            price: o.price,
            base_lots: o.base_lots,
            client_order_id: o.client_order_id,
            expires_at: o.expires_at,
        };
        if head != NIL {
            self.listed_mut(head)?.prev = i;
        }
        if partner != NIL {
            self.stop_mut(partner)?.extra = i.wrapping_add(1);
        }
        let h = &mut *self.header;
        if above {
            h.stop_above_min = h.stop_above_min.min(p.trigger_price);
            self.tree(TreeId::StopsAbove).insert(i)?;
        } else {
            h.stop_below_max = h.stop_below_max.max(p.trigger_price);
            self.tree(TreeId::StopsBelow).insert(i)?;
        }
        let h = &mut *self.header;
        h.seq_num = seq;
        h.stop_count += 1;
        if o.expires_at != 0 {
            h.expiring_orders = h.expiring_orders.saturating_add(1);
        }
        Ok(order_id(seq, i))
    }

    /// Finds a trader's stop node by id.
    pub fn find_stop(&self, trader: u32, id: u64) -> Result<u32> {
        let (seq, i) = split_order_id(id);
        match self.stop(i) {
            Ok(s) if s.seq() == seq && s.trader == trader => Ok(i),
            _ => Err(EngineError::OrderNotFound),
        }
    }

    /// Removes stop `i`, returns its funds to the owner and frees the node.
    /// Its OCO partner, if any, stays as an unpaired stop.
    pub(crate) fn remove_stop(
        &mut self,
        i: u32,
        reason: OutReason,
        sink: &mut impl EventSink,
    ) -> Result<()> {
        let s = *self.stop(i)?;
        let side = if s.is_ask() { Side::Ask } else { Side::Bid };
        let above = s.is_above();
        if s.partner() != NIL {
            self.stop_mut(s.partner())?.extra = 0;
        }
        self.tree(if above {
            TreeId::StopsAbove
        } else {
            TreeId::StopsBelow
        })
        .remove(i)?;
        self.unlink_listed(s.trader, s.prev, s.next)?;
        let amount = self.stop_lock(side, s.price, s.base_lots, u32::from(s.match_limit))?;
        let t = self.trader_mut(s.trader)?;
        match side {
            Side::Ask => {
                t.base_free = t
                    .base_free
                    .checked_add(amount)
                    .ok_or(EngineError::Overflow)?
            }
            Side::Bid => {
                t.quote_free = t
                    .quote_free
                    .checked_add(amount)
                    .ok_or(EngineError::Overflow)?;
            }
        }
        if sink.wants_out(reason) {
            let owner = self.trader(s.trader)?.owner;
            sink.on_out(&OrderOutEvent {
                owner,
                order_id: order_id(s.seq(), i),
                client_order_id: s.client_order_id,
                side,
                price: s.price,
                base_lots: s.base_lots,
                reason,
            });
        }
        self.release(i)?;
        let h = &mut *self.header;
        h.stop_count -= 1;
        if s.expires_at != 0 {
            h.expiring_orders = h.expiring_orders.saturating_sub(1);
        }
        // Refresh the cached trigger if this stop was the first in line.
        if above && h.stop_above_min == s.trigger_price() {
            let j = crate::rbtree::min(self.nodes, self.header.stops_above_root)?;
            self.header.stop_above_min = if j == NIL {
                u64::MAX
            } else {
                self.stop(j)?.trigger_price()
            };
        } else if !above && h.stop_below_max == s.trigger_price() {
            let j = crate::rbtree::max(self.nodes, self.header.stops_below_root)?;
            self.header.stop_below_max = if j == NIL {
                0
            } else {
                self.stop(j)?.trigger_price()
            };
        }
        Ok(())
    }

    /// Places up to `limit` stops whose trigger the last trade price has
    /// reached, first in line first. Permissionless: anyone may call it, and
    /// every trade calls it for [`crate::AUTO_TRIGGER_LIMIT`] stops. Returns how
    /// many stops were handled (placed, or removed if they had expired).
    pub fn trigger_stops(
        &mut self,
        now: u64,
        limit: u32,
        sink: &mut impl EventSink,
    ) -> Result<u32> {
        self.require_active()?;
        self.run_stops(now, limit, sink)
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn run_stops(
        &mut self,
        now: u64,
        limit: u32,
        sink: &mut impl EventSink,
    ) -> Result<u32> {
        let mut n = 0;
        while n < limit {
            let h = &*self.header;
            let last = h.last_price;
            let i = if met(TriggerDirection::Above, h.stop_above_min, last) {
                crate::rbtree::min(self.nodes, h.stops_above_root)?
            } else if met(TriggerDirection::Below, h.stop_below_max, last) {
                crate::rbtree::max(self.nodes, h.stops_below_root)?
            } else {
                break;
            };
            n += 1;
            self.trigger(i, now, sink)?;
        }
        Ok(n)
    }

    /// Takes stop `i` off its tree, cancels its OCO partner and places its
    /// order. An order that is rejected (say, a post-only order that would
    /// cross) just leaves its funds free.
    fn trigger(&mut self, i: u32, now: u64, sink: &mut impl EventSink) -> Result<()> {
        let s = *self.stop(i)?;
        if s.expires_at != 0 && now >= s.expires_at {
            return self.remove_stop(i, OutReason::Expired, sink);
        }
        let partner = s.partner();
        self.remove_stop(i, OutReason::Triggered, sink)?;
        if partner != NIL {
            self.remove_stop(partner, OutReason::OcoCancelled, sink)?;
        }
        let p = OrderParams {
            side: if s.is_ask() { Side::Ask } else { Side::Bid },
            price: s.price,
            base_lots: s.base_lots,
            order_type: OrderType::from_u8(s.order_type).ok_or(EngineError::Corrupted)?,
            self_trade: SelfTradeBehavior::from_u8(s.self_trade).ok_or(EngineError::Corrupted)?,
            client_order_id: s.client_order_id,
            expires_at: s.expires_at,
            quote_budget: None,
            match_limit: u32::from(s.match_limit),
            reverse_spread: 0,
        };
        match self.place_order_with::<false>(s.trader, &p, now, sink) {
            // These are all raised before the order changes anything.
            Ok(_)
            | Err(
                EngineError::PostOnlyWouldCross
                | EngineError::FillOrKillNotFilled
                | EngineError::SelfTrade
                | EngineError::OutOfSpace
                | EngineError::InvalidQuantity
                | EngineError::InsufficientFunds,
            ) => Ok(()),
            Err(e) => Err(e),
        }
    }
}
