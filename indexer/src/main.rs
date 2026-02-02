//! Matchbook Indexer binary.
//!
//! Entry point for the Geyser indexer service.

use std::env;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,matchbook_indexer=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration from environment
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://matchbook:matchbook@localhost/matchbook".to_string());
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let geyser_endpoint =
        env::var("GEYSER_ENDPOINT").unwrap_or_else(|_| "http://localhost:10000".to_string());

    tracing::info!("Starting Matchbook Indexer");
    tracing::info!("Database URL: {}", database_url);
    tracing::info!("Redis URL: {}", redis_url);
    tracing::info!("Geyser endpoint: {}", geyser_endpoint);

    // TODO: Initialize and run the indexer service
    // For now, just keep the process running
    tracing::info!("Indexer service started (placeholder)");

    // Keep the process running
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down indexer");

    Ok(())
}
