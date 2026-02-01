//! Book builder module for Matchbook indexer.
//!
//! This module provides components for maintaining in-memory order book state
//! and computing deltas for real-time updates.
//!
//! # Components
//!
//! - [`types`]: PriceLevel, BookChange, BookUpdate types
//! - [`orderbook`]: FullOrderBook implementation
//! - [`builder`]: BookBuilder for managing multiple markets
//! - [`metrics`]: Book metrics tracking

pub mod builder;
pub mod metrics;
pub mod orderbook;
pub mod types;

pub use builder::BookBuilder;
pub use metrics::BookMetrics;
pub use orderbook::FullOrderBook;
pub use types::{BookChange, BookUpdate, PriceLevel, Side};
