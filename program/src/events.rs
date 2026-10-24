//! Events written to the transaction log with `sol_log_data`.
//!
//! A log entry costs 200 compute units plus one per byte, so events are
//! packed back to back into a buffer and written as a single `Program data:`
//! entry when the instruction ends (or when the buffer fills up). Indexers
//! split an entry by reading the one-byte tag in front of each event; every
//! event has a fixed size per tag. All integers are little-endian.
//!
//! Orders placed and cancelled by their owner are not logged: the order id
//! of a placement is the instruction's return data, and the owner already
//! knows what it cancelled. Orders that leave the book because of someone
//! else's transaction (expiry, self-trade prevention) are logged.
//!
//! Fills carry only what cannot be derived: an instruction that takes
//! liquidity ends with one [`tag::TAKE`] summary naming the taker and its
//! totals, and each fill's quote amount, taker fee and maker rebate follow
//! from the market's lot sizes and fee rates.

use matchbook_engine::{EventSink, FillEvent, OrderOutEvent, OutReason, PlacedEvent, Side};

/// Event tags.
pub mod tag {
    /// A maker order was filled. See [`super::FILL_LEN`].
    pub const FILL: u8 = 1;
    /// An order left the book without trading. See [`super::OUT_LEN`].
    pub const OUT: u8 = 2;
    /// Reserved: placements are reported through return data.
    pub const PLACED: u8 = 3;
    /// Totals of one taking order or swap. See [`super::TAKE_LEN`].
    pub const TAKE: u8 = 4;
}

/// Fill: tag, maker owner (32), maker order id, maker client order id,
/// price, base lots, maker remaining lots.
pub const FILL_LEN: usize = 1 + 32 + 8 * 5;
/// Out: tag, owner (32), order id, client order id, side (1), price,
/// base lots, reason (1).
pub const OUT_LEN: usize = 1 + 32 + 8 + 8 + 1 + 8 + 8 + 1;
/// Placed: tag, owner (32), order id, client order id, side (1), price,
/// base lots, expires at.
pub const PLACED_LEN: usize = 1 + 32 + 8 + 8 + 1 + 8 + 8 + 8;
/// Take: tag, taker (32, zero for a seatless swap), side (1), base lots,
/// quote atoms before fees, taker fee atoms.
pub const TAKE_LEN: usize = 1 + 32 + 1 + 8 + 8 + 8;

/// Bytes buffered before an early flush. Large enough for 13 fills.
const CAPACITY: usize = 960;

struct Enc<const N: usize> {
    b: [u8; N],
    at: usize,
}

impl<const N: usize> Enc<N> {
    #[inline(always)]
    fn new(tag: u8) -> Self {
        let mut b = [0u8; N];
        if let Some(t) = b.first_mut() {
            *t = tag;
        }
        Self { b, at: 1 }
    }
    #[inline(always)]
    fn put<const K: usize>(&mut self, v: &[u8; K]) {
        if let Some(dst) = self.b.get_mut(self.at..self.at + K) {
            dst.copy_from_slice(v);
        }
        self.at += K;
    }
    #[inline(always)]
    fn u64(&mut self, v: u64) {
        self.put(&v.to_le_bytes());
    }
    #[inline(always)]
    fn u8(&mut self, v: u8) {
        self.put(&[v]);
    }
}

/// Encodes a fill event.
pub fn encode_fill(e: &FillEvent) -> [u8; FILL_LEN] {
    let mut b = Enc::<FILL_LEN>::new(tag::FILL);
    b.put(&e.maker);
    b.u64(e.maker_order_id);
    b.u64(e.maker_client_order_id);
    b.u64(e.price);
    b.u64(e.base_lots);
    b.u64(e.maker_remaining);
    b.b
}

/// Encodes an out event.
pub fn encode_out(e: &OrderOutEvent) -> [u8; OUT_LEN] {
    let mut b = Enc::<OUT_LEN>::new(tag::OUT);
    b.put(&e.owner);
    b.u64(e.order_id);
    b.u64(e.client_order_id);
    b.u8(e.side as u8);
    b.u64(e.price);
    b.u64(e.base_lots);
    b.u8(e.reason as u8);
    b.b
}

/// Encodes a placed event.
pub fn encode_placed(e: &PlacedEvent) -> [u8; PLACED_LEN] {
    let mut b = Enc::<PLACED_LEN>::new(tag::PLACED);
    b.put(&e.owner);
    b.u64(e.order_id);
    b.u64(e.client_order_id);
    b.u8(e.side as u8);
    b.u64(e.price);
    b.u64(e.base_lots);
    b.u64(e.expires_at);
    b.b
}

/// Encodes a take summary.
pub fn encode_take(taker: &[u8; 32], side: Side, base_lots: u64, quote_atoms: u64, fee_atoms: u64) -> [u8; TAKE_LEN] {
    let mut b = Enc::<TAKE_LEN>::new(tag::TAKE);
    b.put(taker);
    b.u8(side as u8);
    b.u64(base_lots);
    b.u64(quote_atoms);
    b.u64(fee_atoms);
    b.b
}

/// Sink that buffers events and writes them as one log entry when dropped.
pub struct LogSink {
    buf: [u8; CAPACITY],
    len: usize,
}

impl LogSink {
    /// An empty sink.
    #[inline(always)]
    pub fn new() -> Self {
        Self { buf: [0; CAPACITY], len: 0 }
    }

    /// Appends one encoded event, flushing first if it would not fit.
    #[inline(always)]
    pub fn push<const K: usize>(&mut self, e: &[u8; K]) {
        if self.len + K > CAPACITY {
            self.flush();
        }
        if let Some(dst) = self.buf.get_mut(self.len..self.len + K) {
            dst.copy_from_slice(e);
            self.len += K;
        }
    }

    /// Records the totals of a taking order, if it traded.
    pub fn take(&mut self, taker: &[u8; 32], side: Side, base_lots: u64, quote_atoms: u64, fee_atoms: u64) {
        if base_lots > 0 {
            self.push(&encode_take(taker, side, base_lots, quote_atoms, fee_atoms));
        }
    }

    /// Writes buffered events to the log.
    pub fn flush(&mut self) {
        if let Some(data) = self.buf.get(..self.len) {
            if !data.is_empty() {
                crate::utils::log_data(data);
            }
        }
        self.len = 0;
    }
}

impl Default for LogSink {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for LogSink {
    fn drop(&mut self) {
        self.flush();
    }
}

impl EventSink for LogSink {
    const PLACED: bool = false;
    fn wants_out(&self, reason: OutReason) -> bool {
        // A filled order is described by its fill (`maker_remaining == 0`)
        // and a cancelled one by the owner's own instruction.
        matches!(reason, OutReason::Expired | OutReason::SelfTrade)
    }
    fn on_fill(&mut self, e: &FillEvent) {
        self.push(&encode_fill(e));
    }
    fn on_out(&mut self, e: &OrderOutEvent) {
        if self.wants_out(e.reason) {
            self.push(&encode_out(e));
        }
    }
}
