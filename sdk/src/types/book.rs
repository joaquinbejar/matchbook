//! Order book types for the Matchbook SDK.
//!
//! Provides order book snapshot and level types.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::primitives::{Price, Quantity, Side};

/// A price level in the order book.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookLevel {
    /// Price in ticks.
    pub price: Price,

    /// Total quantity in lots at this level.
    pub quantity: Quantity,

    /// Number of orders at this level.
    pub order_count: u32,
}

impl BookLevel {
    /// Creates a new book level.
    #[must_use]
    pub fn new(price: Price, quantity: Quantity, order_count: u32) -> Self {
        Self {
            price,
            quantity,
            order_count,
        }
    }

    /// Returns true if this level is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.quantity.is_zero() || self.order_count == 0
    }
}

impl fmt::Display for BookLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} @ {} ({} orders)",
            self.quantity, self.price, self.order_count
        )
    }
}

/// Order book snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderBook {
    /// Market address (base58 encoded).
    pub market: String,

    /// Slot of the snapshot.
    pub slot: u64,

    /// Sequence number.
    pub sequence: u64,

    /// Bid levels (sorted by price descending).
    pub bids: Vec<BookLevel>,

    /// Ask levels (sorted by price ascending).
    pub asks: Vec<BookLevel>,

    /// Timestamp in milliseconds since epoch.
    pub timestamp: u64,
}

impl OrderBook {
    /// Creates a new empty order book.
    #[must_use]
    pub fn new(market: String, slot: u64, sequence: u64, timestamp: u64) -> Self {
        Self {
            market,
            slot,
            sequence,
            bids: Vec::new(),
            asks: Vec::new(),
            timestamp,
        }
    }

    /// Returns the best bid price.
    #[must_use]
    pub fn best_bid(&self) -> Option<Price> {
        self.bids.first().map(|l| l.price)
    }

    /// Returns the best ask price.
    #[must_use]
    pub fn best_ask(&self) -> Option<Price> {
        self.asks.first().map(|l| l.price)
    }

    /// Returns the best bid level.
    #[must_use]
    pub fn best_bid_level(&self) -> Option<&BookLevel> {
        self.bids.first()
    }

    /// Returns the best ask level.
    #[must_use]
    pub fn best_ask_level(&self) -> Option<&BookLevel> {
        self.asks.first()
    }

    /// Returns the spread in ticks.
    #[must_use]
    pub fn spread(&self) -> Option<Price> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some(ask.saturating_sub(bid)),
            _ => None,
        }
    }

    /// Returns the midpoint price in ticks.
    #[must_use]
    pub fn midpoint(&self) -> Option<Price> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some(Price::new((bid.value() + ask.value()) / 2)),
            _ => None,
        }
    }

    /// Returns true if the book is crossed (bid >= ask).
    #[must_use]
    pub fn is_crossed(&self) -> bool {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => bid >= ask,
            _ => false,
        }
    }

    /// Returns the total bid quantity up to a given depth.
    #[must_use]
    pub fn total_bid_quantity(&self, depth: usize) -> Quantity {
        self.bids
            .iter()
            .take(depth)
            .fold(Quantity::zero(), |acc, l| acc + l.quantity)
    }

    /// Returns the total ask quantity up to a given depth.
    #[must_use]
    pub fn total_ask_quantity(&self, depth: usize) -> Quantity {
        self.asks
            .iter()
            .take(depth)
            .fold(Quantity::zero(), |acc, l| acc + l.quantity)
    }

    /// Returns the number of bid levels.
    #[must_use]
    pub fn bid_depth(&self) -> usize {
        self.bids.len()
    }

    /// Returns the number of ask levels.
    #[must_use]
    pub fn ask_depth(&self) -> usize {
        self.asks.len()
    }

    /// Returns true if the book is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bids.is_empty() && self.asks.is_empty()
    }

    /// Returns the level at a specific price on the given side.
    #[must_use]
    pub fn level_at(&self, side: Side, price: Price) -> Option<&BookLevel> {
        let levels = match side {
            Side::Bid => &self.bids,
            Side::Ask => &self.asks,
        };
        levels.iter().find(|l| l.price == price)
    }
}

