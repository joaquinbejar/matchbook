//! Market types for the Matchbook SDK.
//!
//! Provides market configuration and state types.

use std::fmt;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::primitives::{Price, Quantity};

/// Market configuration and state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Market {
    /// Market account address (base58 encoded).
    pub address: String,

    /// Base token mint address (base58 encoded).
    pub base_mint: String,

    /// Quote token mint address (base58 encoded).
    pub quote_mint: String,

    /// Bids order book address (base58 encoded).
    pub bids: String,

    /// Asks order book address (base58 encoded).
    pub asks: String,

    /// Event queue address (base58 encoded).
    pub event_queue: String,

    /// Base token vault address (base58 encoded).
    pub base_vault: String,

    /// Quote token vault address (base58 encoded).
    pub quote_vault: String,

    /// Minimum price increment in quote lots.
    pub tick_size: u64,

    /// Minimum quantity increment in base lots.
    pub lot_size: u64,

    /// Decimals for base token.
    pub base_decimals: u8,

    /// Decimals for quote token.
    pub quote_decimals: u8,

    /// Taker fee in basis points.
    pub taker_fee_bps: u16,

    /// Maker fee in basis points (can be negative for rebates).
    pub maker_fee_bps: i16,

    /// Base token symbol.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_symbol: Option<String>,

    /// Quote token symbol.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote_symbol: Option<String>,
}

impl Market {
    /// Returns the market name (e.g., "SOL/USDC").
    #[must_use]
    pub fn name(&self) -> String {
        match (&self.base_symbol, &self.quote_symbol) {
            (Some(base), Some(quote)) => format!("{}/{}", base, quote),
            _ => self.address.clone(),
        }
    }

    /// Converts a human-readable price to ticks.
    ///
    /// # Arguments
    ///
    /// * `price` - Price in quote currency per base unit
    #[must_use]
    pub fn price_to_ticks(&self, price: Decimal) -> Price {
        let tick_size_decimal = Decimal::from(self.tick_size)
            / Decimal::from(10u64.pow(u32::from(self.quote_decimals)));

        let ticks = price / tick_size_decimal;
        Price::new(ticks.to_string().parse().unwrap_or(0))
    }

    /// Converts ticks to a human-readable price.
    ///
    /// # Arguments
    ///
    /// * `ticks` - Price in tick units
    #[must_use]
    pub fn ticks_to_price(&self, ticks: Price) -> Decimal {
        let tick_size_decimal = Decimal::from(self.tick_size)
            / Decimal::from(10u64.pow(u32::from(self.quote_decimals)));

        Decimal::from(ticks.value()) * tick_size_decimal
    }

    /// Converts a human-readable quantity to lots.
    ///
    /// # Arguments
    ///
    /// * `quantity` - Quantity in base units
    #[must_use]
    pub fn quantity_to_lots(&self, quantity: Decimal) -> Quantity {
        let lot_size_decimal =
            Decimal::from(self.lot_size) / Decimal::from(10u64.pow(u32::from(self.base_decimals)));

        let lots = quantity / lot_size_decimal;
        Quantity::new(lots.to_string().parse().unwrap_or(0))
    }

    /// Converts lots to a human-readable quantity.
    ///
    /// # Arguments
    ///
    /// * `lots` - Quantity in lot units
    #[must_use]
    pub fn lots_to_quantity(&self, lots: Quantity) -> Decimal {
        let lot_size_decimal =
            Decimal::from(self.lot_size) / Decimal::from(10u64.pow(u32::from(self.base_decimals)));

        Decimal::from(lots.value()) * lot_size_decimal
    }

    /// Calculates the taker fee for a given notional value.
    ///
    /// # Arguments
    ///
    /// * `notional` - Notional value in quote currency
    #[must_use]
    pub fn calculate_taker_fee(&self, notional: Decimal) -> Decimal {
        notional * Decimal::from(self.taker_fee_bps) / Decimal::from(10_000)
    }

    /// Calculates the maker fee/rebate for a given notional value.
    ///
    /// # Arguments
    ///
    /// * `notional` - Notional value in quote currency
    ///
    /// Returns negative value for rebates.
    #[must_use]
    pub fn calculate_maker_fee(&self, notional: Decimal) -> Decimal {
        notional * Decimal::from(self.maker_fee_bps) / Decimal::from(10_000)
    }

    /// Returns the minimum order size in base units.
    #[must_use]
    pub fn min_order_size(&self) -> Decimal {
        Decimal::from(self.lot_size) / Decimal::from(10u64.pow(u32::from(self.base_decimals)))
    }

    /// Returns the minimum price increment in quote units.
    #[must_use]
    pub fn min_price_increment(&self) -> Decimal {
        Decimal::from(self.tick_size) / Decimal::from(10u64.pow(u32::from(self.quote_decimals)))
    }
}

