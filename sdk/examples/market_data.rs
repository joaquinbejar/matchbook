//! Market Data Example
//!
//! This example demonstrates how to fetch and display market information,
//! order book data, and recent trades using the Matchbook SDK.
//!
//! # Environment Variables
//!
//! - `MATCHBOOK_API_URL` - REST API base URL (default: https://api.matchbook.example/v1)
//! - `MATCHBOOK_MARKET` - Market address to query
//!
//! # Usage
//!
//! ```bash
//! MATCHBOOK_MARKET=ABC123... cargo run --example market_data
//! ```

use matchbook_sdk::client::{ClientConfig, MatchbookClient};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment variables
    let api_url = env::var("MATCHBOOK_API_URL")
        .unwrap_or_else(|_| "https://api.matchbook.example/v1".to_string());
    let market_address = env::var("MATCHBOOK_MARKET")
        .expect("MATCHBOOK_MARKET environment variable is required");

    println!("=== Matchbook Market Data Example ===\n");
    println!("API URL: {api_url}");
    println!("Market: {market_address}\n");

    // Create the HTTP client
    let config = ClientConfig::new(&api_url);
    let client = MatchbookClient::new(config)?;

    // Step 1: Fetch all available markets
    println!("--- Fetching All Markets ---");
    match client.get_markets().await {
        Ok(markets) => {
            println!("Found {} markets:", markets.len());
            for market in markets.iter().take(5) {
                println!(
                    "  - {} ({}/{})",
                    market.address,
                    market.base_symbol.as_deref().unwrap_or("???"),
                    market.quote_symbol.as_deref().unwrap_or("???")
                );
            }
            if markets.len() > 5 {
                println!("  ... and {} more", markets.len() - 5);
            }
        }
        Err(e) => {
            println!("Error fetching markets: {e}");
        }
    }
    println!();

    // Step 2: Fetch specific market details
    println!("--- Fetching Market Details ---");
    match client.get_market(&market_address).await {
        Ok(market) => {
            println!("Market: {}", market.address);
            println!(
                "  Pair: {}/{}",
                market.base_symbol.as_deref().unwrap_or("???"),
                market.quote_symbol.as_deref().unwrap_or("???")
            );
            println!("  Base Mint: {}", market.base_mint);
            println!("  Quote Mint: {}", market.quote_mint);
            println!("  Tick Size: {}", market.tick_size);
            println!("  Lot Size: {}", market.lot_size);
            println!("  Maker Fee: {}%", market.maker_fee_bps as f64 / 100.0);
            println!("  Taker Fee: {}%", market.taker_fee_bps as f64 / 100.0);
        }
        Err(e) => {
            println!("Error fetching market: {e}");
        }
    }
    println!();

    // Step 3: Fetch order book
    println!("--- Fetching Order Book (depth=10) ---");
    match client.get_orderbook(&market_address, Some(10)).await {
        Ok(book) => {
            println!("Order Book for {}", book.market);

            println!("\n  ASKS (selling):");
            for (i, level) in book.asks.iter().take(5).enumerate() {
                println!(
                    "    {}: {} @ {} ({} orders)",
                    i + 1,
                    level.quantity,
                    level.price,
                    level.order_count
                );
            }

            println!("\n  BIDS (buying):");
            for (i, level) in book.bids.iter().take(5).enumerate() {
                println!(
                    "    {}: {} @ {} ({} orders)",
                    i + 1,
                    level.quantity,
                    level.price,
                    level.order_count
                );
            }

            // Calculate spread if we have both sides
            if let (Some(best_bid), Some(best_ask)) = (book.bids.first(), book.asks.first()) {
                let bid_price: f64 = best_bid.price.value() as f64;
                let ask_price: f64 = best_ask.price.value() as f64;
                let spread = ask_price - bid_price;
                let spread_pct = if bid_price > 0.0 {
                    (spread / bid_price) * 100.0
                } else {
                    0.0
                };
                println!("\n  Spread: {spread:.4} ({spread_pct:.4}%)");
            }
        }
        Err(e) => {
            println!("Error fetching order book: {e}");
        }
    }
    println!();

    // Step 4: Fetch recent trades
    println!("--- Fetching Recent Trades (limit=10) ---");
    match client.get_trades(&market_address, Some(10), None).await {
        Ok(trades) => {
            println!("Recent trades:");
            for trade in trades.iter().take(10) {
                println!(
                    "  {} {} @ {} ({})",
                    trade.taker_side, trade.quantity, trade.price, trade.timestamp
                );
            }
        }
        Err(e) => {
            println!("Error fetching trades: {e}");
        }
    }

    println!("\n=== Example Complete ===");
    Ok(())
}
