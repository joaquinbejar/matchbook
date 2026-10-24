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
//! Fills carry only what cannot be derived. The taker is the owner account
//! of the instruction that emitted them, and each fill's quote amount, taker
//! fee and maker rebate follow from the market's lot sizes and fee rates.

use matchbook_engine::{EventSink, FillEvent, OrderOutEvent, OutReason, PlacedEvent};

/// Event tags.
pub mod tag {
    /// A maker order was filled. See [`super::FILL_LEN`].
    pub const FILL: u8 = 1;
    /// An order left the book without trading. See [`super::OUT_LEN`].
    pub const OUT: u8 = 2;
    /// A reverse order was filled and re-posted on the other side. See
    /// [`super::REVERSED_LEN`].
    pub const REVERSED: u8 = 3;
    /// Reserved.
    pub const TAKE: u8 = 4;
}

/// Fill: tag, maker owner (32), maker order id, maker client order id,
/// price, base lots, maker remaining lots.
pub const FILL_LEN: usize = 1 + 32 + 8 * 5;
/// Out: tag, owner (32), order id, client order id, side (1), price,
/// base lots, reason (1).
pub const OUT_LEN: usize = 1 + 32 + 8 + 8 + 1 + 8 + 8 + 1;

/// Reversed: tag, owner (32), new order id, client order id, side (1),
/// price, base lots, expiry.
pub const REVERSED_LEN: usize = 1 + 32 + 8 + 8 + 1 + 8 + 8 + 8;

/// Bytes buffered before an early flush. Large enough for 13 fills.
const CAPACITY: usize = 960;

struct Enc<'a, const N: usize> {
    b: &'a mut [u8; N],
    at: usize,
}

impl<'a, const N: usize> Enc<'a, N> {
    #[inline(always)]
    fn new(b: &'a mut [u8; N], tag: u8) -> Self {
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

#[inline(always)]
fn write_fill(out: &mut [u8; FILL_LEN], e: &FillEvent) {
    let mut b = Enc::new(out, tag::FILL);
    b.put(e.maker);
    b.u64(e.maker_order_id);
    b.u64(e.maker_client_order_id);
    b.u64(e.price);
    b.u64(e.base_lots);
    b.u64(e.maker_remaining);
}

#[inline(always)]
fn write_out(out: &mut [u8; OUT_LEN], e: &OrderOutEvent) {
    let mut b = Enc::new(out, tag::OUT);
    b.put(&e.owner);
    b.u64(e.order_id);
    b.u64(e.client_order_id);
    b.u8(e.side as u8);
    b.u64(e.price);
    b.u64(e.base_lots);
    b.u8(e.reason as u8);
}

#[inline(always)]
fn write_reversed(out: &mut [u8; REVERSED_LEN], e: &PlacedEvent) {
    let mut b = Enc::new(out, tag::REVERSED);
    b.put(&e.owner);
    b.u64(e.order_id);
    b.u64(e.client_order_id);
    b.u8(e.side as u8);
    b.u64(e.price);
    b.u64(e.base_lots);
    b.u64(e.expires_at);
}

/// Encodes a fill event.
pub fn encode_fill(e: &FillEvent) -> [u8; FILL_LEN] {
    let mut b = [0; FILL_LEN];
    write_fill(&mut b, e);
    b
}

/// Encodes an out event.
pub fn encode_out(e: &OrderOutEvent) -> [u8; OUT_LEN] {
    let mut b = [0; OUT_LEN];
    write_out(&mut b, e);
    b
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
        Self {
            buf: [0; CAPACITY],
            len: 0,
        }
    }

    /// Reserves `K` bytes at the end of the buffer, flushing first if they
    /// would not fit.
    #[inline(always)]
    fn slot<const K: usize>(&mut self) -> Option<&mut [u8; K]> {
        if self.len + K > CAPACITY {
            self.flush();
        }
        let at = self.len;
        self.len += K;
        self.buf.get_mut(at..at + K).and_then(|s| s.try_into().ok())
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
        if let Some(b) = self.slot() {
            write_fill(b, e);
        }
    }
    fn on_out(&mut self, e: &OrderOutEvent) {
        if self.wants_out(e.reason) {
            if let Some(b) = self.slot() {
                write_out(b, e);
            }
        }
    }
    fn on_reversed(&mut self, e: &PlacedEvent) {
        if let Some(b) = self.slot() {
            write_reversed(b, e);
        }
    }
}
