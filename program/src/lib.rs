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
