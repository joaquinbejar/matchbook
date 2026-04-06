//! Withdraw instruction for transferring tokens from market vaults to user.
//!
//! This instruction transfers tokens from the market vaults back to a user's wallet,
//! debiting their OpenOrders account. Only the owner can withdraw (not delegate).
//!
//! # Token Flow
//!
//! Market base vault → User's base token account
//! Market quote vault → User's quote token account

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer};

use crate::error::MatchbookError;

/// Parameters for withdrawing tokens.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct WithdrawParams {
    /// Amount of base tokens to withdraw (in base token smallest units).
    pub base_amount: u64,
    /// Amount of quote tokens to withdraw (in quote token smallest units).
    pub quote_amount: u64,
}

impl WithdrawParams {
    /// Validates the withdraw parameters.
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

/// Handler for the Withdraw instruction.
///
/// Transfers tokens from market vaults to user's accounts and debits
/// the user's OpenOrders account. Only the owner can withdraw.
///
/// # Arguments
///
/// * `ctx` - The instruction context containing all accounts
/// * `params` - Withdraw amounts for base and quote tokens
///
/// # Errors
///
/// Returns an error if:
/// - Both amounts are zero
/// - Insufficient free balance
/// - Token transfer fails
pub fn handler(ctx: Context<crate::Withdraw>, params: WithdrawParams) -> Result<()> {
    // Validate parameters
    params.validate()?;

    // Get market signer seeds for PDA transfer
    let market = &ctx.accounts.market;
    let base_mint = market.base_mint;
    let quote_mint = market.quote_mint;
    let bump = market.bump;
    let signer_seeds: &[&[&[u8]]] = &[&[
        crate::MARKET_SEED,
        base_mint.as_ref(),
        quote_mint.as_ref(),
        &[bump],
    ]];

    // Withdraw base tokens if amount > 0
    if params.base_amount > 0 {
        // Check sufficient free balance
        require!(
            ctx.accounts.open_orders.base_free >= params.base_amount,
            MatchbookError::InsufficientFunds
        );

        // Transfer from vault to user
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.base_vault.to_account_info(),
                    to: ctx.accounts.user_base_account.to_account_info(),
                    authority: ctx.accounts.market.to_account_info(),
                },
                signer_seeds,
            ),
            params.base_amount,
        )?;

        // Update OpenOrders base_free balance
        ctx.accounts.open_orders.base_free = ctx
            .accounts
            .open_orders
            .base_free
            .checked_sub(params.base_amount)
            .ok_or(MatchbookError::InsufficientFunds)?;
    }

    // Withdraw quote tokens if amount > 0
    if params.quote_amount > 0 {
        // Check sufficient free balance
        require!(
            ctx.accounts.open_orders.quote_free >= params.quote_amount,
            MatchbookError::InsufficientFunds
        );

        // Transfer from vault to user
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.quote_vault.to_account_info(),
                    to: ctx.accounts.user_quote_account.to_account_info(),
                    authority: ctx.accounts.market.to_account_info(),
                },
                signer_seeds,
            ),
            params.quote_amount,
        )?;

        // Update OpenOrders quote_free balance
        ctx.accounts.open_orders.quote_free = ctx
            .accounts
            .open_orders
            .quote_free
            .checked_sub(params.quote_amount)
            .ok_or(MatchbookError::InsufficientFunds)?;
    }

    // Emit withdraw log
    msg!(
        "Withdraw: base={}, quote={} (market: {}, owner: {})",
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
    fn test_withdraw_params_valid_base_only() {
        let params = WithdrawParams {
            base_amount: 1000,
            quote_amount: 0,
        };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_withdraw_params_valid_quote_only() {
        let params = WithdrawParams {
            base_amount: 0,
            quote_amount: 1000,
        };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_withdraw_params_valid_both() {
        let params = WithdrawParams {
            base_amount: 1000,
            quote_amount: 2000,
        };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_withdraw_params_both_zero() {
        let params = WithdrawParams {
            base_amount: 0,
            quote_amount: 0,
        };
        assert!(params.validate().is_err());
    }
}
