//! Events emitted while operating on a market.

use crate::Side;

/// Why an order left the book.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OutReason {
    /// Fully filled.
    Filled = 0,
    /// Cancelled by its owner.
    Cancelled = 1,
    /// Removed after its expiry.
    Expired = 2,
    /// Cancelled by self-trade prevention.
    SelfTrade = 3,
}

/// A trade between a resting order and a taker. Borrows the owners from
/// the market so that building it costs no copies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FillEvent<'a> {
    /// Maker owner.
    pub maker: &'a [u8; 32],
    /// Maker order id.
    pub maker_order_id: u64,
    /// Maker client order id.
    pub maker_client_order_id: u64,
    /// Taker owner, or `None` for seatless swaps.
    pub taker: Option<&'a [u8; 32]>,
    /// Taker side.
    pub taker_side: Side,
    /// Execution price (the maker's price).
    pub price: u64,
    /// Base lots traded.
    pub base_lots: u64,
    /// Quote atoms traded before fees.
    pub quote_atoms: u64,
    /// Fee paid by the taker.
    pub taker_fee: u64,
    /// Rebate credited to the maker.
    pub maker_rebate: u64,
    /// Base lots left on the maker order.
    pub maker_remaining: u64,
}

/// An order leaving the book.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderOutEvent {
    /// Owner.
    pub owner: [u8; 32],
    /// Order id.
    pub order_id: u64,
    /// Client order id.
    pub client_order_id: u64,
    /// Side.
    pub side: Side,
    /// Price.
    pub price: u64,
    /// Base lots that were still resting (0 when filled).
    pub base_lots: u64,
    /// Reason.
    pub reason: OutReason,
}

/// An order resting on the book.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedEvent {
    /// Owner.
    pub owner: [u8; 32],
    /// Order id.
    pub order_id: u64,
    /// Client order id.
    pub client_order_id: u64,
    /// Side.
    pub side: Side,
    /// Price.
    pub price: u64,
    /// Base lots resting.
    pub base_lots: u64,
    /// Expiry (0 = never).
    pub expires_at: u64,
}

/// Receives events. All methods default to doing nothing.
///
/// The constants and [`EventSink::wants_out`] let a sink opt out of events it
/// ignores, so the engine does not spend compute building them.
pub trait EventSink {
    /// Whether [`EventSink::on_fill`] is used.
    const FILLS: bool = true;
    /// Whether [`EventSink::on_placed`] is used.
    const PLACED: bool = true;
    /// Whether [`EventSink::on_out`] is used for orders leaving for `reason`.
    fn wants_out(&self, _reason: OutReason) -> bool {
        true
    }
    /// A fill happened.
    fn on_fill(&mut self, _e: &FillEvent<'_>) {}
    /// An order left the book.
    fn on_out(&mut self, _e: &OrderOutEvent) {}
    /// An order started resting.
    fn on_placed(&mut self, _e: &PlacedEvent) {}
}

/// Discards every event.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopSink;

impl EventSink for NoopSink {
    const FILLS: bool = false;
    const PLACED: bool = false;
    fn wants_out(&self, _reason: OutReason) -> bool {
        false
    }
}
