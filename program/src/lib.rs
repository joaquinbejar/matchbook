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
    CancelAllOrdersParams, CancelOrderParams, CreateMarketParams, CreateOpenOrdersParams,
    DepositParams, WithdrawParams, BASE_VAULT_SEED, EVENT_QUEUE_ACCOUNT_SIZE, MAX_FEE_BPS,
    MAX_MAKER_FEE_BPS, MAX_MAKER_REBATE_BPS, ORDERBOOK_ACCOUNT_SIZE, QUOTE_VAULT_SEED,
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

    /// Deposits tokens from user's wallet to market vaults.
    ///
    /// Transfers base and/or quote tokens from the user's token accounts
    /// to the market vaults, crediting the user's OpenOrders account.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The instruction context
    /// * `params` - Deposit amounts for base and quote tokens
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Both amounts are zero
    /// - Market is not active
    /// - Balance overflow would occur
    pub fn deposit(ctx: Context<Deposit>, params: DepositParams) -> Result<()> {
        instructions::deposit::handler(ctx, params)
    }

    /// Withdraws tokens from market vaults to user's wallet.
    ///
    /// Transfers base and/or quote tokens from the market vaults
    /// to the user's token accounts, debiting the user's OpenOrders account.
    /// Only the owner can withdraw (delegate cannot).
    ///
    /// # Arguments
    ///
    /// * `ctx` - The instruction context
    /// * `params` - Withdraw amounts for base and quote tokens
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Both amounts are zero
    /// - Insufficient free balance
    pub fn withdraw(ctx: Context<Withdraw>, params: WithdrawParams) -> Result<()> {
        instructions::withdraw::handler(ctx, params)
    }

    /// Cancels an order from the order book.
    ///
    /// Removes the order, releases locked funds back to free balance,
    /// and emits an Out event for off-chain tracking.
    /// Both owner and delegate can cancel orders.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The instruction context
    /// * `params` - Cancel parameters (order_id, side)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Market is closed
    /// - Authority is not authorized
    /// - Order not found
    pub fn cancel_order(ctx: Context<CancelOrder>, params: CancelOrderParams) -> Result<()> {
        instructions::cancel_order::handler(ctx, params)
    }

    /// Cancels all orders for a user up to a limit.
    ///
    /// Iterates through OpenOrders slots and cancels each active order
    /// matching the side filter. Useful for market makers and emergency exit.
    /// Both owner and delegate can cancel orders.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The instruction context
    /// * `params` - Cancel parameters (side filter, limit)
    ///
    /// # Returns
    ///
    /// The number of orders cancelled.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Market is closed
    /// - Authority is not authorized
    /// - Limit is zero
    pub fn cancel_all_orders(
        ctx: Context<CancelAllOrders>,
        params: CancelAllOrdersParams,
    ) -> Result<u8> {
        instructions::cancel_all_orders::handler(ctx, params)
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

/// Accounts for the Deposit instruction.
#[derive(Accounts)]
#[instruction(params: DepositParams)]
pub struct Deposit<'info> {
    /// Owner of the OpenOrders account (must sign).
    pub owner: Signer<'info>,

    /// Market to deposit into.
    #[account(
        constraint = market.is_active() @ MatchbookError::MarketNotActive
    )]
    pub market: Account<'info, Market>,

    /// User's OpenOrders account.
    #[account(
        mut,
        seeds = [OPEN_ORDERS_SEED, market.key().as_ref(), owner.key().as_ref()],
        bump = open_orders.bump,
        has_one = owner @ MatchbookError::Unauthorized
    )]
    pub open_orders: Account<'info, OpenOrders>,

    /// User's base token account.
    #[account(
        mut,
        constraint = user_base_account.mint == market.base_mint @ MatchbookError::InvalidAccountData
    )]
    pub user_base_account: Account<'info, TokenAccount>,

    /// User's quote token account.
    #[account(
        mut,
        constraint = user_quote_account.mint == market.quote_mint @ MatchbookError::InvalidAccountData
    )]
    pub user_quote_account: Account<'info, TokenAccount>,

    /// Market's base token vault.
    #[account(
        mut,
        address = market.base_vault @ MatchbookError::InvalidAccountData
    )]
    pub base_vault: Account<'info, TokenAccount>,

    /// Market's quote token vault.
    #[account(
        mut,
        address = market.quote_vault @ MatchbookError::InvalidAccountData
    )]
    pub quote_vault: Account<'info, TokenAccount>,

    /// SPL Token program.
    pub token_program: Program<'info, Token>,
}

