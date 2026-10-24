//! # MatchBook Rust SDK
//!
//! Client-side helpers for the MatchBook order book on Solana:
//!
//! - [`ix`]: every instruction with its full account list.
//! - [`MarketSnapshot`]: a decoded market account with the book, seats and
//!   an exact swap quote. Quotes run the same matching engine the program
//!   runs, so a quote is what the swap will execute against the same state.
//! - [`pda`]: the vault authority, vaults and associated token accounts.
//! - With the `jupiter` feature, [`jupiter::MatchBookAmm`] implements
//!   `jupiter_amm_interface::Amm` so aggregators can route through any
//!   MatchBook market.

pub mod ix;
pub mod pda;
mod snapshot;

#[cfg(feature = "jupiter")]
pub mod jupiter;

pub use matchbook_program::engine;
pub use matchbook_program::instruction::{
    CreateMarketArgs, DelegateArgs, OrderArgs, StopArgs, SwapArgs,
};
pub use snapshot::{MarketSnapshot, SnapshotError};

use solana_pubkey::Pubkey;

/// The MatchBook program id.
pub const PROGRAM_ID: Pubkey = Pubkey::new_from_array(matchbook_program::ID.to_bytes());

/// SPL Token program.
pub const TOKEN_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
/// SPL Token-2022 program.
pub const TOKEN_2022_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
/// Associated Token Account program.
pub const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
/// System program.
pub const SYSTEM_PROGRAM_ID: Pubkey = Pubkey::from_str_const("11111111111111111111111111111111");

/// Seat hint that makes the program look the seat up.
pub const NO_HINT: u32 = u32::MAX;
