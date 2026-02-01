//! Matchbook API - REST API server for the Matchbook CLOB.
//!
//! This crate provides REST endpoints for market data, order book snapshots,
//! trade history, and transaction building.
//!
//! # Endpoints
//!
//! ## Market Data
//!
//! - `GET /v1/markets` — List all markets
//! - `GET /v1/markets/{market}` — Market details
//! - `GET /v1/markets/{market}/orderbook` — Order book snapshot
//! - `GET /v1/markets/{market}/trades` — Recent trades
//! - `GET /v1/markets/{market}/candles` — OHLCV data
//!
//! ## User Accounts
//!
//! - `GET /v1/accounts/{owner}/orders` — User's open orders
//! - `GET /v1/accounts/{owner}/trades` — User's trade history
//! - `GET /v1/accounts/{owner}/balances` — User's balances
//!
//! ## Transaction Building
//!
//! - `POST /v1/tx/place-order` — Build PlaceOrder transaction
//! - `POST /v1/tx/cancel-order` — Build CancelOrder transaction
//! - `POST /v1/tx/deposit` — Build Deposit transaction
//! - `POST /v1/tx/withdraw` — Build Withdraw transaction
//!
//! ## WebSocket
//!
//! - `GET /v1/ws` — WebSocket endpoint for real-time updates
//!   - `book:{market}` — Order book deltas
//!   - `trades:{market}` — Real-time trades
//!   - `orders:{owner}` — User's order updates (requires auth)

pub mod error;
pub mod handlers;
pub mod models;
pub mod routes;
pub mod server;
pub mod state;
pub mod ws;

pub use error::ApiError;
pub use server::Server;
pub use state::AppState;
pub use ws::{Channel, ChannelManager, WsMetrics, WsState};
