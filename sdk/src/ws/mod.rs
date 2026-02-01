//! WebSocket client for real-time streaming.
//!
//! This module provides a WebSocket client for connecting to the Matchbook
//! streaming API for real-time order book updates, trades, and user order
//! notifications.
//!
//! # Example
//!
//! ```rust,ignore
//! use matchbook_sdk::ws::{MatchbookWsClient, WsConfig, Channel};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = MatchbookWsClient::with_url("wss://ws.matchbook.example/v1/stream")?;
//!
//!     // Connect to the server
//!     client.connect().await?;
//!
//!     // Subscribe to order book updates
//!     client.subscribe_book("ABC123...", Some(20)).await?;
//!
//!     // Process events
//!     loop {
//!         let event = client.next_event().await?;
//!         println!("Received: {:?}", event);
//!     }
//! }
//! ```

pub mod client;
pub mod config;
pub mod error;
pub mod messages;

pub use client::MatchbookWsClient;
pub use config::WsConfig;
pub use error::WsError;
pub use messages::{BookChange, Channel, ClientMessage, ServerMessage, Subscription};
