//! Trade types for the Matchbook SDK.
//!
//! Provides trade-related types.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::primitives::{Price, Quantity, Side};

/// An executed trade.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trade {
    /// Trade ID.
    pub trade_id: String,

    /// Market address (base58 encoded).
    pub market: String,

    /// Price in ticks.
    pub price: Price,

    /// Quantity in lots.
    pub quantity: Quantity,

    /// Taker side.
    pub taker_side: Side,

    /// Maker order ID.
    pub maker_order_id: String,

    /// Taker order ID.
    pub taker_order_id: String,

    /// Maker address (base58 encoded).
    pub maker: String,

    /// Taker address (base58 encoded).
    pub taker: String,

    /// Slot when the trade occurred.
    pub slot: u64,

    /// Timestamp in milliseconds since epoch.
    pub timestamp: u64,

    /// Maker fee paid (negative for rebate).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maker_fee: Option<i64>,

    /// Taker fee paid.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taker_fee: Option<u64>,
}

impl Trade {
    /// Returns the buyer address.
    #[must_use]
    pub fn buyer(&self) -> &str {
        match self.taker_side {
            Side::Bid => &self.taker,
            Side::Ask => &self.maker,
        }
    }

    /// Returns the seller address.
    #[must_use]
    pub fn seller(&self) -> &str {
        match self.taker_side {
            Side::Bid => &self.maker,
            Side::Ask => &self.taker,
        }
    }

    /// Returns the notional value (price * quantity).
    #[must_use]
    pub fn notional(&self) -> u64 {
        self.price.value().saturating_mul(self.quantity.value())
    }
}

impl fmt::Display for Trade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Trade {} @ {} ({})",
            self.quantity, self.price, self.taker_side
        )
    }
}

/// Trade summary for a time period.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeSummary {
    /// Market address.
    pub market: String,

    /// Number of trades.
    pub trade_count: u64,

    /// Total volume in base units.
    pub volume: String,

    /// Total notional in quote units.
    pub notional: String,

    /// Volume-weighted average price.
    pub vwap: String,

    /// Start timestamp.
    pub start_time: u64,

    /// End timestamp.
    pub end_time: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_trade() -> Trade {
        Trade {
            trade_id: "trade123".to_string(),
            market: "market123".to_string(),
            price: Price::new(1000),
            quantity: Quantity::new(100),
            taker_side: Side::Bid,
            maker_order_id: "maker_order".to_string(),
            taker_order_id: "taker_order".to_string(),
            maker: "maker_address".to_string(),
            taker: "taker_address".to_string(),
            slot: 12345,
            timestamp: 1234567890,
            maker_fee: Some(-50),
            taker_fee: Some(100),
        }
    }

    #[test]
    fn test_trade_buyer_seller_bid() {
        let trade = create_test_trade();
        // Taker is bid (buyer), maker is ask (seller)
        assert_eq!(trade.buyer(), "taker_address");
        assert_eq!(trade.seller(), "maker_address");
    }

    #[test]
    fn test_trade_buyer_seller_ask() {
        let mut trade = create_test_trade();
        trade.taker_side = Side::Ask;
        // Taker is ask (seller), maker is bid (buyer)
        assert_eq!(trade.buyer(), "maker_address");
        assert_eq!(trade.seller(), "taker_address");
    }

    #[test]
    fn test_trade_notional() {
        let trade = create_test_trade();
        // price = 1000, quantity = 100
        assert_eq!(trade.notional(), 100_000);
    }

    #[test]
    fn test_trade_display() {
        let trade = create_test_trade();
        assert_eq!(trade.to_string(), "Trade 100 @ 1000 (bid)");
    }

    #[test]
    fn test_trade_serde() {
        let trade = create_test_trade();
        let json = serde_json::to_string(&trade).expect("serialize");
        let parsed: Trade = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(parsed.trade_id, trade.trade_id);
        assert_eq!(parsed.price.value(), trade.price.value());
        assert_eq!(parsed.taker_side, trade.taker_side);
    }

    #[test]
    fn test_trade_serde_without_fees() {
        let mut trade = create_test_trade();
        trade.maker_fee = None;
        trade.taker_fee = None;

        let json = serde_json::to_string(&trade).expect("serialize");
        assert!(!json.contains("makerFee"));
        assert!(!json.contains("takerFee"));
    }
}
