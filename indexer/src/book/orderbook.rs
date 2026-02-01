//! Full order book implementation.
//!
//! Maintains the complete order book state for a single market.

use std::collections::BTreeMap;

use super::types::{BookChange, PriceLevel, Side};
use crate::parser::ParsedOrder;

/// A full order book for a single market.
///
/// Maintains bid and ask levels with aggregated quantities.
#[derive(Debug, Clone)]
pub struct FullOrderBook {
    /// Market address.
    pub market: [u8; 32],

    /// Bid levels (price -> level).
    /// Stored in ascending order, but accessed in descending order for best bid.
    bids: BTreeMap<u64, PriceLevel>,

    /// Ask levels (price -> level).
    /// Stored in ascending order for best ask.
    asks: BTreeMap<u64, PriceLevel>,

    /// Last slot when the book was updated.
    pub last_slot: u64,

    /// Sequence number for ordering updates.
    pub seq: u64,
}

impl FullOrderBook {
    /// Creates a new empty order book.
    #[must_use]
    pub fn new(market: [u8; 32]) -> Self {
        Self {
            market,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            last_slot: 0,
            seq: 0,
        }
    }

    /// Returns the best bid (highest price).
    #[must_use]
    pub fn best_bid(&self) -> Option<&PriceLevel> {
        self.bids.values().next_back()
    }

    /// Returns the best ask (lowest price).
    #[must_use]
    pub fn best_ask(&self) -> Option<&PriceLevel> {
        self.asks.values().next()
    }

    /// Returns the spread (best ask - best bid).
    ///
    /// Returns `None` if either side is empty.
    #[must_use]
    pub fn spread(&self) -> Option<u64> {
        let best_bid = self.best_bid()?.price;
        let best_ask = self.best_ask()?.price;
        Some(best_ask.saturating_sub(best_bid))
    }

    /// Returns the mid price ((best bid + best ask) / 2).
    ///
    /// Returns `None` if either side is empty.
    #[must_use]
    pub fn mid_price(&self) -> Option<u64> {
        let best_bid = self.best_bid()?.price;
        let best_ask = self.best_ask()?.price;
        Some((best_bid.saturating_add(best_ask)) / 2)
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

    /// Returns the total depth (bid + ask levels).
    #[must_use]
    pub fn total_depth(&self) -> usize {
        self.bids.len() + self.asks.len()
    }

    /// Returns true if the book is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bids.is_empty() && self.asks.is_empty()
    }

    /// Returns aggregated bid levels (highest price first).
    ///
    /// # Arguments
    ///
    /// * `depth` - Maximum number of levels to return (0 = all)
    #[must_use]
    pub fn aggregate_bids(&self, depth: usize) -> Vec<PriceLevel> {
        let iter = self.bids.values().rev();
        if depth == 0 {
            iter.cloned().collect()
        } else {
            iter.take(depth).cloned().collect()
        }
    }

    /// Returns aggregated ask levels (lowest price first).
    ///
    /// # Arguments
    ///
    /// * `depth` - Maximum number of levels to return (0 = all)
    #[must_use]
    pub fn aggregate_asks(&self, depth: usize) -> Vec<PriceLevel> {
        let iter = self.asks.values();
        if depth == 0 {
            iter.cloned().collect()
        } else {
            iter.take(depth).cloned().collect()
        }
    }

    /// Applies a list of orders to one side of the book.
    ///
    /// Replaces the entire side with the new orders and computes the delta.
    ///
    /// # Arguments
    ///
    /// * `side` - Which side to update
    /// * `orders` - New orders for this side
    /// * `slot` - Slot number of the update
    ///
    /// # Returns
    ///
    /// List of changes (added, modified, removed levels).
    pub fn apply_orders(
        &mut self,
        side: Side,
        orders: &[ParsedOrder],
        slot: u64,
    ) -> Vec<BookChange> {
        // Build new levels from orders
        let mut new_levels: BTreeMap<u64, PriceLevel> = BTreeMap::new();

        for order in orders {
            let entry = new_levels.entry(order.price).or_insert(PriceLevel {
                price: order.price,
                quantity: 0,
                order_count: 0,
            });
            entry.quantity = entry.quantity.saturating_add(order.quantity);
            entry.order_count = entry.order_count.saturating_add(1);
        }

        // Get reference to the appropriate side
        let old_levels = match side {
            Side::Bid => &self.bids,
            Side::Ask => &self.asks,
        };

        // Compute delta
        let mut changes = Vec::new();

        // Find removed and modified levels
        for (price, old_level) in old_levels {
            match new_levels.get(price) {
                Some(new_level) => {
                    if old_level.quantity != new_level.quantity
                        || old_level.order_count != new_level.order_count
                    {
                        changes.push(BookChange::new(
                            side,
                            *price,
                            new_level.quantity,
                            new_level.order_count,
                        ));
                    }
                }
                None => {
                    // Level removed
                    changes.push(BookChange::new(side, *price, 0, 0));
                }
            }
        }

        // Find added levels
        for (price, new_level) in &new_levels {
            if !old_levels.contains_key(price) {
                changes.push(BookChange::new(
                    side,
                    *price,
                    new_level.quantity,
                    new_level.order_count,
                ));
            }
        }

        // Update the book
        match side {
            Side::Bid => self.bids = new_levels,
            Side::Ask => self.asks = new_levels,
        }

        self.last_slot = slot;
        self.seq = self.seq.saturating_add(1);

        changes
    }

