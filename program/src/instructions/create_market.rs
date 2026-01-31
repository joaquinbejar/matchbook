//! CreateMarket instruction for initializing a new trading market.
//!
//! This instruction creates a new market with all associated accounts:
//! - Market account (PDA)
//! - Bids order book (PDA)
//! - Asks order book (PDA)
//! - Event queue (PDA)
//! - Base token vault (PDA token account)
//! - Quote token vault (PDA token account)

use anchor_lang::prelude::*;

use crate::error::MatchbookError;
use crate::state::MarketStatus;
use crate::CreateMarket;

/// Size of the order book account (header only for now).
/// In production, this would include space for nodes.
pub const ORDERBOOK_ACCOUNT_SIZE: usize = 128;

/// Size of the event queue account (header only for now).
/// In production, this would include space for events.
pub const EVENT_QUEUE_ACCOUNT_SIZE: usize = 120;

/// Seed for base vault PDA derivation.
pub const BASE_VAULT_SEED: &[u8] = b"base_vault";

/// Seed for quote vault PDA derivation.
pub const QUOTE_VAULT_SEED: &[u8] = b"quote_vault";

/// Maximum taker fee in basis points (100% = 10000 bps).
pub const MAX_FEE_BPS: u16 = 10000;

/// Maximum maker rebate in basis points (1% = 100 bps).
pub const MAX_MAKER_REBATE_BPS: i16 = -100;

/// Maximum maker fee in basis points (100% = 10000 bps).
pub const MAX_MAKER_FEE_BPS: i16 = 10000;

/// Parameters for creating a new market.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct CreateMarketParams {
    /// Minimum price increment in quote atoms per base lot.
    pub tick_size: u64,
    /// Minimum base token quantity per lot (in base token smallest units).
    pub base_lot_size: u64,
    /// Minimum quote token quantity per lot (in quote token smallest units).
    pub quote_lot_size: u64,
    /// Minimum order size in lots.
    pub min_order_size: u64,
    /// Taker fee in basis points (1 bp = 0.01%).
    pub taker_fee_bps: u16,
    /// Maker fee in basis points. Negative values represent rebates.
    pub maker_fee_bps: i16,
}

impl CreateMarketParams {
    /// Validates the market parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if any parameter is invalid.
    pub fn validate(&self) -> Result<()> {
        // Validate tick size
        require!(self.tick_size > 0, MatchbookError::InvalidTickSize);

        // Validate lot sizes
        require!(self.base_lot_size > 0, MatchbookError::InvalidLotSize);
        require!(self.quote_lot_size > 0, MatchbookError::InvalidLotSize);

        // Validate min order size
        require!(self.min_order_size > 0, MatchbookError::OrderTooSmall);

        // Validate taker fee (0 to 100%)
        require!(
            self.taker_fee_bps <= MAX_FEE_BPS,
            MatchbookError::InvalidMarketState
        );

        // Validate maker fee (-1% rebate to 100% fee)
        require!(
            self.maker_fee_bps >= MAX_MAKER_REBATE_BPS && self.maker_fee_bps <= MAX_MAKER_FEE_BPS,
            MatchbookError::InvalidMarketState
        );

        Ok(())
    }
}

