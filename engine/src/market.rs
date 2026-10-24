//! Market buffer: loading, allocation, seats, balances and read views.

use bytemuck::{cast_mut, cast_ref};

use crate::header::{MarketHeader, MarketStatus, HEADER_SIZE, MARKET_MAGIC};
use crate::node::{Node, NodeTag, OrderLeaf, TraderLeaf, FLAG_ASK, NIL, NODE_SIZE};
use crate::rbtree::{self, Tree};
use crate::{BookLevel, EngineError, RestingOrder, Result, Side};

/// Maximum fee in basis points (10%).
pub const MAX_FEE_BPS: u32 = 1_000;

/// Bytes needed for a market buffer holding `nodes` nodes.
#[must_use]
pub const fn market_size(nodes: usize) -> usize {
    HEADER_SIZE + nodes * NODE_SIZE
}

/// Parameters for [`Market::init`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MarketParams {
    /// Base mint.
    pub base_mint: [u8; 32],
    /// Quote mint.
    pub quote_mint: [u8; 32],
    /// Base vault.
    pub base_vault: [u8; 32],
    /// Quote vault.
    pub quote_vault: [u8; 32],
    /// Authority.
    pub authority: [u8; 32],
    /// Base atoms per base lot.
    pub base_lot_size: u64,
    /// Quote atoms per quote lot.
    pub quote_lot_size: u64,
    /// Minimum order size in base lots.
    pub min_base_lots: u64,
    /// Taker fee in basis points (max 1000).
    pub taker_fee_bps: u32,
    /// Maker rebate in basis points (at most the taker fee).
    pub maker_rebate_bps: u32,
    /// Opaque value for the host program.
    pub host_bump: u32,
}

/// Totals returned by [`Market::check_invariants`]. The host's vaults must
/// hold at least these amounts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MarketTotals {
    /// Base atoms owed to traders (free plus locked).
    pub base_atoms: u128,
    /// Quote atoms owed to traders (free plus locked) plus accrued fees.
    pub quote_atoms: u128,
    /// Resting orders.
    pub orders: u32,
    /// Seats.
    pub traders: u32,
}

/// Which tree an operation targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TreeId {
    Bids,
    Asks,
    Traders,
}

impl TreeId {
    pub(crate) const fn of(side: Side) -> Self {
        match side {
            Side::Bid => Self::Bids,
            Side::Ask => Self::Asks,
        }
    }
}

/// A mutable view over a market buffer.
pub struct Market<'a> {
    pub(crate) header: &'a mut MarketHeader,
    pub(crate) nodes: &'a mut [Node],
}

fn split(data: &mut [u8]) -> Result<(&mut MarketHeader, &mut [Node])> {
    if data.len() < HEADER_SIZE {
        return Err(EngineError::InvalidAccountData);
    }
    let (h, rest) = data.split_at_mut(HEADER_SIZE);
    let count = (rest.len() / NODE_SIZE).min((NIL - 1) as usize);
    let rest = rest
        .get_mut(..count * NODE_SIZE)
        .ok_or(EngineError::InvalidAccountData)?;
    let header =
        bytemuck::try_from_bytes_mut(h).map_err(|_| EngineError::InvalidAccountData)?;
    let nodes =
        bytemuck::try_cast_slice_mut(rest).map_err(|_| EngineError::InvalidAccountData)?;
    Ok((header, nodes))
}

impl<'a> Market<'a> {
    /// Initializes a zeroed buffer as a market.
    pub fn init(data: &mut [u8], p: &MarketParams) -> Result<()> {
        if p.base_lot_size == 0
            || p.quote_lot_size == 0
            || p.min_base_lots == 0
            || p.taker_fee_bps > MAX_FEE_BPS
            || p.maker_rebate_bps > p.taker_fee_bps
        {
            return Err(EngineError::InvalidMarketParams);
        }
        let (h, _) = split(data)?;
        if h.magic != 0 {
            return Err(EngineError::AlreadyInitialized);
        }
        *h = bytemuck::Zeroable::zeroed();
        h.magic = MARKET_MAGIC;
        h.base_mint = p.base_mint;
        h.quote_mint = p.quote_mint;
        h.base_vault = p.base_vault;
        h.quote_vault = p.quote_vault;
        h.authority = p.authority;
        h.base_lot_size = p.base_lot_size;
        h.quote_lot_size = p.quote_lot_size;
        h.min_base_lots = p.min_base_lots;
        h.taker_fee_bps = p.taker_fee_bps;
        h.maker_rebate_bps = p.maker_rebate_bps;
        h.host_bump = p.host_bump;
        h.bids_root = NIL;
        h.asks_root = NIL;
        h.traders_root = NIL;
        h.free_head = NIL;
        h.best_bid = NIL;
        h.best_ask = NIL;
        h.status = MarketStatus::Active as u32;
        Ok(())
    }

