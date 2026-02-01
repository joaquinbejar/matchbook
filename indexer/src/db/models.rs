//! Database models for Matchbook indexer.
//!
//! These structs map directly to database tables and are used for
//! querying and inserting data.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Market status enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, sqlx::Type)]
#[repr(i16)]
pub enum MarketStatus {
    /// Market is inactive (not yet started).
    #[default]
    Inactive = 0,
    /// Market is active and accepting orders.
    Active = 1,
    /// Market is paused (no new orders).
    Paused = 2,
    /// Market is closed permanently.
    Closed = 3,
}

/// Order side enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[repr(i16)]
pub enum Side {
    /// Buy order.
    Bid = 0,
    /// Sell order.
    Ask = 1,
}

/// Order status enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, sqlx::Type)]
#[repr(i16)]
pub enum OrderStatus {
    /// Order is open and on the book.
    #[default]
    Open = 0,
    /// Order is partially filled.
    PartiallyFilled = 1,
    /// Order is completely filled.
    Filled = 2,
    /// Order was cancelled.
    Cancelled = 3,
    /// Order expired.
    Expired = 4,
}

/// Order type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, sqlx::Type)]
#[repr(i16)]
pub enum OrderType {
    /// Standard limit order.
    #[default]
    Limit = 0,
    /// Post-only order (maker only).
    PostOnly = 1,
    /// Immediate-or-cancel order.
    ImmediateOrCancel = 2,
    /// Fill-or-kill order.
    FillOrKill = 3,
}

/// Time in force enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, sqlx::Type)]
#[repr(i16)]
pub enum TimeInForce {
    /// Good till cancelled.
    #[default]
    GoodTillCancelled = 0,
    /// Immediate or cancel.
    ImmediateOrCancel = 1,
    /// Fill or kill.
    FillOrKill = 2,
    /// Post only.
    PostOnly = 3,
}

/// Market record from the database.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Market {
    /// Database ID.
    pub id: i32,
    /// Solana market account address.
    pub address: String,
    /// Base token mint address.
    pub base_mint: String,
    /// Quote token mint address.
    pub quote_mint: String,
    /// Base token decimals.
    pub base_decimals: i16,
    /// Quote token decimals.
    pub quote_decimals: i16,
    /// Base lot size in native units.
    pub base_lot_size: i64,
    /// Quote lot size in native units.
    pub quote_lot_size: i64,
    /// Tick size in quote lots.
    pub tick_size: i64,
    /// Minimum order size in base lots.
    pub min_order_size: i64,
    /// Taker fee in basis points.
    pub taker_fee_bps: i16,
    /// Maker fee in basis points (negative = rebate).
    pub maker_fee_bps: i16,
    /// Market status.
    pub status: i16,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// New market for insertion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMarket {
    /// Solana market account address.
    pub address: String,
    /// Base token mint address.
    pub base_mint: String,
    /// Quote token mint address.
    pub quote_mint: String,
    /// Base token decimals.
    pub base_decimals: i16,
    /// Quote token decimals.
    pub quote_decimals: i16,
    /// Base lot size in native units.
    pub base_lot_size: i64,
    /// Quote lot size in native units.
    pub quote_lot_size: i64,
    /// Tick size in quote lots.
    pub tick_size: i64,
    /// Minimum order size in base lots.
    pub min_order_size: i64,
    /// Taker fee in basis points.
    pub taker_fee_bps: i16,
    /// Maker fee in basis points (negative = rebate).
    pub maker_fee_bps: i16,
    /// Market status.
    pub status: i16,
}

/// Trade record from the database.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Trade {
    /// Database ID.
    pub id: i64,
    /// Market ID (foreign key).
    pub market_id: i32,
    /// Maker order ID.
    pub maker_order_id: Decimal,
    /// Taker order ID.
    pub taker_order_id: Decimal,
    /// Maker address.
    pub maker_address: String,
    /// Taker address.
    pub taker_address: String,
    /// Execution price in quote lots per base lot.
    pub price: i64,
    /// Executed quantity in base lots.
    pub quantity: i64,
    /// Taker side (0=Bid, 1=Ask).
    pub taker_side: i16,
    /// Taker fee in quote token native units.
    pub taker_fee: i64,
    /// Maker rebate in quote token native units.
    pub maker_rebate: i64,
    /// Solana slot.
    pub slot: i64,
    /// Event queue sequence number.
    pub seq_num: i64,
    /// Trade timestamp.
    pub timestamp: DateTime<Utc>,
}

/// New trade for insertion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTrade {
    /// Market ID (foreign key).
    pub market_id: i32,
    /// Maker order ID.
    pub maker_order_id: Decimal,
    /// Taker order ID.
    pub taker_order_id: Decimal,
    /// Maker address.
    pub maker_address: String,
    /// Taker address.
    pub taker_address: String,
    /// Execution price in quote lots per base lot.
    pub price: i64,
    /// Executed quantity in base lots.
    pub quantity: i64,
    /// Taker side (0=Bid, 1=Ask).
    pub taker_side: i16,
    /// Taker fee in quote token native units.
    pub taker_fee: i64,
    /// Maker rebate in quote token native units.
    pub maker_rebate: i64,
    /// Solana slot.
    pub slot: i64,
    /// Event queue sequence number.
    pub seq_num: i64,
    /// Trade timestamp.
    pub timestamp: DateTime<Utc>,
}

