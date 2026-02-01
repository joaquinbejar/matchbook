//! CreateOpenOrders instruction builder.
//!
//! Builds the instruction to create an OpenOrders account for a user in a market.

use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

use crate::error::SdkError;

use super::pda::derive_open_orders_address;

/// Builder for the CreateOpenOrders instruction.
#[derive(Debug, Clone)]
pub struct CreateOpenOrdersBuilder {
    program_id: Pubkey,
    payer: Option<Pubkey>,
    owner: Option<Pubkey>,
    market: Option<Pubkey>,
}

impl CreateOpenOrdersBuilder {
    /// Creates a new builder.
    #[must_use]
    pub fn new(program_id: Pubkey) -> Self {
        Self {
            program_id,
            payer: None,
            owner: None,
            market: None,
        }
    }

    /// Sets the payer account.
    #[must_use]
    pub fn payer(mut self, payer: Pubkey) -> Self {
        self.payer = Some(payer);
        self
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

    /// Returns the derived open orders PDA.
    ///
    /// # Errors
    ///
    /// Returns an error if market or owner is not set.
    pub fn get_open_orders_address(&self) -> Result<(Pubkey, u8), SdkError> {
        let market = self
            .market
            .ok_or_else(|| SdkError::InvalidAddress("market not set".to_string()))?;
        let owner = self
            .owner
            .ok_or_else(|| SdkError::InvalidAddress("owner not set".to_string()))?;

        Ok(derive_open_orders_address(
            &self.program_id,
            &market,
            &owner,
        ))
    }

    /// Builds the instruction.
    ///
    /// # Errors
    ///
    /// Returns an error if any required field is not set.
    pub fn build(self) -> Result<Instruction, SdkError> {
        let payer = self
            .payer
            .ok_or_else(|| SdkError::InvalidAddress("payer not set".to_string()))?;
        let owner = self
            .owner
            .ok_or_else(|| SdkError::InvalidAddress("owner not set".to_string()))?;
        let market = self
            .market
            .ok_or_else(|| SdkError::InvalidAddress("market not set".to_string()))?;

        // Derive open orders PDA
        let (open_orders, _bump) = derive_open_orders_address(&self.program_id, &market, &owner);

        // Build accounts list
        let accounts = vec![
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(owner, true),
            AccountMeta::new_readonly(market, false),
            AccountMeta::new(open_orders, false),
            AccountMeta::new_readonly(
                solana_sdk::pubkey!("11111111111111111111111111111111"),
                false,
            ),
        ];

        // Instruction discriminator (8 bytes)
        let data = vec![1u8; 8]; // Placeholder discriminator for CreateOpenOrders

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
    fn test_create_open_orders_builder_new() {
        let program_id = test_program_id();
        let builder = CreateOpenOrdersBuilder::new(program_id);
        assert_eq!(builder.program_id, program_id);
    }

    #[test]
    fn test_create_open_orders_builder_chain() {
        let program_id = test_program_id();
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let market = Pubkey::new_unique();

        let builder = CreateOpenOrdersBuilder::new(program_id)
            .payer(payer)
            .owner(owner)
            .market(market);

        assert_eq!(builder.payer, Some(payer));
        assert_eq!(builder.owner, Some(owner));
        assert_eq!(builder.market, Some(market));
    }

    #[test]
    fn test_create_open_orders_builder_get_address() {
        let program_id = test_program_id();
        let owner = Pubkey::new_unique();
        let market = Pubkey::new_unique();

        let builder = CreateOpenOrdersBuilder::new(program_id)
            .owner(owner)
            .market(market);

        let (open_orders, bump) = builder.get_open_orders_address().expect("should derive");
        assert_ne!(open_orders, Pubkey::default());
        let _ = bump; // Bump is always valid u8
    }

    #[test]
    fn test_create_open_orders_builder_build() {
        let program_id = test_program_id();
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let market = Pubkey::new_unique();

        let ix = CreateOpenOrdersBuilder::new(program_id)
            .payer(payer)
            .owner(owner)
            .market(market)
            .build()
            .expect("should build instruction");

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 5);
        assert!(ix.accounts[0].is_signer); // payer
        assert!(ix.accounts[0].is_writable); // payer
        assert!(ix.accounts[1].is_signer); // owner
    }

    #[test]
    fn test_create_open_orders_builder_build_missing_owner() {
        let program_id = test_program_id();
        let payer = Pubkey::new_unique();
        let market = Pubkey::new_unique();

        let result = CreateOpenOrdersBuilder::new(program_id)
            .payer(payer)
            .market(market)
            .build();

        assert!(result.is_err());
    }
}
