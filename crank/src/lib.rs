//! Matchbook Crank - Service for monitoring and executing order matching.
//!
//! This crate provides the crank service that monitors order books for crossing
//! orders and submits MatchOrders transactions. This is a permissionless service
//! that earns priority fees for executing matches.
//!
//! # Components
//!
//! - [`config`]: Crank configuration
//! - [`detector`]: Cross detection logic
//! - [`builder`]: Transaction building
//! - [`submitter`]: Transaction submission
//! - [`service`]: Main crank service
//! - [`metrics`]: Crank metrics

pub mod builder;
pub mod config;
pub mod detector;
pub mod metrics;
pub mod service;
pub mod submitter;

pub use builder::TransactionBuilder;
pub use config::CrankConfig;
pub use detector::{CrossDetector, CrossInfo};
pub use metrics::CrankMetrics;
pub use service::CrankService;
pub use submitter::{SubmitterConfig, TransactionSubmitter};