impl fmt::Display for OrderBook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "OrderBook {} (slot: {}, seq: {})",
            self.market, self.slot, self.sequence
        )?;
        writeln!(f, "  Asks:")?;
        for level in self.asks.iter().rev().take(5) {
            writeln!(f, "    {}", level)?;
        }
        writeln!(f, "  ---")?;
        writeln!(f, "  Bids:")?;
        for level in self.bids.iter().take(5) {
            writeln!(f, "    {}", level)?;
        }
        Ok(())
    }
}

/// A change to the order book.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookChange {
    /// Side of the change.
    pub side: Side,

    /// Price level.
    pub price: Price,

    /// New quantity (0 means level removed).
    pub new_quantity: Quantity,

    /// New order count.
    pub new_order_count: u32,
}

impl BookChange {
    /// Creates a new book change.
    #[must_use]
    pub fn new(side: Side, price: Price, new_quantity: Quantity, new_order_count: u32) -> Self {
        Self {
            side,
            price,
            new_quantity,
            new_order_count,
        }
    }

    /// Returns true if this change removes the level.
    #[must_use]
    pub fn is_removal(&self) -> bool {
        self.new_quantity.is_zero()
    }
}

/// Order book update (snapshot or delta).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OrderBookUpdate {
    /// Full snapshot.
    Snapshot(OrderBook),

    /// Incremental delta.
    Delta {
        /// Market address.
        market: String,
        /// Slot.
        slot: u64,
        /// Sequence number.
        sequence: u64,
        /// Changes.
        changes: Vec<BookChange>,
        /// Timestamp.
        timestamp: u64,
    },
}

impl OrderBookUpdate {
    /// Returns the market address.
    #[must_use]
    pub fn market(&self) -> &str {
        match self {
            Self::Snapshot(book) => &book.market,
            Self::Delta { market, .. } => market,
        }
    }

    /// Returns the sequence number.
    #[must_use]
    pub fn sequence(&self) -> u64 {
        match self {
            Self::Snapshot(book) => book.sequence,
            Self::Delta { sequence, .. } => *sequence,
        }
    }

    /// Returns true if this is a snapshot.
    #[must_use]
    pub const fn is_snapshot(&self) -> bool {
        matches!(self, Self::Snapshot(_))
    }

