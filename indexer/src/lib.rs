//! Matchbook Indexer - Off-chain database layer for the Matchbook CLOB.
//!
//! This crate provides the database schema, models, and utilities for storing
//! and querying market data, trades, orders, and user balances.
//!
//! # Architecture
//!
//! The indexer uses PostgreSQL with TimescaleDB extension for efficient
//! time-series data storage and querying:
//!
//! - **markets**: Market metadata (addresses, lot sizes, fees)
//! - **trades**: Executed trades (TimescaleDB hypertable)
//! - **orders**: Order history
//! - **balances**: User balance snapshots
//! - **candles**: OHLCV aggregates (TimescaleDB continuous aggregates)
//!
//! # Usage
//!
//! ```rust,ignore
//! use matchbook_indexer::db::{Database, Market};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let db = Database::connect("postgres://localhost/matchbook").await?;
//!     db.run_migrations().await?;
//!
//!     let markets = db.list_markets().await?;
//!     Ok(())
//! }
//! ```

pub mod db;
pub mod seed;

pub use db::{Database, DatabaseError};
