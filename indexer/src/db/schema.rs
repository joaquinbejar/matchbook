//! Database schema documentation for Matchbook indexer.
//!
//! This module provides documentation for the database schema design decisions
//! and relationships between tables.
//!
//! # Schema Overview
//!
//! ```text
//! ┌─────────────┐
//! │   markets   │
//! │─────────────│
//! │ id (PK)     │◄──────────────────────────────────────┐
//! │ address     │                                       │
//! │ base_mint   │                                       │
//! │ quote_mint  │                                       │
//! │ ...         │                                       │
//! └─────────────┘                                       │
//!       │                                               │
//!       │ 1:N                                           │
//!       ▼                                               │
//! ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
//! │   trades    │     │   orders    │     │  balances   │
//! │─────────────│     │─────────────│     │─────────────│
//! │ id          │     │ id (PK)     │     │ id (PK)     │
//! │ market_id   │────▶│ market_id   │────▶│ market_id   │
//! │ timestamp   │     │ order_id    │     │ owner       │
//! │ price       │     │ owner       │     │ base_free   │
//! │ quantity    │     │ price       │     │ quote_free  │
//! │ ...         │     │ ...         │     │ ...         │
//! └─────────────┘     └─────────────┘     └─────────────┘
//!       │
//!       │ Aggregated by TimescaleDB
//!       ▼
//! ┌─────────────┐
//! │   candles   │
//! │─────────────│
//! │ market_id   │
//! │ bucket      │
//! │ open/high   │
//! │ low/close   │
//! │ volume      │
//! └─────────────┘
//! ```
//!
//! # Design Decisions
//!
//! ## Data Types
//!
//! - **Addresses**: `VARCHAR(44)` for Solana base58 pubkeys (max 44 chars)
//! - **Order IDs**: `NUMERIC(39)` for u128 values (39 decimal digits max)
//! - **Prices/Quantities**: `BIGINT` for u64 values in native units
//! - **Enums**: `SMALLINT` for compact storage of enum values
//! - **Timestamps**: `TIMESTAMPTZ` for timezone-aware timestamps
//!
//! ## TimescaleDB Features
//!
//! ### Hypertables
//!
//! The `trades` table is converted to a TimescaleDB hypertable for:
//! - Automatic time-based partitioning (1-day chunks)
//! - Efficient time-range queries
//! - Automatic compression of old data
//! - Built-in retention policies
//!
//! ### Continuous Aggregates
//!
//! Candle data is computed using continuous aggregates:
//! - Automatic background refresh
//! - Incremental computation (only new data)
//! - Multiple intervals: 1m, 5m, 15m, 1h, 4h, 1d
//!
//! ## Indexing Strategy
//!
//! ### Primary Query Patterns
//!
//! 1. **Market lookups by address**: `idx_markets_address`
//! 2. **Trades by market and time**: `idx_trades_market_time`
//! 3. **User trade history**: `idx_trades_maker`, `idx_trades_taker`
//! 4. **User order history**: `idx_orders_owner`
//! 5. **User balances**: `idx_balances_owner`
//!
//! ### Composite Keys
//!
//! - `trades`: `(market_id, timestamp, id)` for hypertable partitioning
//! - `balances`: `UNIQUE(market_id, owner)` for upsert operations
//!
//! ## Retention Policies
//!
//! - **trades**: 1 year retention (configurable)
//! - **candles**: No automatic retention (aggregates are small)
//!
//! # Migration Order
//!
//! Migrations must be run in order due to foreign key dependencies:
//!
//! 1. `create_markets` - Base table, no dependencies
//! 2. `create_trades` - Depends on markets
//! 3. `create_orders` - Depends on markets
//! 4. `create_balances` - Depends on markets
//! 5. `create_candles` - Depends on trades (continuous aggregate)
//! 6. `create_retention_policies` - Depends on trades hypertable

/// Candle interval enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandleInterval {
    /// 1-minute candles.
    OneMinute,
    /// 5-minute candles.
    FiveMinutes,
    /// 15-minute candles.
    FifteenMinutes,
    /// 1-hour candles.
    OneHour,
    /// 4-hour candles.
    FourHours,
    /// 1-day candles.
    OneDay,
}

impl CandleInterval {
    /// Returns the view name for this interval.
    #[must_use]
    pub const fn view_name(&self) -> &'static str {
        match self {
            Self::OneMinute => "candles_1m",
            Self::FiveMinutes => "candles_5m",
            Self::FifteenMinutes => "candles_15m",
            Self::OneHour => "candles_1h",
            Self::FourHours => "candles_4h",
            Self::OneDay => "candles_1d",
        }
    }

    /// Returns the interval duration in seconds.
    #[must_use]
    pub const fn seconds(&self) -> u64 {
        match self {
            Self::OneMinute => 60,
            Self::FiveMinutes => 300,
            Self::FifteenMinutes => 900,
            Self::OneHour => 3600,
            Self::FourHours => 14400,
            Self::OneDay => 86400,
        }
    }

    /// Parses an interval from a string.
    ///
    /// Accepts: "1m", "5m", "15m", "1h", "4h", "1d"
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "1m" => Some(Self::OneMinute),
            "5m" => Some(Self::FiveMinutes),
            "15m" => Some(Self::FifteenMinutes),
            "1h" => Some(Self::OneHour),
            "4h" => Some(Self::FourHours),
            "1d" => Some(Self::OneDay),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candle_interval_view_names() {
        assert_eq!(CandleInterval::OneMinute.view_name(), "candles_1m");
        assert_eq!(CandleInterval::FiveMinutes.view_name(), "candles_5m");
        assert_eq!(CandleInterval::FifteenMinutes.view_name(), "candles_15m");
        assert_eq!(CandleInterval::OneHour.view_name(), "candles_1h");
        assert_eq!(CandleInterval::FourHours.view_name(), "candles_4h");
        assert_eq!(CandleInterval::OneDay.view_name(), "candles_1d");
    }

    #[test]
    fn test_candle_interval_seconds() {
        assert_eq!(CandleInterval::OneMinute.seconds(), 60);
        assert_eq!(CandleInterval::FiveMinutes.seconds(), 300);
        assert_eq!(CandleInterval::FifteenMinutes.seconds(), 900);
        assert_eq!(CandleInterval::OneHour.seconds(), 3600);
        assert_eq!(CandleInterval::FourHours.seconds(), 14400);
        assert_eq!(CandleInterval::OneDay.seconds(), 86400);
    }

    #[test]
    fn test_candle_interval_parse() {
        assert_eq!(CandleInterval::parse("1m"), Some(CandleInterval::OneMinute));
        assert_eq!(
            CandleInterval::parse("5m"),
            Some(CandleInterval::FiveMinutes)
        );
        assert_eq!(
            CandleInterval::parse("15m"),
            Some(CandleInterval::FifteenMinutes)
        );
        assert_eq!(CandleInterval::parse("1h"), Some(CandleInterval::OneHour));
        assert_eq!(CandleInterval::parse("4h"), Some(CandleInterval::FourHours));
        assert_eq!(CandleInterval::parse("1d"), Some(CandleInterval::OneDay));
        assert_eq!(CandleInterval::parse("invalid"), None);
    }
}
