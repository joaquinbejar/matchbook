//! Geyser listener module for Matchbook indexer.
//!
//! This module provides components for subscribing to Solana account updates
//! via the Geyser plugin interface.
//!
//! # Components
//!
//! - [`config`]: Configuration types for Geyser connection
//! - [`types`]: Account update and subscription filter types
//! - [`metrics`]: Metrics tracking for monitoring
//! - [`listener`]: The main `GeyserListener` implementation

pub mod config;
pub mod listener;
pub mod metrics;
pub mod types;

pub use config::GeyserConfig;
pub use listener::GeyserListener;
pub use metrics::GeyserMetrics;
pub use types::{AccountType, AccountUpdate, ConnectionState, SubscriptionFilter};
