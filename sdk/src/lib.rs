//! Matchbook SDK - Rust client library for the Matchbook CLOB.
//!
//! This crate provides core types and utilities for interacting with the
//! Matchbook Central Limit Order Book on Solana.
//!
//! # Core Types
//!
//! - [`Price`], [`Quantity`] — Type-safe numeric wrappers
//! - [`Side`] — Order side (Bid/Ask)
//! - [`OrderType`] — Order type (Limit, PostOnly, IOC, FOK)
//! - [`TimeInForce`] — Time in force options
//! - [`OrderStatus`] — Order status
//!
//! # Entity Types
//!
//! - [`Market`] — Market configuration and state
//! - [`Order`] — Order details
//! - [`Trade`] — Executed trade
//! - [`BookLevel`] — Aggregated price level
//! - [`OrderBook`] — Full order book with bids/asks
//! - [`Balance`] — User balances
//!
//! # Example
//!
//! ```rust
//! use matchbook_sdk::{Price, Quantity, Side, OrderType};
//!
//! let price = Price::new(1000);
//! let quantity = Quantity::new(100);
//! let side = Side::Bid;
//! let order_type = OrderType::Limit;
//! ```

pub mod error;
pub mod types;

pub use error::SdkError;
pub use types::{
    Balance, BookLevel, Market, Order, OrderBook, OrderStatus, OrderType, Price, Quantity, Side,
    TimeInForce, Trade,
};