    /// Loads an initialized market. Any extra nodes at the end of the buffer
    /// (after the host grew it) become available immediately.
    pub fn load_mut(data: &'a mut [u8]) -> Result<Self> {
        let (header, nodes) = split(data)?;
        if header.magic != MARKET_MAGIC || header.next_unused as usize > nodes.len() {
            return Err(EngineError::InvalidAccountData);
        }
        Ok(Self { header, nodes })
    }

    /// Header.
    #[must_use]
    pub fn header(&self) -> &MarketHeader {
        self.header
    }

    /// Current status.
    pub fn status(&self) -> Result<MarketStatus> {
        MarketStatus::from_u32(self.header.status).ok_or(EngineError::Corrupted)
    }

    /// Changes the status. Closing requires an empty book.
    pub fn set_status(&mut self, s: MarketStatus) -> Result<()> {
        if s == MarketStatus::Closed && self.header.order_count != 0 {
            return Err(EngineError::InvalidMarketParams);
        }
        self.header.status = s as u32;
        Ok(())
    }

    pub(crate) fn require_status(&self, allowed: &[MarketStatus]) -> Result<()> {
        if allowed.contains(&self.status()?) {
            Ok(())
        } else {
            Err(EngineError::MarketNotActive)
        }
    }

    /// Nodes that can be allocated without growing the buffer.
    #[must_use]
    pub fn free_nodes(&self) -> u32 {
        let tail = (self.nodes.len() as u32).saturating_sub(self.header.next_unused);
        self.header.free_list_len.saturating_add(tail)
    }

    /// Extra bytes the host must add so that `needed` nodes are free.
    #[must_use]
    pub fn bytes_to_grow(&self, needed: u32) -> usize {
        needed.saturating_sub(self.free_nodes()) as usize * NODE_SIZE
    }

    pub(crate) fn require_space(&self, needed: u32) -> Result<()> {
        if self.free_nodes() >= needed {
            Ok(())
        } else {
            Err(EngineError::OutOfSpace)
        }
    }

    pub(crate) fn alloc(&mut self) -> Result<u32> {
        let i = if self.header.free_head != NIL {
            let i = self.header.free_head;
            let n = self.node(i)?;
            if !n.is(NodeTag::Free) {
                return Err(EngineError::Corrupted);
            }
            self.header.free_head = n.extra;
            self.header.free_list_len = self.header.free_list_len.saturating_sub(1);
            i
        } else if (self.header.next_unused as usize) < self.nodes.len() {
            let i = self.header.next_unused;
            self.header.next_unused = i.saturating_add(1);
            i
        } else {
            return Err(EngineError::OutOfSpace);
        };
        *self.node_mut(i)? = bytemuck::Zeroable::zeroed();
        Ok(i)
    }

    pub(crate) fn release(&mut self, i: u32) -> Result<()> {
        let head = self.header.free_head;
        let n = self.node_mut(i)?;
        *n = bytemuck::Zeroable::zeroed();
        n.tag = NodeTag::Free as u32;
        n.extra = head;
        self.header.free_head = i;
        self.header.free_list_len = self.header.free_list_len.saturating_add(1);
        Ok(())
    }

    #[inline]
    pub(crate) fn node(&self, i: u32) -> Result<&Node> {
        rbtree::node(self.nodes, i)
    }

    #[inline]
    pub(crate) fn node_mut(&mut self, i: u32) -> Result<&mut Node> {
        self.nodes.get_mut(i as usize).ok_or(EngineError::Corrupted)
    }

