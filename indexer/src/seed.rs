//! Seed data for Matchbook indexer testing.
//!
//! Provides functions to populate the database with test data.

use crate::db::models::{NewMarket, NewOrder, NewTrade, UpsertBalance};
use chrono::{Duration, Utc};
use rust_decimal::Decimal;

/// Creates a sample market for testing.
#[must_use]
pub fn sample_market() -> NewMarket {
    NewMarket {
        address: "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF".to_string(),
        base_mint: "So11111111111111111111111111111111111111112".to_string(),
        quote_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
        base_decimals: 9,
        quote_decimals: 6,
        base_lot_size: 1_000_000,
        quote_lot_size: 1_000,
        tick_size: 100,
        min_order_size: 1,
        taker_fee_bps: 30,
        maker_fee_bps: -10,
        status: 1, // Active
    }
}

/// Creates a second sample market for testing.
#[must_use]
pub fn sample_market_2() -> NewMarket {
    NewMarket {
        address: "8xLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGG".to_string(),
        base_mint: "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So".to_string(),
        quote_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
        base_decimals: 9,
        quote_decimals: 6,
        base_lot_size: 1_000_000,
        quote_lot_size: 1_000,
        tick_size: 100,
        min_order_size: 1,
        taker_fee_bps: 25,
        maker_fee_bps: -5,
        status: 1, // Active
    }
}

/// Creates sample trades for testing.
///
/// # Arguments
///
/// * `market_id` - The market ID to associate trades with
/// * `count` - Number of trades to generate
#[must_use]
pub fn sample_trades(market_id: i32, count: usize) -> Vec<NewTrade> {
    let now = Utc::now();
    let maker_address = "Maker111111111111111111111111111111111111111".to_string();
    let taker_address = "Taker111111111111111111111111111111111111111".to_string();

    (0..count)
        .map(|i| {
            let timestamp = now - Duration::seconds((count - i) as i64 * 60);
            let price = 10500 + (i as i64 % 100) - 50; // Price varies around 10500
            let quantity = 100 + (i as i64 % 50);

            NewTrade {
                market_id,
                maker_order_id: Decimal::from(1000 + i as i64),
                taker_order_id: Decimal::from(2000 + i as i64),
                maker_address: maker_address.clone(),
                taker_address: taker_address.clone(),
                price,
                quantity,
                taker_side: (i % 2) as i16, // Alternating buy/sell
                taker_fee: quantity * price / 10000 * 30, // 30 bps
                maker_rebate: quantity * price / 10000 * 10, // 10 bps rebate
                slot: 200_000_000 + i as i64,
                seq_num: i as i64,
                timestamp,
            }
        })
        .collect()
}

/// Creates sample orders for testing.
///
/// # Arguments
///
/// * `market_id` - The market ID to associate orders with
/// * `count` - Number of orders to generate
#[must_use]
pub fn sample_orders(market_id: i32, count: usize) -> Vec<NewOrder> {
    let now = Utc::now();
    let owner = "Owner111111111111111111111111111111111111111".to_string();

    (0..count)
        .map(|i| {
            let placed_at = now - Duration::seconds((count - i) as i64 * 120);
            let side = (i % 2) as i16;
            let price = if side == 0 {
                10400 + i as i64
            } else {
                10600 - i as i64
            };

            NewOrder {
                market_id,
                order_id: Decimal::from(3000 + i as i64),
                owner: owner.clone(),
                side,
                price,
                original_quantity: 100 + (i as i64 % 100),
                order_type: 0,    // Limit
                time_in_force: 0, // GTC
                client_order_id: Some(i as i64),
                slot: 200_000_000 + i as i64,
                placed_at,
            }
        })
        .collect()
}

/// Creates sample balances for testing.
///
/// # Arguments
///
/// * `market_id` - The market ID to associate balances with
/// * `owners` - List of owner addresses
#[must_use]
pub fn sample_balances(market_id: i32, owners: &[&str]) -> Vec<UpsertBalance> {
    owners
        .iter()
        .enumerate()
        .map(|(i, owner)| UpsertBalance {
            market_id,
            owner: (*owner).to_string(),
            base_free: 1_000_000_000 * (i as i64 + 1),
            base_locked: 100_000_000 * (i as i64 + 1),
            quote_free: 10_000_000_000 * (i as i64 + 1),
            quote_locked: 1_000_000_000 * (i as i64 + 1),
            slot: 200_000_000,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_market() {
        let market = sample_market();
        assert_eq!(market.base_decimals, 9);
        assert_eq!(market.quote_decimals, 6);
        assert_eq!(market.taker_fee_bps, 30);
        assert_eq!(market.maker_fee_bps, -10);
        assert_eq!(market.status, 1);
    }

    #[test]
    fn test_sample_market_2() {
        let market = sample_market_2();
        assert_ne!(market.address, sample_market().address);
        assert_eq!(market.status, 1);
    }

    #[test]
    fn test_sample_trades() {
        let trades = sample_trades(1, 10);
        assert_eq!(trades.len(), 10);

        for (i, trade) in trades.iter().enumerate() {
            assert_eq!(trade.market_id, 1);
            assert_eq!(trade.seq_num, i as i64);
        }
    }

    #[test]
    fn test_sample_orders() {
        let orders = sample_orders(1, 5);
        assert_eq!(orders.len(), 5);

        for order in &orders {
            assert_eq!(order.market_id, 1);
            assert!(order.client_order_id.is_some());
        }
    }

    #[test]
    fn test_sample_balances() {
        let owners = vec!["Owner1", "Owner2", "Owner3"];
        let balances = sample_balances(1, &owners);
        assert_eq!(balances.len(), 3);

        for (i, balance) in balances.iter().enumerate() {
            assert_eq!(balance.market_id, 1);
            assert_eq!(balance.owner, owners[i]);
        }
    }
}
