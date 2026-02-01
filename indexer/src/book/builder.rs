//! Book builder implementation.
//!
//! Manages multiple order books and provides snapshot/delta generation.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use super::metrics::BookMetrics;
use super::orderbook::FullOrderBook;
use super::types::{BookChange, BookUpdate, PriceLevel, Side};
use crate::parser::ParsedOrder;

/// Book builder for managing multiple order books.
///
/// Maintains in-memory order book state for multiple markets and
/// provides snapshot and delta generation for clients.
///
/// # Example
///
/// ```rust,ignore
/// use matchbook_indexer::book::{BookBuilder, Side};
/// use matchbook_indexer::parser::ParsedOrder;
///
/// let mut builder = BookBuilder::new();
///
/// // Apply orders to a market
/// let market = [1u8; 32];
/// let orders = vec![/* parsed orders */];
/// let changes = builder.apply_update(market, Side::Bid, orders, 100);
///
/// // Get a snapshot for new subscribers
/// if let Some(snapshot) = builder.get_snapshot(&market, 10) {
///     // Send to client
/// }
/// ```
pub struct BookBuilder {
    /// Order books per market.
    books: HashMap<[u8; 32], FullOrderBook>,

    /// Metrics for monitoring.
    metrics: Arc<BookMetrics>,
}