/// Accounts for the Withdraw instruction.
///
/// Note: Withdrawal is allowed even if market is paused/closed.
/// Only the owner can withdraw (delegate cannot).
#[derive(Accounts)]
#[instruction(params: WithdrawParams)]
pub struct Withdraw<'info> {
    /// Owner of the OpenOrders account (must sign). Delegate cannot withdraw.
    pub owner: Signer<'info>,

    /// Market to withdraw from. Withdrawal allowed even if paused/closed.
    pub market: Account<'info, Market>,

    /// User's OpenOrders account.
    #[account(
        mut,
        seeds = [OPEN_ORDERS_SEED, market.key().as_ref(), owner.key().as_ref()],
        bump = open_orders.bump,
        has_one = owner @ MatchbookError::Unauthorized
    )]
    pub open_orders: Account<'info, OpenOrders>,

    /// User's base token account.
    #[account(
        mut,
        constraint = user_base_account.mint == market.base_mint @ MatchbookError::InvalidAccountData
    )]
    pub user_base_account: Account<'info, TokenAccount>,

    /// User's quote token account.
    #[account(
        mut,
        constraint = user_quote_account.mint == market.quote_mint @ MatchbookError::InvalidAccountData
    )]
    pub user_quote_account: Account<'info, TokenAccount>,

    /// Market's base token vault.
    #[account(
        mut,
        address = market.base_vault @ MatchbookError::InvalidAccountData
    )]
    pub base_vault: Account<'info, TokenAccount>,

    /// Market's quote token vault.
    #[account(
        mut,
        address = market.quote_vault @ MatchbookError::InvalidAccountData
    )]
    pub quote_vault: Account<'info, TokenAccount>,

    /// SPL Token program.
    pub token_program: Program<'info, Token>,
}

/// Accounts for the CancelOrder instruction.
///
/// Note: Cancellation is allowed even if market is paused (CancelOnly mode).
/// Cancellation is NOT allowed if market is closed.
#[derive(Accounts)]
#[instruction(params: CancelOrderParams)]
pub struct CancelOrder<'info> {
    /// Authority cancelling the order (owner or delegate).
    pub authority: Signer<'info>,

    /// Market the order is on.
    #[account(
        has_one = bids @ MatchbookError::InvalidAccountData,
        has_one = asks @ MatchbookError::InvalidAccountData,
        has_one = event_queue @ MatchbookError::InvalidAccountData
    )]
    pub market: Account<'info, Market>,

    /// User's OpenOrders account.
    #[account(
        mut,
        seeds = [OPEN_ORDERS_SEED, market.key().as_ref(), open_orders.owner.as_ref()],
        bump = open_orders.bump,
        constraint = open_orders.is_authorized(authority.key) @ MatchbookError::Unauthorized
    )]
    pub open_orders: Account<'info, OpenOrders>,

    /// Bids order book account.
    /// CHECK: Validated via has_one on market.
    #[account(mut)]
    pub bids: UncheckedAccount<'info>,

    /// Asks order book account.
    /// CHECK: Validated via has_one on market.
    #[account(mut)]
    pub asks: UncheckedAccount<'info>,

    /// Event queue account.
    /// CHECK: Validated via has_one on market.
    #[account(mut)]
    pub event_queue: UncheckedAccount<'info>,
}

/// Accounts for the CancelAllOrders instruction.
///
/// Note: Cancellation is allowed even if market is paused (CancelOnly mode).
/// Cancellation is NOT allowed if market is closed.
#[derive(Accounts)]
#[instruction(params: CancelAllOrdersParams)]
pub struct CancelAllOrders<'info> {
    /// Authority cancelling the orders (owner or delegate).
    pub authority: Signer<'info>,

    /// Market the orders are on.
    #[account(
        has_one = bids @ MatchbookError::InvalidAccountData,
        has_one = asks @ MatchbookError::InvalidAccountData,
        has_one = event_queue @ MatchbookError::InvalidAccountData
    )]
    pub market: Account<'info, Market>,

    /// User's OpenOrders account.
    #[account(
        mut,
        seeds = [OPEN_ORDERS_SEED, market.key().as_ref(), open_orders.owner.as_ref()],
        bump = open_orders.bump,
        constraint = open_orders.is_authorized(authority.key) @ MatchbookError::Unauthorized
    )]
    pub open_orders: Account<'info, OpenOrders>,

    /// Bids order book account.
    /// CHECK: Validated via has_one on market.
    #[account(mut)]
    pub bids: UncheckedAccount<'info>,

    /// Asks order book account.
    /// CHECK: Validated via has_one on market.
    #[account(mut)]
    pub asks: UncheckedAccount<'info>,

    /// Event queue account.
    /// CHECK: Validated via has_one on market.
    #[account(mut)]
    pub event_queue: UncheckedAccount<'info>,
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
