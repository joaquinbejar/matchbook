//! # Matchbook program
//!
//! A crankless central limit order book for Solana built on
//! [`matchbook_engine`]. Takers are matched and makers are settled inside the
//! taker's own instruction: there is no crank and no event queue.
//!
//! Each market is one program-owned account (header plus a node pool) that
//! grows as it is used, so creating a market costs only the rent of its
//! header. Vaults are associated token accounts of a per-market PDA, so both
//! SPL Token and Token-2022 mints are supported.
#![no_std]

#[cfg(feature = "client")]
extern crate std;

pub mod error;
pub mod events;
pub mod instruction;
pub mod processor;
mod utils;

pub use matchbook_engine as engine;

pinocchio::address::declare_id!("MbKAoWL85AKTWWBvf5FbLJZFKQuBcH1DaWQdfnkJxTw");

/// Seed of the per-market PDA that owns both vaults.
pub const VAULT_AUTHORITY_SEED: &[u8] = b"vault_authority";

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint {
    use pinocchio::{no_allocator, nostd_panic_handler, program_entrypoint};

    program_entrypoint!(crate::processor::process_instruction);
    no_allocator!();
    nostd_panic_handler!();
}