impl fmt::Display for Market {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Market statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketStats {
    /// Market address (base58 encoded).
    pub market: String,

    /// 24-hour volume in quote currency.
    pub volume_24h: String,

    /// 24-hour price change percentage.
    pub price_change_24h: String,

    /// Current best bid price.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_bid: Option<String>,

    /// Current best ask price.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_ask: Option<String>,

    /// Last trade price.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_price: Option<String>,

    /// 24-hour high price.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high_24h: Option<String>,

    /// 24-hour low price.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low_24h: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_market() -> Market {
        Market {
            address: "market123".to_string(),
            base_mint: "base_mint".to_string(),
            quote_mint: "quote_mint".to_string(),
            bids: "bids".to_string(),
            asks: "asks".to_string(),
            event_queue: "event_queue".to_string(),
            base_vault: "base_vault".to_string(),
            quote_vault: "quote_vault".to_string(),
            tick_size: 100,
            lot_size: 1000,
            base_decimals: 9,
            quote_decimals: 6,
            taker_fee_bps: 10,
            maker_fee_bps: -5,
            base_symbol: Some("SOL".to_string()),
            quote_symbol: Some("USDC".to_string()),
        }
    }

    #[test]
    fn test_market_name() {
        let market = create_test_market();
        assert_eq!(market.name(), "SOL/USDC");
    }

    #[test]
    fn test_market_name_no_symbols() {
        let mut market = create_test_market();
        market.base_symbol = None;
        market.quote_symbol = None;
        assert_eq!(market.name(), "market123");
    }

    #[test]
    fn test_market_display() {
        let market = create_test_market();
        assert_eq!(market.to_string(), "SOL/USDC");
    }

    #[test]
    fn test_price_to_ticks() {
        let market = create_test_market();
        // tick_size = 100, quote_decimals = 6
        // tick_size_decimal = 100 / 1_000_000 = 0.0001
        // price = 100.0 -> ticks = 100.0 / 0.0001 = 1_000_000
        let price = Decimal::from(100);
        let ticks = market.price_to_ticks(price);
        assert_eq!(ticks.value(), 1_000_000);
    }

    #[test]
    fn test_ticks_to_price() {
        let market = create_test_market();
        let ticks = Price::new(1_000_000);
        let price = market.ticks_to_price(ticks);
        assert_eq!(price, Decimal::from(100));
    }

    #[test]
    fn test_quantity_to_lots() {
        let market = create_test_market();
        // lot_size = 1000, base_decimals = 9
        // lot_size_decimal = 1000 / 1_000_000_000 = 0.000001
        // quantity = 1.0 -> lots = 1.0 / 0.000001 = 1_000_000
        let quantity = Decimal::from(1);
        let lots = market.quantity_to_lots(quantity);
        assert_eq!(lots.value(), 1_000_000);
    }

    #[test]
    fn test_lots_to_quantity() {
        let market = create_test_market();
        let lots = Quantity::new(1_000_000);
        let quantity = market.lots_to_quantity(lots);
        assert_eq!(quantity, Decimal::from(1));
    }

    #[test]
    fn test_calculate_taker_fee() {
        let market = create_test_market();
        // taker_fee_bps = 10 (0.1%)
        let notional = Decimal::from(1000);
        let fee = market.calculate_taker_fee(notional);
        assert_eq!(fee, Decimal::from(1)); // 1000 * 10 / 10000 = 1
    }

    #[test]
    fn test_calculate_maker_fee() {
        let market = create_test_market();
        // maker_fee_bps = -5 (-0.05% rebate)
        let notional = Decimal::from(1000);
        let fee = market.calculate_maker_fee(notional);
        // 1000 * -5 / 10000 = -0.5
        assert!(fee < Decimal::ZERO);
    }

    #[test]
    fn test_min_order_size() {
        let market = create_test_market();
        let min_size = market.min_order_size();
        // lot_size = 1000, base_decimals = 9
        // 1000 / 1_000_000_000 = 0.000001
        assert_eq!(
            min_size,
            Decimal::from(1000) / Decimal::from(1_000_000_000u64)
        );
    }

    #[test]
    fn test_min_price_increment() {
        let market = create_test_market();
        let min_increment = market.min_price_increment();
        // tick_size = 100, quote_decimals = 6
        // 100 / 1_000_000 = 0.0001
        assert_eq!(
            min_increment,
            Decimal::from(100) / Decimal::from(1_000_000u64)
        );
    }

    #[test]
    fn test_market_serde() {
        let market = create_test_market();
        let json = serde_json::to_string(&market).expect("serialize");
        let parsed: Market = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(parsed.address, market.address);
        assert_eq!(parsed.tick_size, market.tick_size);
        assert_eq!(parsed.base_symbol, market.base_symbol);
    }
}
