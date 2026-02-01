//! Types for the book builder.
//!
//! Defines price levels, book changes, and update types.

use serde::{Deserialize, Serialize};

/// Order side enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Side {
    /// Buy side (bids).
    Bid,
    /// Sell side (asks).
    Ask,
}

impl Side {
    /// Returns the opposite side.
    #[must_use]
    pub const fn opposite(&self) -> Self {
        match self {
            Self::Bid => Self::Ask,
            Self::Ask => Self::Bid,
        }
    }

    /// Returns a human-readable name.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Bid => "bid",
            Self::Ask => "ask",
        }
    }
}

/// A price level in the order book.
///
/// Represents aggregated quantity at a specific price.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceLevel {
    /// Price in quote lots per base lot.
    pub price: u64,
    /// Total quantity at this price in base lots.
    pub quantity: u64,
    /// Number of orders at this price.
    pub order_count: u32,
}

impl PriceLevel {
    /// Creates a new price level.
    #[must_use]
    pub const fn new(price: u64, quantity: u64, order_count: u32) -> Self {
        Self {
            price,
            quantity,
            order_count,
        }
    }

    /// Returns true if this level is empty (no quantity).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.quantity == 0
    }
}

/// A change to a price level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookChange {
    /// Side of the change.
    pub side: Side,
    /// Price level that changed.
    pub price: u64,
    /// New quantity at this price (0 = level removed).
    pub new_quantity: u64,
    /// New order count at this price.
    pub order_count: u32,
}

impl BookChange {
    /// Creates a new book change.
    #[must_use]
    pub const fn new(side: Side, price: u64, new_quantity: u64, order_count: u32) -> Self {
        Self {
            side,
            price,
            new_quantity,
            order_count,
        }
    }

    /// Returns true if this change removes the level.
    #[must_use]
    pub const fn is_removal(&self) -> bool {
        self.new_quantity == 0
    }
}

/// An order book update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BookUpdate {
    /// Full snapshot of the order book.
    Snapshot {
        /// Market address.
        market: [u8; 32],
        /// Bid levels (highest price first).
        bids: Vec<PriceLevel>,
        /// Ask levels (lowest price first).
        asks: Vec<PriceLevel>,
        /// Slot number.
        slot: u64,
        /// Sequence number.
        seq: u64,
    },

    /// Incremental update to the order book.
    Delta {
        /// Market address.
        market: [u8; 32],
        /// List of changes.
        changes: Vec<BookChange>,
        /// Slot number.
        slot: u64,
        /// Sequence number.
        seq: u64,
    },
}

impl BookUpdate {
    /// Returns the market address.
    #[must_use]
    pub const fn market(&self) -> &[u8; 32] {
        match self {
            Self::Snapshot { market, .. } | Self::Delta { market, .. } => market,
        }
    }

    /// Returns the slot number.
    #[must_use]
    pub const fn slot(&self) -> u64 {
        match self {
            Self::Snapshot { slot, .. } | Self::Delta { slot, .. } => *slot,
        }
    }

    /// Returns the sequence number.
    #[must_use]
    pub const fn seq(&self) -> u64 {
        match self {
            Self::Snapshot { seq, .. } | Self::Delta { seq, .. } => *seq,
        }
    }

    /// Returns true if this is a snapshot.
    #[must_use]
    pub const fn is_snapshot(&self) -> bool {
        matches!(self, Self::Snapshot { .. })
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

    #[test]
    fn test_side_opposite() {
        assert_eq!(Side::Bid.opposite(), Side::Ask);
        assert_eq!(Side::Ask.opposite(), Side::Bid);
    }

    #[test]
    fn test_side_as_str() {
        assert_eq!(Side::Bid.as_str(), "bid");
        assert_eq!(Side::Ask.as_str(), "ask");
    }

    #[test]
    fn test_price_level_new() {
        let level = PriceLevel::new(1000, 500, 3);
        assert_eq!(level.price, 1000);
        assert_eq!(level.quantity, 500);
        assert_eq!(level.order_count, 3);
    }

    #[test]
    fn test_price_level_is_empty() {
        let empty = PriceLevel::new(1000, 0, 0);
        let non_empty = PriceLevel::new(1000, 100, 1);

        assert!(empty.is_empty());
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn test_book_change_new() {
        let change = BookChange::new(Side::Bid, 1000, 500, 3);
        assert_eq!(change.side, Side::Bid);
        assert_eq!(change.price, 1000);
        assert_eq!(change.new_quantity, 500);
        assert_eq!(change.order_count, 3);
    }

    #[test]
    fn test_book_change_is_removal() {
        let removal = BookChange::new(Side::Bid, 1000, 0, 0);
        let update = BookChange::new(Side::Bid, 1000, 100, 1);

        assert!(removal.is_removal());
        assert!(!update.is_removal());
    }

    #[test]
    fn test_book_update_snapshot() {
        let update = BookUpdate::Snapshot {
            market: [1u8; 32],
            bids: vec![PriceLevel::new(1000, 100, 1)],
            asks: vec![PriceLevel::new(1001, 100, 1)],
            slot: 100,
            seq: 1,
        };

        assert!(update.is_snapshot());
        assert!(!update.is_delta());
        assert_eq!(update.slot(), 100);
        assert_eq!(update.seq(), 1);
    }

    #[test]
    fn test_book_update_delta() {
        let update = BookUpdate::Delta {
            market: [1u8; 32],
            changes: vec![BookChange::new(Side::Bid, 1000, 200, 2)],
            slot: 101,
            seq: 2,
        };

        assert!(update.is_delta());
        assert!(!update.is_snapshot());
        assert_eq!(update.slot(), 101);
        assert_eq!(update.seq(), 2);
    }
}
