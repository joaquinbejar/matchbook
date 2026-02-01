//! Types for the event processor.
//!
//! Defines processed event structures and order update types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::book::Side;

/// Reason for order removal from the book.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutReason {
    /// Order was cancelled by user.
    Cancelled,
    /// Order was fully filled.
    Filled,
    /// Order expired (time in force).
    Expired,
    /// Order was evicted (e.g., self-trade prevention).
    Evicted,
    /// Unknown reason.
    Unknown,
}

impl OutReason {
    /// Creates an OutReason from a u8 value.
    #[must_use]
    pub const fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Cancelled,
            1 => Self::Filled,
            2 => Self::Expired,
            3 => Self::Evicted,
            _ => Self::Unknown,
        }
    }

    /// Returns a human-readable name.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
            Self::Filled => "filled",
            Self::Expired => "expired",
            Self::Evicted => "evicted",
            Self::Unknown => "unknown",
        }
    }
}

/// A processed fill event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedFill {
    /// Market address.
    pub market: [u8; 32],
    /// Maker order ID.
    pub maker_order_id: u128,
    /// Taker order ID.
    pub taker_order_id: u128,
    /// Maker address.
    pub maker_address: [u8; 32],
    /// Taker address.
    pub taker_address: [u8; 32],
    /// Execution price.
    pub price: u64,
    /// Executed quantity.
    pub quantity: u64,
    /// Taker side.
    pub taker_side: Side,
    /// Taker fee paid.
    pub taker_fee: u64,
    /// Maker rebate received.
    pub maker_rebate: u64,
    /// Solana slot.
    pub slot: u64,
    /// Event sequence number.
    pub seq_num: u64,
    /// Processing timestamp.
    pub timestamp: DateTime<Utc>,
}

impl ProcessedFill {
    /// Returns the maker address as a base58 string.
    #[must_use]
    pub fn maker_address_string(&self) -> String {
        bs58::encode(&self.maker_address).into_string()
    }

    /// Returns the taker address as a base58 string.
    #[must_use]
    pub fn taker_address_string(&self) -> String {
        bs58::encode(&self.taker_address).into_string()
    }

    /// Returns the market as a base58 string.
    #[must_use]
    pub fn market_string(&self) -> String {
        bs58::encode(&self.market).into_string()
    }

    /// Returns the notional value (price * quantity).
    #[must_use]
    pub fn notional(&self) -> u128 {
        (self.price as u128).saturating_mul(self.quantity as u128)
    }
}

/// A processed out (cancellation) event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedOut {
    /// Market address.
    pub market: [u8; 32],
    /// Order ID.
    pub order_id: u128,
    /// Owner address.
    pub owner: [u8; 32],
    /// Reason for removal.
    pub reason: OutReason,
    /// Remaining quantity when removed.
    pub remaining_quantity: u64,
    /// Solana slot.
    pub slot: u64,
    /// Event sequence number.
    pub seq_num: u64,
    /// Processing timestamp.
    pub timestamp: DateTime<Utc>,
}

impl ProcessedOut {
    /// Returns the owner as a base58 string.
    #[must_use]
    pub fn owner_string(&self) -> String {
        bs58::encode(&self.owner).into_string()
    }

    /// Returns the market as a base58 string.
    #[must_use]
    pub fn market_string(&self) -> String {
        bs58::encode(&self.market).into_string()
    }

    /// Returns true if the order was cancelled (not filled).
    #[must_use]
    pub const fn is_cancellation(&self) -> bool {
        matches!(
            self.reason,
            OutReason::Cancelled | OutReason::Expired | OutReason::Evicted
        )
    }
}

/// An order update for publishing to clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderUpdate {
    /// Order was filled (partially or fully).
    Filled {
        /// Market address.
        market: [u8; 32],
        /// Order ID.
        order_id: u128,
        /// Filled quantity.
        filled_quantity: u64,
        /// Remaining quantity.
        remaining_quantity: u64,
        /// Fill price.
        fill_price: u64,
        /// Sequence number.
        seq_num: u64,
    },

    /// Order was cancelled/removed.
    Cancelled {
        /// Market address.
        market: [u8; 32],
        /// Order ID.
        order_id: u128,
        /// Cancellation reason.
        reason: OutReason,
        /// Sequence number.
        seq_num: u64,
    },
}

impl OrderUpdate {
    /// Returns the market address.
    #[must_use]
    pub const fn market(&self) -> &[u8; 32] {
        match self {
            Self::Filled { market, .. } | Self::Cancelled { market, .. } => market,
        }
    }

    /// Returns the order ID.
    #[must_use]
    pub const fn order_id(&self) -> u128 {
        match self {
            Self::Filled { order_id, .. } | Self::Cancelled { order_id, .. } => *order_id,
        }
    }

