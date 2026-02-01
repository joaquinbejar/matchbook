//! Instruction builders for Matchbook transactions.
//!
//! This module provides builders for constructing Solana instructions for all
//! Matchbook operations. Each builder handles account resolution, PDA derivation,
//! and instruction serialization.
//!
//! # Example
//!
//! ```rust,ignore
//! use matchbook_sdk::instructions::{PlaceOrderBuilder, pda};
//! use matchbook_sdk::types::{Side, Price, Quantity, OrderType};
//! use solana_sdk::pubkey::Pubkey;
//!
//! let program_id = Pubkey::new_unique();
//! let owner = Pubkey::new_unique();
//! let market = Pubkey::new_unique();
//!
//! let ix = PlaceOrderBuilder::new(program_id)
//!     .owner(owner)
//!     .market(market)
//!     .side(Side::Bid)
//!     .price(Price::new(1000))
//!     .quantity(Quantity::new(100))
//!     .order_type(OrderType::Limit)
//!     .build()
//!     .expect("should build instruction");
//! ```

pub mod cancel_all_orders;
pub mod cancel_order;
pub mod consume_events;
pub mod create_market;
pub mod create_open_orders;
pub mod deposit;
pub mod match_orders;
pub mod pda;
pub mod place_order;
pub mod withdraw;

pub use cancel_all_orders::CancelAllOrdersBuilder;
pub use cancel_order::CancelOrderBuilder;
pub use consume_events::ConsumeEventsBuilder;
pub use create_market::{CreateMarketBuilder, CreateMarketParams};
pub use create_open_orders::CreateOpenOrdersBuilder;
pub use deposit::{DepositBuilder, DepositParams};
pub use match_orders::MatchOrdersBuilder;
pub use pda::{
    derive_asks_address, derive_base_vault_address, derive_bids_address,
    derive_event_queue_address, derive_market_address, derive_open_orders_address,
    derive_quote_vault_address, MarketPdas,
};
pub use place_order::PlaceOrderBuilder;
pub use withdraw::{WithdrawBuilder, WithdrawParams};