impl Default for BookBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BookBuilder {
    /// Creates a new book builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            books: HashMap::new(),
            metrics: Arc::new(BookMetrics::new()),
        }
    }

    /// Returns a reference to the metrics.
    #[must_use]
    pub fn metrics(&self) -> Arc<BookMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Returns the number of markets being tracked.
    #[must_use]
    pub fn market_count(&self) -> usize {
        self.books.len()
    }

    /// Returns true if a market is being tracked.
    #[must_use]
    pub fn has_market(&self, market: &[u8; 32]) -> bool {
        self.books.contains_key(market)
    }

    /// Returns a reference to a market's order book.
    #[must_use]
    pub fn get_book(&self, market: &[u8; 32]) -> Option<&FullOrderBook> {
        self.books.get(market)
    }

    /// Applies an update to a market's order book.
    ///
    /// # Arguments
    ///
    /// * `market` - Market address
    /// * `side` - Which side to update
    /// * `orders` - New orders for this side
    /// * `slot` - Slot number of the update
    ///
    /// # Returns
    ///
    /// List of changes (added, modified, removed levels).
    pub fn apply_update(
        &mut self,
        market: [u8; 32],
        side: Side,
        orders: Vec<ParsedOrder>,
        slot: u64,
    ) -> Vec<BookChange> {
        let start = Instant::now();

        // Get or create the book
        let book = self
            .books
            .entry(market)
            .or_insert_with(|| FullOrderBook::new(market));

        // Apply the update
        let changes = book.apply_orders(side, &orders, slot);

        // Update metrics
        let elapsed = start.elapsed();
        self.metrics.record_update(elapsed);
        self.update_aggregate_metrics();

        changes
    }

    /// Generates a snapshot for a market.
    ///
    /// # Arguments
    ///
    /// * `market` - Market address
    /// * `depth` - Maximum number of levels per side (0 = all)
    ///
    /// # Returns
    ///
    /// A `BookUpdate::Snapshot` if the market exists, `None` otherwise.
    #[must_use]
    pub fn get_snapshot(&self, market: &[u8; 32], depth: usize) -> Option<BookUpdate> {
        let book = self.books.get(market)?;

        self.metrics.record_snapshot();

        Some(BookUpdate::Snapshot {
            market: *market,
            bids: book.aggregate_bids(depth),
            asks: book.aggregate_asks(depth),
            slot: book.last_slot,
            seq: book.seq,
        })
    }

    /// Generates a delta update for a market.
    ///
    /// # Arguments
    ///
    /// * `market` - Market address
    /// * `changes` - List of changes
    /// * `slot` - Slot number
    ///
    /// # Returns
    ///
    /// A `BookUpdate::Delta`.
    #[must_use]
    pub fn create_delta(
        &self,
        market: [u8; 32],
        changes: Vec<BookChange>,
        slot: u64,
    ) -> BookUpdate {
        let seq = self.books.get(&market).map(|b| b.seq).unwrap_or(0);

        BookUpdate::Delta {
            market,
            changes,
            slot,
            seq,
        }
    }

    /// Returns the best bid for a market.
    #[must_use]
    pub fn best_bid(&self, market: &[u8; 32]) -> Option<&PriceLevel> {
        self.books.get(market)?.best_bid()
    }

    /// Returns the best ask for a market.
    #[must_use]
    pub fn best_ask(&self, market: &[u8; 32]) -> Option<&PriceLevel> {
        self.books.get(market)?.best_ask()
    }

    /// Returns the spread for a market.
    #[must_use]
    pub fn spread(&self, market: &[u8; 32]) -> Option<u64> {
        self.books.get(market)?.spread()
    }

    /// Returns the mid price for a market.
    #[must_use]
    pub fn mid_price(&self, market: &[u8; 32]) -> Option<u64> {
        self.books.get(market)?.mid_price()
    }

    /// Removes a market from tracking.
    pub fn remove_market(&mut self, market: &[u8; 32]) -> Option<FullOrderBook> {
        let book = self.books.remove(market);
        self.update_aggregate_metrics();
        book
    }

    /// Clears all markets.
    pub fn clear(&mut self) {
        self.books.clear();
        self.update_aggregate_metrics();
    }

    /// Updates aggregate metrics across all books.
    fn update_aggregate_metrics(&self) {
        let total_depth: usize = self.books.values().map(|b| b.total_depth()).sum();
        self.metrics.set_total_depth(total_depth as u64);

        // Use spread from first market with a spread (for simplicity)
        let spread = self.books.values().find_map(|b| b.spread());
        self.metrics.set_spread(spread);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_order(price: u64, quantity: u64) -> ParsedOrder {
        ParsedOrder {
            order_id: 0,
            owner: [0u8; 32],
            price,
            quantity,
            client_order_id: 0,
        }
    }

    #[test]
    fn test_builder_new() {
        let builder = BookBuilder::new();
        assert_eq!(builder.market_count(), 0);
    }

    #[test]
    fn test_builder_default() {
        let builder = BookBuilder::default();
        assert_eq!(builder.market_count(), 0);
    }

    #[test]
    fn test_builder_apply_update() {
        let mut builder = BookBuilder::new();
        let market = [1u8; 32];

        let orders = vec![create_order(1000, 100), create_order(1001, 200)];

        let changes = builder.apply_update(market, Side::Bid, orders, 100);

        assert_eq!(builder.market_count(), 1);
        assert!(builder.has_market(&market));
        assert_eq!(changes.len(), 2);
    }

    #[test]
    fn test_builder_get_book() {
        let mut builder = BookBuilder::new();
        let market = [1u8; 32];

        assert!(builder.get_book(&market).is_none());

        builder.apply_update(market, Side::Bid, vec![create_order(1000, 100)], 100);

        let book = builder.get_book(&market);
        assert!(book.is_some());
        assert_eq!(book.expect("book").bid_depth(), 1);
    }

    #[test]
    fn test_builder_get_snapshot() {
        let mut builder = BookBuilder::new();
        let market = [1u8; 32];

        // No snapshot for unknown market
        assert!(builder.get_snapshot(&market, 10).is_none());

        // Add some orders
        builder.apply_update(
            market,
            Side::Bid,
            vec![create_order(1000, 100), create_order(1001, 200)],
            100,
        );
        builder.apply_update(
            market,
            Side::Ask,
            vec![create_order(1002, 150), create_order(1003, 250)],
            101,
        );

        let snapshot = builder.get_snapshot(&market, 10);
        assert!(snapshot.is_some());

        if let Some(BookUpdate::Snapshot { bids, asks, .. }) = snapshot {
            assert_eq!(bids.len(), 2);
            assert_eq!(asks.len(), 2);
            assert_eq!(bids[0].price, 1001); // Highest bid first
            assert_eq!(asks[0].price, 1002); // Lowest ask first
        } else {
            panic!("Expected snapshot");
        }
    }

    #[test]
    fn test_builder_create_delta() {
        let mut builder = BookBuilder::new();
        let market = [1u8; 32];

        builder.apply_update(market, Side::Bid, vec![create_order(1000, 100)], 100);

        let changes = vec![BookChange::new(Side::Bid, 1000, 200, 2)];
        let delta = builder.create_delta(market, changes, 101);

        assert!(delta.is_delta());
        assert_eq!(delta.slot(), 101);
    }

    #[test]
    fn test_builder_best_bid_ask() {
        let mut builder = BookBuilder::new();
        let market = [1u8; 32];

        // No best bid/ask for unknown market
        assert!(builder.best_bid(&market).is_none());
        assert!(builder.best_ask(&market).is_none());

        builder.apply_update(
            market,
            Side::Bid,
            vec![create_order(1000, 100), create_order(1001, 200)],
            100,
        );
        builder.apply_update(market, Side::Ask, vec![create_order(1005, 150)], 101);

        let best_bid = builder.best_bid(&market);
        let best_ask = builder.best_ask(&market);

        assert!(best_bid.is_some());
        assert_eq!(best_bid.expect("bid").price, 1001);

        assert!(best_ask.is_some());
        assert_eq!(best_ask.expect("ask").price, 1005);
    }

    #[test]
    fn test_builder_spread() {
        let mut builder = BookBuilder::new();
        let market = [1u8; 32];

        // No spread for unknown market
        assert!(builder.spread(&market).is_none());

        builder.apply_update(market, Side::Bid, vec![create_order(1000, 100)], 100);
        builder.apply_update(market, Side::Ask, vec![create_order(1005, 100)], 101);

        assert_eq!(builder.spread(&market), Some(5));
    }

    #[test]
    fn test_builder_mid_price() {
        let mut builder = BookBuilder::new();
        let market = [1u8; 32];

        builder.apply_update(market, Side::Bid, vec![create_order(1000, 100)], 100);
        builder.apply_update(market, Side::Ask, vec![create_order(1010, 100)], 101);

        assert_eq!(builder.mid_price(&market), Some(1005));
    }

    #[test]
    fn test_builder_remove_market() {
        let mut builder = BookBuilder::new();
        let market = [1u8; 32];

        builder.apply_update(market, Side::Bid, vec![create_order(1000, 100)], 100);
        assert_eq!(builder.market_count(), 1);

        let removed = builder.remove_market(&market);
        assert!(removed.is_some());
        assert_eq!(builder.market_count(), 0);
    }

    #[test]
    fn test_builder_clear() {
        let mut builder = BookBuilder::new();

        builder.apply_update([1u8; 32], Side::Bid, vec![create_order(1000, 100)], 100);
        builder.apply_update([2u8; 32], Side::Bid, vec![create_order(1000, 100)], 100);

        assert_eq!(builder.market_count(), 2);

        builder.clear();

        assert_eq!(builder.market_count(), 0);
    }

    #[test]
    fn test_builder_metrics() {
        let mut builder = BookBuilder::new();
        let market = [1u8; 32];

        builder.apply_update(market, Side::Bid, vec![create_order(1000, 100)], 100);
        builder.apply_update(market, Side::Ask, vec![create_order(1001, 100)], 101);

        let metrics = builder.metrics();
        assert_eq!(metrics.update_count(), 2);
        assert_eq!(metrics.total_depth(), 2);
    }

    #[test]
    fn test_builder_multiple_markets() {
        let mut builder = BookBuilder::new();
        let market1 = [1u8; 32];
        let market2 = [2u8; 32];

        builder.apply_update(market1, Side::Bid, vec![create_order(1000, 100)], 100);
        builder.apply_update(market2, Side::Bid, vec![create_order(2000, 200)], 100);

        assert_eq!(builder.market_count(), 2);
        assert!(builder.has_market(&market1));
        assert!(builder.has_market(&market2));

        assert_eq!(builder.best_bid(&market1).expect("bid1").price, 1000);
        assert_eq!(builder.best_bid(&market2).expect("bid2").price, 2000);
    }
}
