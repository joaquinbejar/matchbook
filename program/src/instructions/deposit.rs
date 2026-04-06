//! Deposit instruction for transferring tokens to market vaults.
//!
//! This instruction transfers tokens from a user's wallet to the market vaults,
//! crediting their OpenOrders account with the deposited amounts.
//!
//! # Token Flow
//!
//! User's base token account → Market base vault
//! User's quote token account → Market quote vault

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer};

use crate::error::MatchbookError;

/// Parameters for depositing tokens.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct DepositParams {
    /// Amount of base tokens to deposit (in base token smallest units).
    pub base_amount: u64,
    /// Amount of quote tokens to deposit (in quote token smallest units).
    pub quote_amount: u64,
}

impl DepositParams {
    /// Validates the deposit parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if both amounts are zero.
    pub fn validate(&self) -> Result<()> {
        require!(
            self.base_amount > 0 || self.quote_amount > 0,
            MatchbookError::InvalidQuantity
        );
        Ok(())
    }
}

/// Handler for the Deposit instruction.
///
/// Transfers tokens from user's accounts to market vaults and credits
/// the user's OpenOrders account.
///
/// # Arguments
///
/// * `ctx` - The instruction context containing all accounts
/// * `params` - Deposit amounts for base and quote tokens
///
/// # Errors
///
/// Returns an error if:
/// - Both amounts are zero
/// - Market is not active
/// - Balance overflow would occur
/// - Token transfer fails
pub fn handler(ctx: Context<crate::Deposit>, params: DepositParams) -> Result<()> {
    // Validate parameters
    params.validate()?;

    // Validate market is active
    require!(
        ctx.accounts.market.is_active(),
        MatchbookError::MarketNotActive
    );

    // Transfer base tokens if amount > 0
    if params.base_amount > 0 {
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.user_base_account.to_account_info(),
                    to: ctx.accounts.base_vault.to_account_info(),
                    authority: ctx.accounts.owner.to_account_info(),
                },
            ),
            params.base_amount,
        )?;

        // Update OpenOrders base_free balance
        ctx.accounts.open_orders.base_free = ctx
            .accounts
            .open_orders
            .base_free
            .checked_add(params.base_amount)
            .ok_or(MatchbookError::BalanceOverflow)?;
    }

    // Transfer quote tokens if amount > 0
    if params.quote_amount > 0 {
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.user_quote_account.to_account_info(),
                    to: ctx.accounts.quote_vault.to_account_info(),
                    authority: ctx.accounts.owner.to_account_info(),
                },
            ),
            params.quote_amount,
        )?;

        // Update OpenOrders quote_free balance
        ctx.accounts.open_orders.quote_free = ctx
            .accounts
            .open_orders
            .quote_free
            .checked_add(params.quote_amount)
            .ok_or(MatchbookError::BalanceOverflow)?;
    }

    // Emit deposit log
    msg!(
        "Deposit: base={}, quote={} (market: {}, owner: {})",
        params.base_amount,
        params.quote_amount,
        ctx.accounts.market.key(),
        ctx.accounts.owner.key()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deposit_params_valid_base_only() {
        let params = DepositParams {
            base_amount: 1000,
            quote_amount: 0,
        };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_deposit_params_valid_quote_only() {
        let params = DepositParams {
            base_amount: 0,
            quote_amount: 1000,
        };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_deposit_params_valid_both() {
        let params = DepositParams {
            base_amount: 1000,
            quote_amount: 2000,
        };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_deposit_params_both_zero() {
        let params = DepositParams {
            base_amount: 0,
            quote_amount: 0,
        };
        assert!(params.validate().is_err());
    }
}
