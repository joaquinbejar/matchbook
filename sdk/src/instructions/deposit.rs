//! Deposit instruction builder.
//!
//! Builds the instruction to deposit tokens into a user's OpenOrders account.

use borsh::BorshSerialize;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

use crate::error::SdkError;

use super::create_market::TOKEN_PROGRAM_ID;
use super::pda::{
    derive_base_vault_address, derive_open_orders_address, derive_quote_vault_address,
};

/// Parameters for depositing tokens.
#[derive(Debug, Clone, BorshSerialize)]
pub struct DepositParams {
    /// Amount of base tokens to deposit.
    pub base_amount: u64,
    /// Amount of quote tokens to deposit.
    pub quote_amount: u64,
}

/// Builder for the Deposit instruction.
#[derive(Debug, Clone)]
pub struct DepositBuilder {
    program_id: Pubkey,
    owner: Option<Pubkey>,
    market: Option<Pubkey>,
    user_base_account: Option<Pubkey>,
    user_quote_account: Option<Pubkey>,
    base_amount: u64,
    quote_amount: u64,
}

impl DepositBuilder {
    /// Creates a new builder.
    #[must_use]
    pub fn new(program_id: Pubkey) -> Self {
        Self {
            program_id,
            owner: None,
            market: None,
            user_base_account: None,
            user_quote_account: None,
            base_amount: 0,
            quote_amount: 0,
        }
    }

    /// Sets the owner account.
    #[must_use]
    pub fn owner(mut self, owner: Pubkey) -> Self {
        self.owner = Some(owner);
        self
    }

    /// Sets the market.
    #[must_use]
    pub fn market(mut self, market: Pubkey) -> Self {
        self.market = Some(market);
        self
    }

    /// Sets the user's base token account.
    #[must_use]
    pub fn user_base_account(mut self, account: Pubkey) -> Self {
        self.user_base_account = Some(account);
        self
    }

    /// Sets the user's quote token account.
    #[must_use]
    pub fn user_quote_account(mut self, account: Pubkey) -> Self {
        self.user_quote_account = Some(account);
        self
    }

    /// Sets the base amount to deposit.
    #[must_use]
    pub fn base_amount(mut self, amount: u64) -> Self {
        self.base_amount = amount;
        self
    }

    /// Sets the quote amount to deposit.
    #[must_use]
    pub fn quote_amount(mut self, amount: u64) -> Self {
        self.quote_amount = amount;
        self
    }

    /// Builds the instruction.
    ///
    /// # Errors
    ///
    /// Returns an error if any required field is not set.
    pub fn build(self) -> Result<Instruction, SdkError> {
        let owner = self
            .owner
            .ok_or_else(|| SdkError::InvalidAddress("owner not set".to_string()))?;
        let market = self
            .market
            .ok_or_else(|| SdkError::InvalidAddress("market not set".to_string()))?;
        let user_base_account = self
            .user_base_account
            .ok_or_else(|| SdkError::InvalidAddress("user_base_account not set".to_string()))?;
        let user_quote_account = self
            .user_quote_account
            .ok_or_else(|| SdkError::InvalidAddress("user_quote_account not set".to_string()))?;

        // Derive PDAs
        let (open_orders, _) = derive_open_orders_address(&self.program_id, &market, &owner);
        let (base_vault, _) = derive_base_vault_address(&self.program_id, &market);
        let (quote_vault, _) = derive_quote_vault_address(&self.program_id, &market);

        // Build accounts list
        let accounts = vec![
            AccountMeta::new_readonly(owner, true),
            AccountMeta::new_readonly(market, false),
            AccountMeta::new(open_orders, false),
            AccountMeta::new(user_base_account, false),
            AccountMeta::new(user_quote_account, false),
            AccountMeta::new(base_vault, false),
            AccountMeta::new(quote_vault, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        ];

        // Serialize instruction data
        let params = DepositParams {
            base_amount: self.base_amount,
            quote_amount: self.quote_amount,
        };

        let mut data = vec![2u8; 8]; // Placeholder discriminator for Deposit
        data.extend(borsh::to_vec(&params).map_err(|e| SdkError::Serialization(e.to_string()))?);

        Ok(Instruction {
            program_id: self.program_id,
            accounts,
            data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_program_id() -> Pubkey {
        Pubkey::new_unique()
    }

    #[test]
    fn test_deposit_builder_new() {
        let program_id = test_program_id();
        let builder = DepositBuilder::new(program_id);
        assert_eq!(builder.program_id, program_id);
        assert_eq!(builder.base_amount, 0);
        assert_eq!(builder.quote_amount, 0);
    }

    #[test]
    fn test_deposit_builder_chain() {
        let program_id = test_program_id();
        let owner = Pubkey::new_unique();
        let market = Pubkey::new_unique();
        let user_base = Pubkey::new_unique();
        let user_quote = Pubkey::new_unique();

        let builder = DepositBuilder::new(program_id)
            .owner(owner)
            .market(market)
            .user_base_account(user_base)
            .user_quote_account(user_quote)
            .base_amount(1000)
            .quote_amount(5000);

        assert_eq!(builder.owner, Some(owner));
        assert_eq!(builder.market, Some(market));
        assert_eq!(builder.base_amount, 1000);
        assert_eq!(builder.quote_amount, 5000);
    }

    #[test]
    fn test_deposit_builder_build() {
        let program_id = test_program_id();
        let owner = Pubkey::new_unique();
        let market = Pubkey::new_unique();
        let user_base = Pubkey::new_unique();
        let user_quote = Pubkey::new_unique();

        let ix = DepositBuilder::new(program_id)
            .owner(owner)
            .market(market)
            .user_base_account(user_base)
            .user_quote_account(user_quote)
            .base_amount(1000)
            .quote_amount(5000)
            .build()
            .expect("should build instruction");

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 8);
        assert!(ix.accounts[0].is_signer); // owner
    }

    #[test]
    fn test_deposit_builder_build_missing_owner() {
        let program_id = test_program_id();
        let market = Pubkey::new_unique();
        let user_base = Pubkey::new_unique();
        let user_quote = Pubkey::new_unique();

        let result = DepositBuilder::new(program_id)
            .market(market)
            .user_base_account(user_base)
            .user_quote_account(user_quote)
            .build();

        assert!(result.is_err());
    }
}
