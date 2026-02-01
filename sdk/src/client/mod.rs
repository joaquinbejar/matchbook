//! HTTP client for the Matchbook REST API.
//!
//! This module provides a type-safe HTTP client for interacting with the
//! Matchbook REST API.
//!
//! # Example
//!
//! ```rust,ignore
//! use matchbook_sdk::client::{MatchbookClient, ClientConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = MatchbookClient::with_base_url("https://api.matchbook.example/v1")?;
//!
//!     // Get all markets
//!     let markets = client.get_markets().await?;
//!     println!("Found {} markets", markets.len());
//!
//!     // Get order book for a market
//!     let orderbook = client.get_orderbook("ABC123...", Some(10)).await?;
//!     println!("Best bid: {:?}", orderbook.best_bid());
//!
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod error;
pub mod http;

pub use config::ClientConfig;
pub use error::ClientError;
pub use http::MatchbookClient;
