//! User Data Example
//!
//! This example demonstrates how to fetch user-specific data including
//! orders, trades, and balances using the Matchbook SDK.
//!
//! # Environment Variables
//!
//! - `MATCHBOOK_API_URL` - REST API base URL (default: https://api.matchbook.example/v1)
//! - `MATCHBOOK_OWNER` - Wallet address to query
//! - `MATCHBOOK_MARKET` - Optional market address to filter by
//!
//! # Usage
//!
//! ```bash
//! MATCHBOOK_OWNER=9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM cargo run --example user_data
//! ```

use matchbook_sdk::client::{ClientConfig, MatchbookClient};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment variables
    let api_url = env::var("MATCHBOOK_API_URL")
        .unwrap_or_else(|_| "https://api.matchbook.example/v1".to_string());
    let owner_address =
        env::var("MATCHBOOK_OWNER").expect("MATCHBOOK_OWNER environment variable is required");
    let market_filter = env::var("MATCHBOOK_MARKET").ok();

    println!("=== Matchbook User Data Example ===\n");
    println!("API URL: {api_url}");
    println!("Owner: {owner_address}");
    if let Some(ref market) = market_filter {
        println!("Market filter: {market}");
    }
    println!();

    // Create the HTTP client
    let config = ClientConfig::new(&api_url);
    let client = MatchbookClient::new(config)?;

    // Step 1: Fetch user's open orders
    println!("--- Fetching Open Orders ---");
    match client
        .get_orders(&owner_address, market_filter.as_deref())
        .await
    {
        Ok(orders) => {
            println!("Found {} open orders:", orders.len());
            for order in orders.iter().take(10) {
                println!(
                    "  {} {} {} @ {} (status: {:?})",
                    order.order_id, order.side, order.remaining_quantity, order.price, order.status
                );
            }
            if orders.len() > 10 {
                println!("  ... and {} more", orders.len() - 10);
            }
        }
        Err(e) => {
            println!("Error fetching orders: {e}");
        }
    }
    println!();

    // Step 2: Fetch user's trade history
    println!("--- Fetching Trade History ---");
    match client
        .get_user_trades(&owner_address, market_filter.as_deref())
        .await
    {
        Ok(trades) => {
            println!("Found {} trades:", trades.len());
            for trade in trades.iter().take(10) {
                println!(
                    "  {} {} @ {} ({})",
                    trade.taker_side, trade.quantity, trade.price, trade.timestamp
                );
            }
            if trades.len() > 10 {
                println!("  ... and {} more", trades.len() - 10);
            }
        }
        Err(e) => {
            println!("Error fetching trades: {e}");
        }
    }
    println!();

    // Step 3: Fetch user's balances
    println!("--- Fetching Balances ---");
    match client.get_balances(&owner_address).await {
        Ok(balances) => {
            println!("Found {} balance entries:", balances.len());
            for balance in balances.iter() {
                println!("  Market: {}", balance.market);
                println!("    Base: {} available, {} locked", balance.base_free, balance.base_locked);
                println!(
                    "    Quote: {} available, {} locked",
                    balance.quote_free, balance.quote_locked
                );
            }
        }
        Err(e) => {
            println!("Error fetching balances: {e}");
        }
    }

    println!("\n=== Example Complete ===");
    Ok(())
}
