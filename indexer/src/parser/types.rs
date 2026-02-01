//! Types for the account parser.
//!
//! Defines parsed account structures and error types.

use serde::{Deserialize, Serialize};

/// Parsed account types.
#[derive(Debug, Clone)]
pub enum ParsedAccount {
    /// Parsed Market account.
    Market(ParsedMarket),

    /// Parsed OrderBook side (Bids or Asks).
    OrderBookSide {
        /// Orders in the book.
        orders: Vec<ParsedOrder>,
    },

    /// Parsed EventQueue.
    EventQueue {
        /// Events in the queue.
        events: Vec<ParsedEvent>,
    },

    /// Parsed OpenOrders account.
    OpenOrders(ParsedOpenOrders),

    /// Unknown account type.
    Unknown {
        /// The unrecognized discriminator.
        discriminator: [u8; 8],
    },
}

/// Parsed Market account data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedMarket {
    /// Base token mint address.
    pub base_mint: [u8; 32],
    /// Quote token mint address.
    pub quote_mint: [u8; 32],
    /// Base lot size in native units.
    pub base_lot_size: u64,
    /// Quote lot size in native units.
    pub quote_lot_size: u64,
    /// Tick size in quote lots.
    pub tick_size: u64,
    /// Taker fee in basis points.
    pub taker_fee_bps: u16,
    /// Maker fee in basis points (negative = rebate).
    pub maker_fee_bps: i16,
    /// Market status.
    pub status: u8,
}

impl ParsedMarket {
    /// Returns the base mint as a base58 string.
    #[must_use]
    pub fn base_mint_string(&self) -> String {
        bs58::encode(&self.base_mint).into_string()
    }

    /// Returns the quote mint as a base58 string.
    #[must_use]
    pub fn quote_mint_string(&self) -> String {
        bs58::encode(&self.quote_mint).into_string()
    }
}

/// Parsed order from the order book.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedOrder {
    /// Order ID (encodes price and sequence).
    pub order_id: u128,
    /// Owner address.
    pub owner: [u8; 32],
    /// Price in quote lots per base lot.
    pub price: u64,
    /// Quantity in base lots.
    pub quantity: u64,
    /// Client-provided order ID.
    pub client_order_id: u64,
}

impl ParsedOrder {
    /// Returns the owner as a base58 string.
    #[must_use]
    pub fn owner_string(&self) -> String {
        bs58::encode(&self.owner).into_string()
    }
}

/// Event type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    /// Fill event (trade executed).
    Fill,
    /// Out event (order removed from book).
    Out,
}

/// Parsed event from the event queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedEvent {
    /// Type of event.
    pub event_type: EventType,
}

/// Order side enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    /// Buy order.
    Bid,
    /// Sell order.
    Ask,
}

/// Parsed OpenOrders account data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedOpenOrders {
    /// Owner address.
    pub owner: [u8; 32],
    /// Market address.
    pub market: [u8; 32],
    /// Available base token balance.
    pub base_free: u64,
    /// Locked base token balance.
    pub base_locked: u64,
    /// Available quote token balance.
    pub quote_free: u64,
    /// Locked quote token balance.
    pub quote_locked: u64,
}

impl ParsedOpenOrders {
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

    /// Returns the total base balance (free + locked).
    #[must_use]
    pub fn total_base(&self) -> u64 {
        self.base_free.saturating_add(self.base_locked)
    }

    /// Returns the total quote balance (free + locked).
    #[must_use]
    pub fn total_quote(&self) -> u64 {
        self.quote_free.saturating_add(self.quote_locked)
    }
}

/// Parse errors.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ParseError {
    /// Data is too short.
    #[error("Data too short: expected at least {expected} bytes, got {actual}")]
    DataTooShort {
        /// Expected minimum size.
        expected: usize,
        /// Actual size.
        actual: usize,
    },

    /// Invalid discriminator.
    #[error("Invalid account discriminator")]
    InvalidDiscriminator,

    /// Invalid data format.
    #[error("Invalid data for field: {0}")]
    InvalidData(String),

    /// Unsupported version.
    #[error("Unsupported account version: {0}")]
    UnsupportedVersion(u8),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsed_market_mint_strings() {
        let market = ParsedMarket {
            base_mint: [0u8; 32],
            quote_mint: [1u8; 32],
            base_lot_size: 1000,
            quote_lot_size: 100,
            tick_size: 10,
            taker_fee_bps: 30,
            maker_fee_bps: -10,
            status: 1,
        };

        assert_eq!(
            market.base_mint_string(),
            "11111111111111111111111111111111"
        );
        assert!(!market.quote_mint_string().is_empty());
    }

    #[test]
    fn test_parsed_order_owner_string() {
        let order = ParsedOrder {
            order_id: 12345,
            owner: [0u8; 32],
            price: 1000,
            quantity: 100,
            client_order_id: 1,
        };

        assert_eq!(order.owner_string(), "11111111111111111111111111111111");
    }

    #[test]
    fn test_parsed_open_orders_totals() {
        let oo = ParsedOpenOrders {
            owner: [0u8; 32],
            market: [0u8; 32],
            base_free: 100,
            base_locked: 50,
            quote_free: 1000,
            quote_locked: 500,
        };

        assert_eq!(oo.total_base(), 150);
        assert_eq!(oo.total_quote(), 1500);
    }

    #[test]
    fn test_event_type_equality() {
        assert_eq!(EventType::Fill, EventType::Fill);
        assert_ne!(EventType::Fill, EventType::Out);
    }

    #[test]
    fn test_side_equality() {
        assert_eq!(Side::Bid, Side::Bid);
        assert_ne!(Side::Bid, Side::Ask);
    }

    #[test]
    fn test_parse_error_display() {
        let err = ParseError::DataTooShort {
            expected: 100,
            actual: 50,
        };
        assert_eq!(
            err.to_string(),
            "Data too short: expected at least 100 bytes, got 50"
        );

        let err = ParseError::InvalidData("test".to_string());
        assert_eq!(err.to_string(), "Invalid data for field: test");
    }
}
