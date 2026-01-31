//! Matchbook: High-performance Central Limit Order Book on Solana.
//!
//! This program implements a non-custodial CLOB with the following features:
//! - Price-time priority order matching
//! - Permissionless crank for order execution
//! - Efficient B+ tree order book structure
//! - Event queue for asynchronous settlement
//!
//! # Architecture
//!
//! The program uses the following account structure:
//! - **Market**: Configuration and global state
//! - **OrderBookSide**: Bids and Asks stored as B+ trees
//! - **EventQueue**: Ring buffer for fill and cancel events
//! - **OpenOrders**: Per-user account for orders and balances
//!
//! See `.internalDoc/03-onchain-design.md` for detailed architecture.

use anchor_lang::prelude::*;

pub mod error;
pub mod state;

pub use error::{ErrorCategory, MatchbookError};

pub use state::{
    critbit, get_bit, AnyNode, Event, EventQueueHeader, FillEvent, FreeNode, InnerNode, LeafNode,
    Market, MarketStatus, NodeTag, OpenOrders, OrderBookSideHeader, OrderId, OrderSlot, OutEvent,
    OutReason, Side, TimeInForce, ASKS_SEED, BIDS_SEED, EMPTY_ORDER_ID, EVENT_QUEUE_HEADER_SIZE,
    EVENT_QUEUE_SEED, MARKET_SEED, MAX_ORDERS, NODE_SIZE, OPEN_ORDERS_SEED, ORDERBOOK_HEADER_SIZE,
    SENTINEL,
};

declare_id!("MATCHBooK1111111111111111111111111111111111");

/// Matchbook program module.
#[program]
pub mod matchbook {
    use super::*;

    /// Placeholder instruction for initial setup.
    ///
    /// This will be replaced with actual instructions in subsequent issues.
    pub fn initialize(_ctx: Context<Initialize>) -> Result<()> {
        msg!("Matchbook program initialized");
        Ok(())
    }
}

/// Accounts for the initialize instruction.
#[derive(Accounts)]
pub struct Initialize<'info> {
    /// The signer account (placeholder).
    #[account(mut)]
    pub signer: Signer<'info>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_id() {
        // Verify the program ID is set
        assert_ne!(ID, Pubkey::default());
    }
}
