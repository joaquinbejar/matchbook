//! Matchbook Crank Service binary.
//!
//! Entry point for the crank service that monitors and executes order matching.

use std::env;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,matchbook_crank=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration from environment
    let solana_rpc_url =
        env::var("SOLANA_RPC_URL").unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
    let min_profit: u64 = env::var("MIN_PROFIT_LAMPORTS")
        .unwrap_or_else(|_| "1000".to_string())
        .parse()
        .expect("MIN_PROFIT_LAMPORTS must be a valid number");
    let max_matches: u8 = env::var("MAX_MATCHES_PER_TX")
        .unwrap_or_else(|_| "8".to_string())
        .parse()
        .expect("MAX_MATCHES_PER_TX must be a valid number");

    tracing::info!("Starting Matchbook Crank Service");
    tracing::info!("Solana RPC URL: {}", solana_rpc_url);
    tracing::info!("Min profit: {} lamports", min_profit);
    tracing::info!("Max matches per tx: {}", max_matches);

    // TODO: Initialize and run the crank service
    // For now, just keep the process running
    tracing::info!("Crank service started (placeholder)");

    // Keep the process running
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down crank service");

    Ok(())
}
