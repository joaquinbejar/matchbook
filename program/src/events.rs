//! Events written to the transaction log with `sol_log_data`.
//!
//! Every event is one log entry: a one-byte tag followed by little-endian
//! fields. Indexers decode them from `Program data:` lines.

use matchbook_engine::{EventSink, FillEvent, OrderOutEvent, OutReason, PlacedEvent};

/// Event tags.
pub mod tag {
    /// [`super::FillEvent`].
    pub const FILL: u8 = 1;
    /// [`super::OrderOutEvent`].
    pub const OUT: u8 = 2;
    /// [`super::PlacedEvent`].
    pub const PLACED: u8 = 3;
}

/// Size of an encoded fill event.
pub const FILL_LEN: usize = 1 + 32 + 8 + 8 + 32 + 1 + 8 * 6;
/// Size of an encoded out event.
pub const OUT_LEN: usize = 1 + 32 + 8 + 8 + 1 + 8 + 8 + 1;
/// Size of an encoded placed event.
pub const PLACED_LEN: usize = 1 + 32 + 8 + 8 + 1 + 8 + 8 + 8;

struct Buf<const N: usize> {
    b: [u8; N],
    at: usize,
}

impl<const N: usize> Buf<N> {
    fn new(tag: u8) -> Self {
        let mut b = [0u8; N];
        if let Some(x) = b.first_mut() {
            *x = tag;
        }
        Self { b, at: 1 }
    }
    fn put(&mut self, v: &[u8]) {
        if let Some(dst) = self.b.get_mut(self.at..self.at + v.len()) {
            dst.copy_from_slice(v);
        }
        self.at += v.len();
    }
    fn u64(&mut self, v: u64) {
        self.put(&v.to_le_bytes());
    }
}

/// Encodes a fill event.
pub fn encode_fill(e: &FillEvent) -> [u8; FILL_LEN] {
    let mut b = Buf::<FILL_LEN>::new(tag::FILL);
    b.put(&e.maker);
    b.u64(e.maker_order_id);
    b.u64(e.maker_client_order_id);
    b.put(&e.taker.unwrap_or([0; 32]));
    b.put(&[e.taker_side as u8]);
    b.u64(e.price);
    b.u64(e.base_lots);
    b.u64(e.quote_atoms);
    b.u64(e.taker_fee);
    b.u64(e.maker_rebate);
    b.u64(e.maker_remaining);
    b.b
}

/// Encodes an out event.
pub fn encode_out(e: &OrderOutEvent) -> [u8; OUT_LEN] {
    let mut b = Buf::<OUT_LEN>::new(tag::OUT);
    b.put(&e.owner);
    b.u64(e.order_id);
    b.u64(e.client_order_id);
    b.put(&[e.side as u8]);
    b.u64(e.price);
    b.u64(e.base_lots);
    b.put(&[e.reason as u8]);
    b.b
}

/// Encodes a placed event.
pub fn encode_placed(e: &PlacedEvent) -> [u8; PLACED_LEN] {
    let mut b = Buf::<PLACED_LEN>::new(tag::PLACED);
    b.put(&e.owner);
    b.u64(e.order_id);
    b.u64(e.client_order_id);
    b.put(&[e.side as u8]);
    b.u64(e.price);
    b.u64(e.base_lots);
    b.u64(e.expires_at);
    b.b
}

/// Sink that writes every event to the log (a no-op off-chain).
pub struct LogSink;

impl EventSink for LogSink {
    fn on_fill(&mut self, e: &FillEvent) {
        crate::utils::log_data(&encode_fill(e));
    }
    fn on_out(&mut self, e: &OrderOutEvent) {
        // A fully filled order is already described by its fill event
        // (`maker_remaining == 0`), so only cancels and expiries are logged.
        if e.reason != OutReason::Filled {
            crate::utils::log_data(&encode_out(e));
        }
    }
    fn on_placed(&mut self, e: &PlacedEvent) {
        crate::utils::log_data(&encode_placed(e));
    }
}