    /// Returns the sequence number.
    #[must_use]
    pub const fn seq_num(&self) -> u64 {
        match self {
            Self::Filled { seq_num, .. } | Self::Cancelled { seq_num, .. } => *seq_num,
        }
    }

    /// Returns true if this is a fill update.
    #[must_use]
    pub const fn is_fill(&self) -> bool {
        matches!(self, Self::Filled { .. })
    }

    /// Returns true if this is a cancellation update.
    #[must_use]
    pub const fn is_cancellation(&self) -> bool {
        matches!(self, Self::Cancelled { .. })
    }
}

/// Result of processing events.
#[derive(Debug, Clone)]
pub struct ProcessingResult {
    /// Number of events processed.
    pub events_processed: usize,
    /// Number of fills.
    pub fills: usize,
    /// Number of outs (cancellations).
    pub outs: usize,
    /// Number of events skipped (already processed).
    pub skipped: usize,
    /// Processed fills.
    pub processed_fills: Vec<ProcessedFill>,
    /// Processed outs.
    pub processed_outs: Vec<ProcessedOut>,
    /// Order updates for publishing.
    pub order_updates: Vec<OrderUpdate>,
}

impl ProcessingResult {
    /// Creates an empty result.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            events_processed: 0,
            fills: 0,
            outs: 0,
            skipped: 0,
            processed_fills: Vec::new(),
            processed_outs: Vec::new(),
            order_updates: Vec::new(),
        }
    }

    /// Returns true if no events were processed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events_processed == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_out_reason_from_u8() {
        assert_eq!(OutReason::from_u8(0), OutReason::Cancelled);
        assert_eq!(OutReason::from_u8(1), OutReason::Filled);
        assert_eq!(OutReason::from_u8(2), OutReason::Expired);
        assert_eq!(OutReason::from_u8(3), OutReason::Evicted);
        assert_eq!(OutReason::from_u8(255), OutReason::Unknown);
    }

    #[test]
    fn test_out_reason_as_str() {
        assert_eq!(OutReason::Cancelled.as_str(), "cancelled");
        assert_eq!(OutReason::Filled.as_str(), "filled");
        assert_eq!(OutReason::Expired.as_str(), "expired");
        assert_eq!(OutReason::Evicted.as_str(), "evicted");
        assert_eq!(OutReason::Unknown.as_str(), "unknown");
    }

    #[test]
    fn test_processed_fill_notional() {
        let fill = ProcessedFill {
            market: [0u8; 32],
            maker_order_id: 1,
            taker_order_id: 2,
            maker_address: [0u8; 32],
            taker_address: [0u8; 32],
            price: 1000,
            quantity: 100,
            taker_side: Side::Bid,
            taker_fee: 30,
            maker_rebate: 10,
            slot: 100,
            seq_num: 1,
            timestamp: Utc::now(),
        };

        assert_eq!(fill.notional(), 100_000);
    }

    #[test]
    fn test_processed_out_is_cancellation() {
        let cancelled = ProcessedOut {
            market: [0u8; 32],
            order_id: 1,
            owner: [0u8; 32],
            reason: OutReason::Cancelled,
            remaining_quantity: 50,
            slot: 100,
            seq_num: 1,
            timestamp: Utc::now(),
        };

        let filled = ProcessedOut {
            reason: OutReason::Filled,
            ..cancelled.clone()
        };

        assert!(cancelled.is_cancellation());
        assert!(!filled.is_cancellation());
    }

    #[test]
    fn test_order_update_filled() {
        let update = OrderUpdate::Filled {
            market: [1u8; 32],
            order_id: 123,
            filled_quantity: 50,
            remaining_quantity: 50,
            fill_price: 1000,
            seq_num: 1,
        };

        assert!(update.is_fill());
        assert!(!update.is_cancellation());
        assert_eq!(update.order_id(), 123);
        assert_eq!(update.seq_num(), 1);
    }

    #[test]
    fn test_order_update_cancelled() {
        let update = OrderUpdate::Cancelled {
            market: [1u8; 32],
            order_id: 456,
            reason: OutReason::Cancelled,
            seq_num: 2,
        };

        assert!(update.is_cancellation());
        assert!(!update.is_fill());
        assert_eq!(update.order_id(), 456);
        assert_eq!(update.seq_num(), 2);
    }

    #[test]
    fn test_processing_result_empty() {
        let result = ProcessingResult::empty();
        assert!(result.is_empty());
        assert_eq!(result.events_processed, 0);
        assert_eq!(result.fills, 0);
        assert_eq!(result.outs, 0);
    }
}
