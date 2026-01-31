//! Instructions for the Matchbook program.
//!
//! This module contains all instruction handlers and their associated
//! account structures.
//!
//! # Instructions
//!
//! - [`create_market`] - Creates a new trading market

pub mod create_market;

pub use create_market::{
    CreateMarketParams, BASE_VAULT_SEED, EVENT_QUEUE_ACCOUNT_SIZE, MAX_FEE_BPS, MAX_MAKER_FEE_BPS,
    MAX_MAKER_REBATE_BPS, ORDERBOOK_ACCOUNT_SIZE, QUOTE_VAULT_SEED,
};
