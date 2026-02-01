//! Core types for the Matchbook SDK.
//!
//! This module provides all the core types needed for interacting with
//! the Matchbook CLOB.

pub mod balance;
pub mod book;
pub mod market;
pub mod order;
pub mod primitives;
pub mod trade;

pub use balance::Balance;
pub use book::{BookLevel, OrderBook};
pub use market::Market;
pub use order::{Order, OrderStatus, OrderType, PlaceOrderParams, SelfTradeBehavior, TimeInForce};
pub use primitives::{Price, Quantity, Side};
pub use trade::Trade;
