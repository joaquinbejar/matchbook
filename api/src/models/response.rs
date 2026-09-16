//! Response models for the API.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Market information response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketResponse {
    /// Market address (base58).
    pub address: String,
    /// Base token mint (base58).
    pub base_mint: String,
    /// Quote token mint (base58).
    pub quote_mint: String,
    /// Base lot size.
    pub base_lot_size: u64,
    /// Quote lot size.
    pub quote_lot_size: u64,
    /// Tick size.
    pub tick_size: u64,
    /// Taker fee in basis points.
    pub taker_fee_bps: u16,
    /// Maker fee in basis points (negative = rebate).
    pub maker_fee_bps: i16,
}

/// List of markets response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketsResponse {
    /// List of markets.
    pub markets: Vec<MarketResponse>,
}

/// Price level in the order book.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevelResponse {
    /// Price.
    pub price: u64,
    /// Total quantity at this price.
    pub quantity: u64,
    /// Number of orders at this price.
    pub order_count: u32,
}

/// Order book snapshot response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookResponse {
    /// Market address.
    pub market: String,
    /// Bid levels (highest price first).
    pub bids: Vec<PriceLevelResponse>,
    /// Ask levels (lowest price first).
    pub asks: Vec<PriceLevelResponse>,
    /// Slot number.
    pub slot: u64,
    /// Sequence number.
    pub seq: u64,
}

/// Trade response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeResponse {
    /// Trade ID.
    pub id: String,
    /// Market address.
    pub market: String,
    /// Price.
    pub price: u64,
    /// Quantity.
    pub quantity: u64,
    /// Taker side ("buy" or "sell").
    pub taker_side: String,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
}

/// List of trades response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradesResponse {
    /// List of trades.
    pub trades: Vec<TradeResponse>,
    /// Next cursor for pagination.
    pub next_cursor: Option<String>,
}

/// OHLCV candle response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleResponse {
    /// Candle start time.
    pub time: DateTime<Utc>,
    /// Open price.
    pub open: u64,
    /// High price.
    pub high: u64,
    /// Low price.
    pub low: u64,
    /// Close price.
    pub close: u64,
    /// Volume.
    pub volume: u64,
}

/// List of candles response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandlesResponse {
    /// Market address.
    pub market: String,
    /// Interval.
    pub interval: String,
    /// List of candles.
    pub candles: Vec<CandleResponse>,
}

/// User order response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponse {
    /// Order ID.
    pub order_id: String,
    /// Market address.
    pub market: String,
    /// Side ("buy" or "sell").
    pub side: String,
    /// Price.
    pub price: u64,
    /// Original quantity.
    pub quantity: u64,
    /// Filled quantity.
    pub filled_quantity: u64,
    /// Client order ID.
    pub client_order_id: Option<u64>,
    /// Created timestamp.
    pub created_at: DateTime<Utc>,
}

/// List of orders response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrdersResponse {
    /// List of orders.
    pub orders: Vec<OrderResponse>,
}

/// User balance response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceResponse {
    /// Market address.
    pub market: String,
    /// Base token free balance.
    pub base_free: u64,
    /// Base token locked balance.
    pub base_locked: u64,
    /// Quote token free balance.
    pub quote_free: u64,
    /// Quote token locked balance.
    pub quote_locked: u64,
}

/// List of balances response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalancesResponse {
    /// Owner address.
    pub owner: String,
    /// List of balances.
    pub balances: Vec<BalanceResponse>,
}

/// Transaction response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResponse {
    /// Base64-encoded transaction.
    pub transaction: String,
    /// Message to sign.
    pub message: String,
}

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Service status.
    pub status: String,
    /// Version.
    pub version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_response() {
        let market = MarketResponse {
            address: "market123".to_string(),
            base_mint: "base456".to_string(),
            quote_mint: "quote789".to_string(),
            base_lot_size: 1000,
            quote_lot_size: 100,
            tick_size: 10,
            taker_fee_bps: 30,
            maker_fee_bps: -10,
        };

        assert_eq!(market.address, "market123");
        assert_eq!(market.taker_fee_bps, 30);
    }

    #[test]
    fn test_orderbook_response() {
        let book = OrderBookResponse {
            market: "market123".to_string(),
            bids: vec![PriceLevelResponse {
                price: 1000,
                quantity: 100,
                order_count: 2,
            }],
            asks: vec![],
            slot: 100,
            seq: 1,
        };

        assert_eq!(book.bids.len(), 1);
        assert_eq!(book.bids[0].price, 1000);
    }

    #[test]
    fn test_health_response() {
        let health = HealthResponse {
            status: "ok".to_string(),
            version: "0.2.0".to_string(),
        };

        assert_eq!(health.status, "ok");
    }
}