    /// Clears all levels from the book.
    pub fn clear(&mut self) {
        self.bids.clear();
        self.asks.clear();
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
    fn test_new_book() {
        let book = FullOrderBook::new([1u8; 32]);
        assert!(book.is_empty());
        assert_eq!(book.bid_depth(), 0);
        assert_eq!(book.ask_depth(), 0);
        assert!(book.best_bid().is_none());
        assert!(book.best_ask().is_none());
    }

    #[test]
    fn test_apply_orders_bids() {
        let mut book = FullOrderBook::new([1u8; 32]);

        let orders = vec![
            create_order(1000, 100),
            create_order(1001, 200),
            create_order(1000, 50), // Same price, should aggregate
        ];

        let changes = book.apply_orders(Side::Bid, &orders, 100);

        assert_eq!(book.bid_depth(), 2);
        assert_eq!(changes.len(), 2); // Two new levels

        let best = book.best_bid().expect("Should have best bid");
        assert_eq!(best.price, 1001);
        assert_eq!(best.quantity, 200);
    }

    #[test]
    fn test_apply_orders_asks() {
        let mut book = FullOrderBook::new([1u8; 32]);

        let orders = vec![create_order(1002, 100), create_order(1003, 200)];

        let changes = book.apply_orders(Side::Ask, &orders, 100);

        assert_eq!(book.ask_depth(), 2);
        assert_eq!(changes.len(), 2);

        let best = book.best_ask().expect("Should have best ask");
        assert_eq!(best.price, 1002);
        assert_eq!(best.quantity, 100);
    }

    #[test]
    fn test_spread() {
        let mut book = FullOrderBook::new([1u8; 32]);

        // No spread when empty
        assert!(book.spread().is_none());

        book.apply_orders(Side::Bid, &[create_order(1000, 100)], 100);
        book.apply_orders(Side::Ask, &[create_order(1005, 100)], 101);

        assert_eq!(book.spread(), Some(5));
    }

    #[test]
    fn test_mid_price() {
        let mut book = FullOrderBook::new([1u8; 32]);

        book.apply_orders(Side::Bid, &[create_order(1000, 100)], 100);
        book.apply_orders(Side::Ask, &[create_order(1010, 100)], 101);

        assert_eq!(book.mid_price(), Some(1005));
    }

    #[test]
    fn test_aggregate_bids() {
        let mut book = FullOrderBook::new([1u8; 32]);

        let orders = vec![
            create_order(1000, 100),
            create_order(1001, 200),
            create_order(1002, 300),
        ];

        book.apply_orders(Side::Bid, &orders, 100);

        // All levels
        let all = book.aggregate_bids(0);
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].price, 1002); // Highest first

        // Limited depth
        let limited = book.aggregate_bids(2);
        assert_eq!(limited.len(), 2);
        assert_eq!(limited[0].price, 1002);
        assert_eq!(limited[1].price, 1001);
    }

    #[test]
    fn test_aggregate_asks() {
        let mut book = FullOrderBook::new([1u8; 32]);

        let orders = vec![
            create_order(1000, 100),
            create_order(1001, 200),
            create_order(1002, 300),
        ];

        book.apply_orders(Side::Ask, &orders, 100);

        // All levels
        let all = book.aggregate_asks(0);
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].price, 1000); // Lowest first

        // Limited depth
        let limited = book.aggregate_asks(2);
        assert_eq!(limited.len(), 2);
        assert_eq!(limited[0].price, 1000);
        assert_eq!(limited[1].price, 1001);
    }

    #[test]
    fn test_delta_computation() {
        let mut book = FullOrderBook::new([1u8; 32]);

        // Initial state
        let orders1 = vec![create_order(1000, 100), create_order(1001, 200)];
        book.apply_orders(Side::Bid, &orders1, 100);

        // Update: modify 1000, remove 1001, add 1002
        let orders2 = vec![create_order(1000, 150), create_order(1002, 300)];
        let changes = book.apply_orders(Side::Bid, &orders2, 101);

        assert_eq!(changes.len(), 3);

        // Find each change
        let modified = changes.iter().find(|c| c.price == 1000);
        let removed = changes.iter().find(|c| c.price == 1001);
        let added = changes.iter().find(|c| c.price == 1002);

        assert!(modified.is_some());
        assert_eq!(modified.expect("modified").new_quantity, 150);

        assert!(removed.is_some());
        assert!(removed.expect("removed").is_removal());

        assert!(added.is_some());
        assert_eq!(added.expect("added").new_quantity, 300);
    }

    #[test]
    fn test_clear() {
        let mut book = FullOrderBook::new([1u8; 32]);

        book.apply_orders(Side::Bid, &[create_order(1000, 100)], 100);
        book.apply_orders(Side::Ask, &[create_order(1001, 100)], 101);

        assert!(!book.is_empty());

        book.clear();

        assert!(book.is_empty());
    }

    #[test]
    fn test_sequence_increments() {
        let mut book = FullOrderBook::new([1u8; 32]);
        assert_eq!(book.seq, 0);

        book.apply_orders(Side::Bid, &[create_order(1000, 100)], 100);
        assert_eq!(book.seq, 1);

        book.apply_orders(Side::Ask, &[create_order(1001, 100)], 101);
        assert_eq!(book.seq, 2);
    }
}
