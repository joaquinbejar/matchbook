//! CreateOpenOrders instruction for initializing a user's trading account.
//!
//! This instruction creates an OpenOrders account for a user in a specific market.
//! Users must create this account before they can deposit funds or place orders.
//!
//! # PDA Derivation
//!
//! OpenOrders: `["open_orders", market, owner]`

use anchor_lang::prelude::*;

use crate::error::MatchbookError;
use crate::state::OrderSlot;

/// Parameters for creating an OpenOrders account.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct CreateOpenOrdersParams {
    /// Optional delegate that can place/cancel orders on behalf of the owner.
    pub delegate: Option<Pubkey>,
}

/// Handler for the CreateOpenOrders instruction.
///
/// Creates an OpenOrders account for a user in a market.
///
/// # Arguments
///
/// * `ctx` - The instruction context containing all accounts
/// * `params` - Optional delegate configuration
///
/// # Errors
///
/// Returns an error if the market is not active.
pub fn handler(
    ctx: Context<crate::CreateOpenOrders>,
    params: CreateOpenOrdersParams,
) -> Result<()> {
    // Validate market is active
    require!(
        ctx.accounts.market.is_active(),
        MatchbookError::MarketNotActive
    );

    // Get bump
    let bump = ctx.bumps.open_orders;

    // Initialize OpenOrders account
    let open_orders = &mut ctx.accounts.open_orders;
    open_orders.bump = bump;
    open_orders.market = ctx.accounts.market.key();
    open_orders.owner = ctx.accounts.owner.key();
    open_orders.delegate = params.delegate.unwrap_or_default();
    open_orders.base_locked = 0;
    open_orders.quote_locked = 0;
    open_orders.base_free = 0;
    open_orders.quote_free = 0;
    open_orders.referrer_rebates = 0;
    open_orders.num_orders = 0;
    open_orders.reserved = [0u8; 64];
    open_orders.orders = [OrderSlot::default(); crate::MAX_ORDERS];

    // Emit creation log
    msg!(
        "OpenOrders created: {} (market: {}, owner: {})",
        open_orders.key(),
        ctx.accounts.market.key(),
        ctx.accounts.owner.key()
    );

    if params.delegate.is_some() {
        msg!("Delegate: {}", open_orders.delegate);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_open_orders_params_no_delegate() {
        let params = CreateOpenOrdersParams { delegate: None };
        assert!(params.delegate.is_none());
    }

    #[test]
    fn test_create_open_orders_params_with_delegate() {
        let delegate = Pubkey::new_unique();
        let params = CreateOpenOrdersParams {
            delegate: Some(delegate),
        };
        assert_eq!(params.delegate, Some(delegate));
    }
}
