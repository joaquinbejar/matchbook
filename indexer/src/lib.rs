//! Matchbook Indexer - Off-chain indexer and database layer for the Matchbook CLOB.
//!
//! This crate provides the database schema, models, Geyser listener, and utilities
//! for indexing and querying market data, trades, orders, and user balances.
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
//! # Geyser Listener
//!
//! The [`geyser`] module provides real-time subscription to Solana account updates
//! via the Geyser plugin interface:
//!
//! - [`GeyserConfig`]: Configuration for Geyser connection
//! - [`GeyserListener`]: Main listener that streams account updates
//! - [`GeyserMetrics`]: Metrics for monitoring connection health
//!
//! # Account Parser
//!
//! The [`parser`] module deserializes raw Solana account data into structured types:
//!
//! - [`AccountParser`]: Main parser for all account types
//! - [`ParsedAccount`]: Enum of parsed account variants
//! - [`ParserMetrics`]: Metrics for parse operations
//!
//! # Book Builder
//!
//! The [`book`] module maintains in-memory order book state:
//!
//! - [`BookBuilder`]: Manages order books for multiple markets
//! - [`FullOrderBook`]: Complete order book for a single market
//! - [`BookMetrics`]: Metrics for book operations
//!
//! # Usage
//!
//! ```rust,ignore
//! use matchbook_indexer::db::{Database, Market};
//! use matchbook_indexer::geyser::{GeyserConfig, GeyserListener};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Database connection
//!     let db = Database::connect("postgres://localhost/matchbook").await?;
//!     db.run_migrations().await?;
//!
//!     // Geyser listener
//!     let config = GeyserConfig::new(
//!         "http://localhost:10000",
//!         "MATCHBooK1111111111111111111111111111111111",
//!     );
//!     let (tx, rx) = tokio::sync::mpsc::channel(10000);
//!     let listener = GeyserListener::new(config, tx)?;
//!
//!     Ok(())
//! }
//! ```

pub mod book;
pub mod db;
pub mod geyser;
pub mod parser;
pub mod seed;

pub use book::{BookBuilder, BookMetrics, FullOrderBook};
pub use db::{Database, DatabaseError};
pub use geyser::{GeyserConfig, GeyserListener, GeyserMetrics};
pub use parser::{AccountParser, ParsedAccount, ParserMetrics};