    /// Returns true if this is a delta.
    #[must_use]
    pub const fn is_delta(&self) -> bool {
        matches!(self, Self::Delta { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_book() -> OrderBook {
        OrderBook {
            market: "market123".to_string(),
            slot: 100,
            sequence: 1,
            bids: vec![
                BookLevel::new(Price::new(1000), Quantity::new(100), 2),
                BookLevel::new(Price::new(990), Quantity::new(200), 3),
            ],
            asks: vec![
                BookLevel::new(Price::new(1010), Quantity::new(150), 1),
                BookLevel::new(Price::new(1020), Quantity::new(250), 4),
            ],
            timestamp: 1234567890,
        }
    }

    #[test]
    fn test_book_level_new() {
        let level = BookLevel::new(Price::new(1000), Quantity::new(100), 2);
        assert_eq!(level.price.value(), 1000);
        assert_eq!(level.quantity.value(), 100);
        assert_eq!(level.order_count, 2);
    }

    #[test]
    fn test_book_level_is_empty() {
        let empty = BookLevel::new(Price::new(1000), Quantity::zero(), 0);
        assert!(empty.is_empty());

        let not_empty = BookLevel::new(Price::new(1000), Quantity::new(100), 1);
        assert!(!not_empty.is_empty());
    }

    #[test]
    fn test_book_level_display() {
        let level = BookLevel::new(Price::new(1000), Quantity::new(100), 2);
        assert_eq!(level.to_string(), "100 @ 1000 (2 orders)");
    }

    #[test]
    fn test_order_book_best_bid() {
        let book = create_test_book();
        assert_eq!(book.best_bid().map(|p| p.value()), Some(1000));
    }

    #[test]
    fn test_order_book_best_ask() {
        let book = create_test_book();
        assert_eq!(book.best_ask().map(|p| p.value()), Some(1010));
    }

    #[test]
    fn test_order_book_spread() {
        let book = create_test_book();
        assert_eq!(book.spread().map(|p| p.value()), Some(10));
    }

    #[test]
    fn test_order_book_midpoint() {
        let book = create_test_book();
        assert_eq!(book.midpoint().map(|p| p.value()), Some(1005));
    }

    #[test]
    fn test_order_book_is_crossed() {
        let mut book = create_test_book();
        assert!(!book.is_crossed());

        // Make it crossed
        book.bids[0].price = Price::new(1020);
        assert!(book.is_crossed());
    }

    #[test]
    fn test_order_book_total_quantity() {
        let book = create_test_book();
        assert_eq!(book.total_bid_quantity(10).value(), 300); // 100 + 200
        assert_eq!(book.total_ask_quantity(10).value(), 400); // 150 + 250
    }

    #[test]
    fn test_order_book_depth() {
        let book = create_test_book();
        assert_eq!(book.bid_depth(), 2);
        assert_eq!(book.ask_depth(), 2);
    }

    #[test]
    fn test_order_book_is_empty() {
        let book = create_test_book();
        assert!(!book.is_empty());

        let empty_book = OrderBook::new("market".to_string(), 0, 0, 0);
        assert!(empty_book.is_empty());
    }

    #[test]
    fn test_order_book_level_at() {
        let book = create_test_book();

        let bid_level = book.level_at(Side::Bid, Price::new(1000));
        assert!(bid_level.is_some());
        assert_eq!(bid_level.map(|l| l.quantity.value()), Some(100));

        let ask_level = book.level_at(Side::Ask, Price::new(1010));
        assert!(ask_level.is_some());

        let missing = book.level_at(Side::Bid, Price::new(999));
        assert!(missing.is_none());
    }

    #[test]
    fn test_book_change_new() {
        let change = BookChange::new(Side::Bid, Price::new(1000), Quantity::new(100), 2);
        assert_eq!(change.side, Side::Bid);
        assert_eq!(change.price.value(), 1000);
        assert!(!change.is_removal());
    }

    #[test]
    fn test_book_change_is_removal() {
        let removal = BookChange::new(Side::Bid, Price::new(1000), Quantity::zero(), 0);
        assert!(removal.is_removal());
    }

    #[test]
    fn test_order_book_update_snapshot() {
        let book = create_test_book();
        let update = OrderBookUpdate::Snapshot(book);

        assert!(update.is_snapshot());
        assert!(!update.is_delta());
        assert_eq!(update.market(), "market123");
        assert_eq!(update.sequence(), 1);
    }

    #[test]
    fn test_order_book_update_delta() {
        let update = OrderBookUpdate::Delta {
            market: "market123".to_string(),
            slot: 101,
            sequence: 2,
            changes: vec![BookChange::new(
                Side::Bid,
                Price::new(1000),
                Quantity::new(150),
                3,
            )],
            timestamp: 1234567891,
        };

        assert!(!update.is_snapshot());
        assert!(update.is_delta());
        assert_eq!(update.market(), "market123");
        assert_eq!(update.sequence(), 2);
    }

    #[test]
    fn test_order_book_serde() {
        let book = create_test_book();
        let json = serde_json::to_string(&book).expect("serialize");
        let parsed: OrderBook = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(parsed.market, book.market);
        assert_eq!(parsed.slot, book.slot);
        assert_eq!(parsed.bids.len(), book.bids.len());
    }

    #[test]
    fn test_order_book_update_serde() {
        let book = create_test_book();
        let update = OrderBookUpdate::Snapshot(book);

        let json = serde_json::to_string(&update).expect("serialize");
        assert!(json.contains("\"type\":\"snapshot\""));

        let parsed: OrderBookUpdate = serde_json::from_str(&json).expect("deserialize");
        assert!(parsed.is_snapshot());
    }
}