/// Handler for the CreateMarket instruction.
///
/// Creates a new trading market with all associated accounts.
///
/// # Arguments
///
/// * `ctx` - The instruction context containing all accounts
/// * `params` - Market configuration parameters
///
/// # Errors
///
/// Returns an error if:
/// - Parameters are invalid (zero tick/lot size, invalid fees)
/// - Base and quote mints are the same
pub fn handler(ctx: Context<CreateMarket>, params: CreateMarketParams) -> Result<()> {
    // Validate parameters
    params.validate()?;

    // Validate mints are different
    require!(
        ctx.accounts.base_mint.key() != ctx.accounts.quote_mint.key(),
        MatchbookError::InvalidMarketState
    );

    // Get market bump
    let market_bump = ctx.bumps.market;

    // Initialize market account
    let market = &mut ctx.accounts.market;
    market.bump = market_bump;
    market.status = MarketStatus::Active;
    market.base_mint = ctx.accounts.base_mint.key();
    market.quote_mint = ctx.accounts.quote_mint.key();
    market.base_vault = ctx.accounts.base_vault.key();
    market.quote_vault = ctx.accounts.quote_vault.key();
    market.bids = ctx.accounts.bids.key();
    market.asks = ctx.accounts.asks.key();
    market.event_queue = ctx.accounts.event_queue.key();
    market.authority = ctx.accounts.authority.key();
    market.fee_destination = ctx.accounts.fee_recipient.key();
    market.tick_size = params.tick_size;
    market.base_lot_size = params.base_lot_size;
    market.quote_lot_size = params.quote_lot_size;
    market.min_order_size = params.min_order_size;
    market.taker_fee_bps = params.taker_fee_bps;
    market.maker_fee_bps = params.maker_fee_bps;
    market.seq_num = 0;
    market.reserved = [0u8; 64];

    // Note: Bids, Asks, and EventQueue accounts are created as zero-initialized
    // by Anchor's init constraint. Their headers will be properly initialized
    // when first accessed by other instructions.

    // Emit market creation log
    msg!(
        "Market created: {} (base: {}, quote: {})",
        market.key(),
        ctx.accounts.base_mint.key(),
        ctx.accounts.quote_mint.key()
    );
    msg!(
        "Config: tick_size={}, base_lot_size={}, quote_lot_size={}, min_order_size={}",
        params.tick_size,
        params.base_lot_size,
        params.quote_lot_size,
        params.min_order_size
    );
    msg!(
        "Fees: taker={}bps, maker={}bps",
        params.taker_fee_bps,
        params.maker_fee_bps
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_market_params_valid() {
        let params = CreateMarketParams {
            tick_size: 100,
            base_lot_size: 1000,
            quote_lot_size: 1,
            min_order_size: 1,
            taker_fee_bps: 30,
            maker_fee_bps: -10,
        };

        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_create_market_params_zero_tick_size() {
        let params = CreateMarketParams {
            tick_size: 0,
            base_lot_size: 1000,
            quote_lot_size: 1,
            min_order_size: 1,
            taker_fee_bps: 30,
            maker_fee_bps: -10,
        };

        assert!(params.validate().is_err());
    }

    #[test]
    fn test_create_market_params_zero_base_lot_size() {
        let params = CreateMarketParams {
            tick_size: 100,
            base_lot_size: 0,
            quote_lot_size: 1,
            min_order_size: 1,
            taker_fee_bps: 30,
            maker_fee_bps: -10,
        };

        assert!(params.validate().is_err());
    }

    #[test]
    fn test_create_market_params_zero_quote_lot_size() {
        let params = CreateMarketParams {
            tick_size: 100,
            base_lot_size: 1000,
            quote_lot_size: 0,
            min_order_size: 1,
            taker_fee_bps: 30,
            maker_fee_bps: -10,
        };

        assert!(params.validate().is_err());
    }

    #[test]
    fn test_create_market_params_zero_min_order_size() {
        let params = CreateMarketParams {
            tick_size: 100,
            base_lot_size: 1000,
            quote_lot_size: 1,
            min_order_size: 0,
            taker_fee_bps: 30,
            maker_fee_bps: -10,
        };

        assert!(params.validate().is_err());
    }

    #[test]
    fn test_create_market_params_taker_fee_too_high() {
        let params = CreateMarketParams {
            tick_size: 100,
            base_lot_size: 1000,
            quote_lot_size: 1,
            min_order_size: 1,
            taker_fee_bps: 10001, // > 100%
            maker_fee_bps: -10,
        };

        assert!(params.validate().is_err());
    }

    #[test]
    fn test_create_market_params_maker_rebate_too_high() {
        let params = CreateMarketParams {
            tick_size: 100,
            base_lot_size: 1000,
            quote_lot_size: 1,
            min_order_size: 1,
            taker_fee_bps: 30,
            maker_fee_bps: -101, // > 1% rebate
        };

        assert!(params.validate().is_err());
    }

    #[test]
    fn test_create_market_params_maker_fee_too_high() {
        let params = CreateMarketParams {
            tick_size: 100,
            base_lot_size: 1000,
            quote_lot_size: 1,
            min_order_size: 1,
            taker_fee_bps: 30,
            maker_fee_bps: 10001, // > 100%
        };

        assert!(params.validate().is_err());
    }

    #[test]
    fn test_create_market_params_max_valid_fees() {
        // Max taker fee (100%)
        let params = CreateMarketParams {
            tick_size: 100,
            base_lot_size: 1000,
            quote_lot_size: 1,
            min_order_size: 1,
            taker_fee_bps: 10000,
            maker_fee_bps: 0,
        };
        assert!(params.validate().is_ok());

        // Max maker rebate (1%)
        let params = CreateMarketParams {
            tick_size: 100,
            base_lot_size: 1000,
            quote_lot_size: 1,
            min_order_size: 1,
            taker_fee_bps: 30,
            maker_fee_bps: -100,
        };
        assert!(params.validate().is_ok());

        // Max maker fee (100%)
        let params = CreateMarketParams {
            tick_size: 100,
            base_lot_size: 1000,
            quote_lot_size: 1,
            min_order_size: 1,
            taker_fee_bps: 30,
            maker_fee_bps: 10000,
        };
        assert!(params.validate().is_ok());
    }
}
