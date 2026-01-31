//! Instructions for the Matchbook program.
//!
//! This module contains all instruction handlers and their associated
//! account structures.
//!
//! # Instructions
//!
//! - [`create_market`] - Creates a new trading market
//! - [`create_open_orders`] - Creates a user's trading account for a market
//! - [`deposit`] - Deposits tokens into a user's OpenOrders account
//! - [`withdraw`] - Withdraws tokens from a user's OpenOrders account
//! - [`cancel_order`] - Cancels an order from the order book
//! - [`cancel_all_orders`] - Cancels all orders for a user
//! - [`match_orders`] - Executes the matching algorithm (crank)
//! - [`consume_events`] - Processes events and settles funds

pub mod cancel_all_orders;
pub mod cancel_order;
pub mod consume_events;
pub mod create_market;
pub mod create_open_orders;
pub mod deposit;
pub mod match_orders;
pub mod withdraw;

pub use cancel_all_orders::CancelAllOrdersParams;
pub use cancel_order::CancelOrderParams;
pub use consume_events::ConsumeEventsParams;
pub use create_market::{
    CreateMarketParams, BASE_VAULT_SEED, EVENT_QUEUE_ACCOUNT_SIZE, MAX_FEE_BPS, MAX_MAKER_FEE_BPS,
    MAX_MAKER_REBATE_BPS, ORDERBOOK_ACCOUNT_SIZE, QUOTE_VAULT_SEED,
};
pub use create_open_orders::CreateOpenOrdersParams;
pub use deposit::DepositParams;
pub use match_orders::MatchOrdersParams;
pub use withdraw::WithdrawParams;