/// Order record from the database.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Order {
    /// Database ID.
    pub id: i64,
    /// Market ID (foreign key).
    pub market_id: i32,
    /// On-chain order ID.
    pub order_id: Decimal,
    /// Owner address.
    pub owner: String,
    /// Order side (0=Bid, 1=Ask).
    pub side: i16,
    /// Limit price in quote lots per base lot.
    pub price: i64,
    /// Original order quantity in base lots.
    pub original_quantity: i64,
    /// Cumulative filled quantity in base lots.
    pub filled_quantity: i64,
    /// Order status.
    pub status: i16,
    /// Order type.
    pub order_type: i16,
    /// Time in force.
    pub time_in_force: i16,
    /// Client-provided order ID.
    pub client_order_id: Option<i64>,
    /// Solana slot when order was placed.
    pub slot: i64,
    /// Placement timestamp.
    pub placed_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// New order for insertion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewOrder {
    /// Market ID (foreign key).
    pub market_id: i32,
    /// On-chain order ID.
    pub order_id: Decimal,
    /// Owner address.
    pub owner: String,
    /// Order side (0=Bid, 1=Ask).
    pub side: i16,
    /// Limit price in quote lots per base lot.
    pub price: i64,
    /// Original order quantity in base lots.
    pub original_quantity: i64,
    /// Order type.
    pub order_type: i16,
    /// Time in force.
    pub time_in_force: i16,
    /// Client-provided order ID.
    pub client_order_id: Option<i64>,
    /// Solana slot when order was placed.
    pub slot: i64,
    /// Placement timestamp.
    pub placed_at: DateTime<Utc>,
}

/// Balance record from the database.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Balance {
    /// Database ID.
    pub id: i64,
    /// Market ID (foreign key).
    pub market_id: i32,
    /// Owner address.
    pub owner: String,
    /// Available base token balance in native units.
    pub base_free: i64,
    /// Locked base token balance in native units.
    pub base_locked: i64,
    /// Available quote token balance in native units.
    pub quote_free: i64,
    /// Locked quote token balance in native units.
    pub quote_locked: i64,
    /// Solana slot when balance was last updated.
    pub slot: i64,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// New or updated balance for upsert.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertBalance {
    /// Market ID (foreign key).
    pub market_id: i32,
    /// Owner address.
    pub owner: String,
    /// Available base token balance in native units.
    pub base_free: i64,
    /// Locked base token balance in native units.
    pub base_locked: i64,
    /// Available quote token balance in native units.
    pub quote_free: i64,
    /// Locked quote token balance in native units.
    pub quote_locked: i64,
    /// Solana slot when balance was last updated.
    pub slot: i64,
}

/// Candle (OHLCV) record from the database.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Candle {
    /// Market ID (foreign key).
    pub market_id: i32,
    /// Candle start time.
    pub bucket: DateTime<Utc>,
    /// Opening price.
    pub open: i64,
    /// Highest price.
    pub high: i64,
    /// Lowest price.
    pub low: i64,
    /// Closing price.
    pub close: i64,
    /// Total volume in base lots.
    pub volume: i64,
    /// Total volume in quote (price * quantity).
    pub quote_volume: i64,
    /// Number of trades.
    pub trade_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_status_default() {
        assert_eq!(MarketStatus::default(), MarketStatus::Inactive);
    }

    #[test]
    fn test_order_status_default() {
        assert_eq!(OrderStatus::default(), OrderStatus::Open);
    }

    #[test]
    fn test_order_type_default() {
        assert_eq!(OrderType::default(), OrderType::Limit);
    }

    #[test]
    fn test_time_in_force_default() {
        assert_eq!(TimeInForce::default(), TimeInForce::GoodTillCancelled);
    }

    #[test]
    fn test_side_values() {
        assert_eq!(Side::Bid as i16, 0);
        assert_eq!(Side::Ask as i16, 1);
    }

    #[test]
    fn test_market_status_values() {
        assert_eq!(MarketStatus::Inactive as i16, 0);
        assert_eq!(MarketStatus::Active as i16, 1);
        assert_eq!(MarketStatus::Paused as i16, 2);
        assert_eq!(MarketStatus::Closed as i16, 3);
    }

    #[test]
    fn test_order_status_values() {
        assert_eq!(OrderStatus::Open as i16, 0);
        assert_eq!(OrderStatus::PartiallyFilled as i16, 1);
        assert_eq!(OrderStatus::Filled as i16, 2);
        assert_eq!(OrderStatus::Cancelled as i16, 3);
        assert_eq!(OrderStatus::Expired as i16, 4);
    }
}