    pub(crate) fn tree(&mut self, t: TreeId) -> Tree<'_> {
        let root = match t {
            TreeId::Bids => &mut self.header.bids_root,
            TreeId::Asks => &mut self.header.asks_root,
            TreeId::Traders => &mut self.header.traders_root,
        };
        Tree {
            nodes: self.nodes,
            root,
        }
    }

    /// Order node `i`.
    pub fn order(&self, i: u32) -> Result<&OrderLeaf> {
        let n = self.node(i)?;
        if !n.is(NodeTag::Order) {
            return Err(EngineError::OrderNotFound);
        }
        Ok(cast_ref(n))
    }

    pub(crate) fn order_mut(&mut self, i: u32) -> Result<&mut OrderLeaf> {
        let n = self.node_mut(i)?;
        if !n.is(NodeTag::Order) {
            return Err(EngineError::OrderNotFound);
        }
        Ok(cast_mut(n))
    }

    /// Trader seat `i`.
    pub fn trader(&self, i: u32) -> Result<&TraderLeaf> {
        let n = self.node(i)?;
        if !n.is(NodeTag::Trader) {
            return Err(EngineError::TraderNotFound);
        }
        Ok(cast_ref(n))
    }

    pub(crate) fn trader_mut(&mut self, i: u32) -> Result<&mut TraderLeaf> {
        let n = self.node_mut(i)?;
        if !n.is(NodeTag::Trader) {
            return Err(EngineError::TraderNotFound);
        }
        Ok(cast_mut(n))
    }

    /// Finds the seat of `owner`.
    pub fn find_trader(&self, owner: &[u8; 32]) -> Result<Option<u32>> {
        let i = rbtree::find(self.nodes, self.header.traders_root, TraderLeaf::key_for(owner))?;
        if i == NIL {
            return Ok(None);
        }
        Ok((self.trader(i)?.owner == *owner).then_some(i))
    }

    /// Resolves a seat from a client-supplied hint, falling back to a lookup.
    pub fn resolve_trader(&self, owner: &[u8; 32], hint: Option<u32>) -> Result<u32> {
        if let Some(h) = hint {
            if self.trader(h).is_ok_and(|t| t.owner == *owner) {
                return Ok(h);
            }
        }
        self.find_trader(owner)?.ok_or(EngineError::TraderNotFound)
    }

    /// Creates a seat for `owner`. Needs one free node.
    pub fn claim_seat(&mut self, owner: &[u8; 32]) -> Result<u32> {
        self.require_status(&[MarketStatus::Active])?;
        let key = TraderLeaf::key_for(owner);
        if rbtree::find(self.nodes, self.header.traders_root, key)? != NIL {
            return Err(EngineError::SeatAlreadyClaimed);
        }
        self.require_space(1)?;
        let i = self.alloc()?;
        {
            let t: &mut TraderLeaf = cast_mut(self.node_mut(i)?);
            t.tag = NodeTag::Trader as u32;
            t.order_head = NIL;
            t.owner = *owner;
        }
        self.node_mut(i)?.set_key(key);
        self.tree(TreeId::Traders).insert(i)?;
        self.header.trader_count = self.header.trader_count.saturating_add(1);
        Ok(i)
    }

    /// Removes an empty seat and frees its node.
    pub fn release_seat(&mut self, i: u32) -> Result<()> {
        let t = self.trader(i)?;
        if t.order_head != NIL || t.base_free != 0 || t.quote_free != 0 {
            return Err(EngineError::SeatNotEmpty);
        }
        self.tree(TreeId::Traders).remove(i)?;
        self.release(i)?;
        self.header.trader_count = self.header.trader_count.saturating_sub(1);
        Ok(())
    }

    /// Credits atoms the host already moved into the vaults.
    pub fn deposit(&mut self, i: u32, base_atoms: u64, quote_atoms: u64) -> Result<()> {
        self.require_status(&[MarketStatus::Active])?;
        let t = self.trader_mut(i)?;
        let b = t.base_free.checked_add(base_atoms).ok_or(EngineError::Overflow)?;
        let q = t.quote_free.checked_add(quote_atoms).ok_or(EngineError::Overflow)?;
        t.base_free = b;
        t.quote_free = q;
        Ok(())
    }

    /// Debits free balances; the host then moves atoms out of the vaults.
    /// Always allowed, whatever the market status.
    pub fn withdraw(&mut self, i: u32, base_atoms: u64, quote_atoms: u64) -> Result<()> {
        let t = self.trader_mut(i)?;
        if t.base_free < base_atoms || t.quote_free < quote_atoms {
            return Err(EngineError::InsufficientFunds);
        }
        t.base_free -= base_atoms;
        t.quote_free -= quote_atoms;
        Ok(())
    }

    /// Takes all accrued fees; the host moves them out of the quote vault.
    pub fn collect_fees(&mut self) -> u64 {
        core::mem::take(&mut self.header.fees_accrued)
    }

    /// Best bid order node, ignoring expiry.
    #[must_use]
    pub fn best_bid(&self) -> u32 {
        self.header.best_bid
    }

    /// Best ask order node, ignoring expiry.
    #[must_use]
    pub fn best_ask(&self) -> u32 {
        self.header.best_ask
    }

    /// Next order in priority order on `side`, or [`NIL`].
    pub fn next_order(&self, side: Side, i: u32) -> Result<u32> {
        match side {
            Side::Ask => rbtree::next(self.nodes, i),
            Side::Bid => rbtree::prev(self.nodes, i),
        }
    }

    /// Visits live (unexpired) orders on `side` in priority order until `f`
    /// returns false.
    pub fn for_each_order(
        &self,
        side: Side,
        now: u64,
        mut f: impl FnMut(RestingOrder) -> bool,
    ) -> Result<()> {
        let mut i = match side {
            Side::Bid => self.header.best_bid,
            Side::Ask => self.header.best_ask,
        };
        while i != NIL {
            let o = self.order(i)?;
            if !o.is_expired(now) {
                let owner = self.trader(o.trader)?.owner;
                if !f(resting(o, i, owner)) {
                    break;
                }
            }
            i = self.next_order(side, i)?;
        }
        Ok(())
    }

    /// Aggregated levels on `side`, best first, until `f` returns false.
    pub fn for_each_level(
        &self,
        side: Side,
        now: u64,
        mut f: impl FnMut(BookLevel) -> bool,
    ) -> Result<()> {
        let mut cur: Option<BookLevel> = None;
        let mut stop = false;
        self.for_each_order(side, now, |o| {
            match cur.as_mut() {
                Some(l) if l.price == o.price => {
                    l.base_lots = l.base_lots.saturating_add(o.base_lots);
                    l.orders = l.orders.saturating_add(1);
                }
                _ => {
                    if let Some(l) = cur.take() {
                        if !f(l) {
                            stop = true;
                            return false;
                        }
                    }
                    cur = Some(BookLevel {
                        price: o.price,
                        base_lots: o.base_lots,
                        orders: 1,
                    });
                }
            }
            true
        })?;
        if let (false, Some(l)) = (stop, cur) {
            f(l);
        }
        Ok(())
    }

    /// Visits a trader's orders, newest first.
    pub fn for_each_trader_order(&self, trader: u32, mut f: impl FnMut(u32, RestingOrder)) -> Result<()> {
        let t = self.trader(trader)?;
        let owner = t.owner;
        let mut i = t.order_head;
        while i != NIL {
            let o = self.order(i)?;
            f(i, resting(o, i, owner));
            i = o.next;
        }
        Ok(())
    }

    /// Base and quote atoms locked in a trader's resting orders.
    pub fn locked(&self, trader: u32) -> Result<(u64, u64)> {
        let mut base = 0u64;
        let mut quote = 0u64;
        let mut err = None;
        self.for_each_trader_order(trader, |_, o| {
            let r = match o.side {
                Side::Ask => self.base_for(o.base_lots).and_then(|b| {
                    base = base.checked_add(b).ok_or(EngineError::Overflow)?;
                    Ok(())
                }),
                Side::Bid => self.quote_for(o.base_lots, o.price).and_then(|q| {
                    quote = quote.checked_add(q).ok_or(EngineError::Overflow)?;
                    Ok(())
                }),
            };
            if let Err(e) = r {
                err = Some(e);
            }
        })?;
        err.map_or(Ok((base, quote)), Err)
    }

    pub(crate) fn base_for(&self, lots: u64) -> Result<u64> {
        crate::math::mul(lots, self.header.base_lot_size).ok_or(EngineError::Overflow)
    }

    #[inline]
    pub(crate) fn quote_for(&self, lots: u64, price: u64) -> Result<u64> {
        crate::math::mul(lots, price)
            .and_then(|v| crate::math::mul(v, self.header.quote_lot_size))
            .ok_or(EngineError::Overflow)
    }

    /// Verifies every structural and accounting invariant. Intended for tests,
    /// fuzzing and off-chain audits; it walks the whole buffer.
    pub fn check_invariants(&self) -> Result<MarketTotals> {
        let h = &*self.header;
        let bids = rbtree::validate(self.nodes, h.bids_root)?;
        let asks = rbtree::validate(self.nodes, h.asks_root)?;
        let traders = rbtree::validate(self.nodes, h.traders_root)?;
        let bad = |c: bool| if c { Err(EngineError::Corrupted) } else { Ok(()) };
        bad(bids + asks != h.order_count || traders != h.trader_count)?;
        bad(h.best_bid != rbtree::max(self.nodes, h.bids_root)?)?;
        bad(h.best_ask != rbtree::min(self.nodes, h.asks_root)?)?;

        let mut free = 0u32;
        let mut i = h.free_head;
        while i != NIL {
            let n = self.node(i)?;
            bad(!n.is(NodeTag::Free))?;
            free = free.checked_add(1).ok_or(EngineError::Corrupted)?;
            bad(free > h.next_unused)?;
            i = n.extra;
        }
        bad(free != h.free_list_len)?;
        bad(free + h.order_count + h.trader_count != h.next_unused)?;

        let mut totals = MarketTotals {
            orders: h.order_count,
            traders: h.trader_count,
            quote_atoms: u128::from(h.fees_accrued),
            ..MarketTotals::default()
        };
        let mut seen_orders = 0u32;
        for idx in 0..h.next_unused {
            let n = self.node(idx)?;
            if n.is(NodeTag::Trader) {
                let t = self.trader(idx)?;
                bad(TraderLeaf::key_for(&t.owner) != n.key())?;
                let (bl, ql) = self.locked(idx)?;
                totals.base_atoms += u128::from(t.base_free) + u128::from(bl);
                totals.quote_atoms += u128::from(t.quote_free) + u128::from(ql);
                let mut prev = NIL;
                let mut j = t.order_head;
                while j != NIL {
                    let o = self.order(j)?;
                    bad(o.trader != idx || o.prev != prev || o.base_lots == 0)?;
                    let side = if o.is_ask() { Side::Ask } else { Side::Bid };
                    bad(order_key(side, o.price(), o.seq) != n_key(self.node(j)?))?;
                    bad(o.seq == 0 || o.seq > h.seq_num)?;
                    seen_orders = seen_orders.checked_add(1).ok_or(EngineError::Corrupted)?;
                    bad(seen_orders > h.order_count)?;
                    prev = j;
                    j = o.next;
                }
            }
        }
        bad(seen_orders != h.order_count)?;
        Ok(totals)
    }
}

fn n_key(n: &Node) -> u128 {
    n.key()
}

pub(crate) fn order_key(side: Side, price: u64, seq: u64) -> u128 {
    let lo = match side {
        Side::Ask => seq,
        Side::Bid => !seq,
    };
    (u128::from(price) << 64) | u128::from(lo)
}

pub(crate) fn resting(o: &OrderLeaf, index: u32, owner: [u8; 32]) -> RestingOrder {
    RestingOrder {
        order_id: crate::node::order_id(o.seq, index),
        side: if o.flags & FLAG_ASK != 0 { Side::Ask } else { Side::Bid },
        price: o.price(),
        base_lots: o.base_lots,
        client_order_id: o.client_order_id,
        expires_at: o.expires_at,
        owner,
    }
}
