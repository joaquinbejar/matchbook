//! WebSocket module for real-time updates.
//!
//! Provides WebSocket server functionality for streaming order book updates,
//! trades, and user-specific order updates to connected clients.
//!
//! # Channels
//!
//! - `book:{market}` — Order book deltas
//! - `trades:{market}` — Real-time trades
//! - `orders:{owner}` — User's order updates (requires auth)
//!
//! # Message Types
//!
//! - `subscribe` / `unsubscribe` — Channel subscription requests
//! - `snapshot` — Initial state on subscription
//! - `update` — Incremental updates
//! - `error` — Error responses

pub mod channels;
pub mod connection;
pub mod handler;
pub mod messages;
pub mod metrics;

pub use channels::{Channel, ChannelManager};
pub use connection::Connection;
pub use handler::{ws_handler, WsState};
pub use messages::{ClientMessage, ServerMessage};
pub use metrics::WsMetrics;
