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
use anchor_spl::token::{Mint, Token, TokenAccount};

pub mod error;
pub mod instructions;
pub mod state;

pub use error::{ErrorCategory, MatchbookError};
pub use instructions::{
    CreateMarketParams, CreateOpenOrdersParams, BASE_VAULT_SEED, EVENT_QUEUE_ACCOUNT_SIZE,
    MAX_FEE_BPS, MAX_MAKER_FEE_BPS, MAX_MAKER_REBATE_BPS, ORDERBOOK_ACCOUNT_SIZE, QUOTE_VAULT_SEED,
};

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

    /// Creates a new trading market with all associated accounts.
    ///
    /// This instruction initializes:
    /// - Market account (PDA)
    /// - Bids order book (PDA)
    /// - Asks order book (PDA)
    /// - Event queue (PDA)
    /// - Base token vault (PDA token account)
    /// - Quote token vault (PDA token account)
    ///
    /// # Arguments
    ///
    /// * `ctx` - The instruction context
    /// * `params` - Market configuration parameters
    ///
    /// # Errors
    ///
    /// Returns an error if parameters are invalid or mints are the same.
    pub fn create_market(ctx: Context<CreateMarket>, params: CreateMarketParams) -> Result<()> {
        instructions::create_market::handler(ctx, params)
    }

    /// Creates an OpenOrders account for a user in a market.
    ///
    /// Users must create this account before they can deposit funds or place orders.
    /// Each user needs one OpenOrders account per market they trade on.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The instruction context
    /// * `params` - Optional delegate configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the market is not active.
    pub fn create_open_orders(
        ctx: Context<CreateOpenOrders>,
        params: CreateOpenOrdersParams,
    ) -> Result<()> {
        instructions::create_open_orders::handler(ctx, params)
    }
}

/// Accounts for the CreateMarket instruction.
#[derive(Accounts)]
#[instruction(params: CreateMarketParams)]
pub struct CreateMarket<'info> {
    /// Payer for account creation rent.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Market authority that can modify market parameters.
    pub authority: Signer<'info>,

    /// Base token mint.
    pub base_mint: Account<'info, Mint>,

    /// Quote token mint.
    pub quote_mint: Account<'info, Mint>,

    /// Market account (PDA).
    #[account(
        init,
        payer = payer,
        space = 8 + Market::INIT_SPACE,
        seeds = [MARKET_SEED, base_mint.key().as_ref(), quote_mint.key().as_ref()],
        bump
    )]
    pub market: Account<'info, Market>,

    /// Bids order book account (PDA).
    /// CHECK: Initialized manually as zero_copy account.
    #[account(
        init,
        payer = payer,
        space = 8 + ORDERBOOK_ACCOUNT_SIZE,
        seeds = [BIDS_SEED, market.key().as_ref()],
        bump
    )]
    pub bids: UncheckedAccount<'info>,

    /// Asks order book account (PDA).
    /// CHECK: Initialized manually as zero_copy account.
    #[account(
        init,
        payer = payer,
        space = 8 + ORDERBOOK_ACCOUNT_SIZE,
        seeds = [ASKS_SEED, market.key().as_ref()],
        bump
    )]
    pub asks: UncheckedAccount<'info>,

    /// Event queue account (PDA).
    /// CHECK: Initialized manually as zero_copy account.
    #[account(
        init,
        payer = payer,
        space = 8 + EVENT_QUEUE_ACCOUNT_SIZE,
        seeds = [EVENT_QUEUE_SEED, market.key().as_ref()],
        bump
    )]
    pub event_queue: UncheckedAccount<'info>,

    /// Base token vault (PDA token account).
    #[account(
        init,
        payer = payer,
        token::mint = base_mint,
        token::authority = market,
        seeds = [BASE_VAULT_SEED, market.key().as_ref()],
        bump
    )]
    pub base_vault: Account<'info, TokenAccount>,

    /// Quote token vault (PDA token account).
    #[account(
        init,
        payer = payer,
        token::mint = quote_mint,
        token::authority = market,
        seeds = [QUOTE_VAULT_SEED, market.key().as_ref()],
        bump
    )]
    pub quote_vault: Account<'info, TokenAccount>,

    /// Fee recipient account for collected fees.
    /// CHECK: Can be any account, validated by authority.
    pub fee_recipient: UncheckedAccount<'info>,

    /// System program for account creation.
    pub system_program: Program<'info, System>,

    /// Token program for vault creation.
    pub token_program: Program<'info, Token>,
}

/// Accounts for the CreateOpenOrders instruction.
#[derive(Accounts)]
#[instruction(params: CreateOpenOrdersParams)]
pub struct CreateOpenOrders<'info> {
    /// Payer for account creation rent.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Owner of the OpenOrders account.
    pub owner: Signer<'info>,

    /// Market the OpenOrders account is for.
    #[account(
        constraint = market.is_active() @ MatchbookError::MarketNotActive
    )]
    pub market: Account<'info, Market>,

    /// OpenOrders account (PDA).
    #[account(
        init,
        payer = payer,
        space = 8 + OpenOrders::INIT_SPACE,
        seeds = [OPEN_ORDERS_SEED, market.key().as_ref(), owner.key().as_ref()],
        bump
    )]
    pub open_orders: Account<'info, OpenOrders>,

    /// System program for account creation.
    pub system_program: Program<'info, System>,
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
