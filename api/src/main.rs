//! Matchbook API Server binary.
//!
//! Entry point for the REST API and WebSocket server.

use std::env;

use matchbook_api::{AppState, Server, ServerConfig};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,matchbook_api=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration from environment
    let host = env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = env::var("API_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("API_PORT must be a valid port number");

    let config = ServerConfig::new(host, port);
    let state = AppState::default();

    tracing::info!(
        "Starting Matchbook API server on {}:{}",
        config.host,
        config.port
    );

    let server = Server::new(config, state);
    server.run().await?;

    Ok(())
}
