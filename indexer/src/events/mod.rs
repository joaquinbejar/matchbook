//! Event processor module for Matchbook indexer.
//!
//! This module provides components for processing on-chain events
//! (fills and cancellations) and persisting them to the database.
//!
//! # Components
//!
//! - [`types`]: ProcessedFill, ProcessedOut, OrderUpdate types
//! - [`cursor`]: EventCursor for tracking processing progress
//! - [`processor`]: EventProcessor implementation
//! - [`metrics`]: Event processor metrics

pub mod cursor;
pub mod metrics;
pub mod processor;
pub mod types;

pub use cursor::EventCursor;
pub use metrics::EventMetrics;
pub use processor::EventProcessor;
pub use types::{OrderUpdate, OutReason, ProcessedFill, ProcessedOut, ProcessingResult};
