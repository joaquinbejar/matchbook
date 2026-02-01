//! MatchOrders instruction builder.
//!
//! Builds the instruction to trigger matching of crossed orders.

use borsh::BorshSerialize;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

use crate::error::SdkError;

use super::create_market::TOKEN_PROGRAM_ID;
use super::pda::{
    derive_asks_address, derive_base_vault_address, derive_bids_address,
    derive_event_queue_address, derive_quote_vault_address,
};

/// Parameters for matching orders (on-chain format).
#[derive(Debug, Clone, BorshSerialize)]
struct MatchOrdersInstructionData {
    /// Maximum matches to execute.
    limit: u8,
}

/// Builder for the MatchOrders instruction.
#[derive(Debug, Clone)]
pub struct MatchOrdersBuilder {
    program_id: Pubkey,
    market: Option<Pubkey>,
    limit: u8,
    /// Remaining accounts: OpenOrders for makers being matched.
    maker_open_orders: Vec<Pubkey>,
}

impl MatchOrdersBuilder {
    /// Creates a new builder.
    #[must_use]
    pub fn new(program_id: Pubkey) -> Self {
        Self {
            program_id,
            market: None,
            limit: 10, // Default limit
            maker_open_orders: Vec::new(),
        }
    }

    /// Sets the market.
    #[must_use]
    pub fn market(mut self, market: Pubkey) -> Self {
        self.market = Some(market);
        self
    }

    /// Sets the maximum number of matches to execute.
    #[must_use]
    pub fn limit(mut self, limit: u8) -> Self {
        self.limit = limit;
        self
    }

    /// Adds a maker's open orders account.
    #[must_use]
    pub fn add_maker_open_orders(mut self, open_orders: Pubkey) -> Self {
        self.maker_open_orders.push(open_orders);
        self
    }

    /// Sets all maker open orders accounts.
    #[must_use]
    pub fn maker_open_orders(mut self, accounts: Vec<Pubkey>) -> Self {
        self.maker_open_orders = accounts;
        self
    }

    /// Builds the instruction.
    ///
    /// # Errors
    ///
    /// Returns an error if any required field is not set.
    pub fn build(self) -> Result<Instruction, SdkError> {
        let market = self
            .market
            .ok_or_else(|| SdkError::InvalidAddress("market not set".to_string()))?;

        // Derive PDAs
        let (bids, _) = derive_bids_address(&self.program_id, &market);
        let (asks, _) = derive_asks_address(&self.program_id, &market);
        let (event_queue, _) = derive_event_queue_address(&self.program_id, &market);
        let (base_vault, _) = derive_base_vault_address(&self.program_id, &market);
        let (quote_vault, _) = derive_quote_vault_address(&self.program_id, &market);

        // Build accounts list
        let mut accounts = vec![
            AccountMeta::new_readonly(market, false),
            AccountMeta::new(bids, false),
            AccountMeta::new(asks, false),
            AccountMeta::new(event_queue, false),
            AccountMeta::new(base_vault, false),
            AccountMeta::new(quote_vault, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        ];

        // Add maker open orders as remaining accounts
        for open_orders in &self.maker_open_orders {
            accounts.push(AccountMeta::new(*open_orders, false));
        }

        // Serialize instruction data
        let instruction_data = MatchOrdersInstructionData { limit: self.limit };

        let mut data = vec![7u8; 8]; // Placeholder discriminator for MatchOrders
        data.extend(
            borsh::to_vec(&instruction_data).map_err(|e| SdkError::Serialization(e.to_string()))?,
        );

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
    fn test_match_orders_builder_new() {
        let program_id = test_program_id();
        let builder = MatchOrdersBuilder::new(program_id);
        assert_eq!(builder.program_id, program_id);
        assert_eq!(builder.limit, 10);
    }

    #[test]
    fn test_match_orders_builder_chain() {
        let program_id = test_program_id();
        let market = Pubkey::new_unique();
        let maker1 = Pubkey::new_unique();
        let maker2 = Pubkey::new_unique();

        let builder = MatchOrdersBuilder::new(program_id)
            .market(market)
            .limit(5)
            .add_maker_open_orders(maker1)
            .add_maker_open_orders(maker2);

        assert_eq!(builder.market, Some(market));
        assert_eq!(builder.limit, 5);
        assert_eq!(builder.maker_open_orders.len(), 2);
    }

    #[test]
    fn test_match_orders_builder_build() {
        let program_id = test_program_id();
        let market = Pubkey::new_unique();

        let ix = MatchOrdersBuilder::new(program_id)
            .market(market)
            .build()
            .expect("should build instruction");

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 7); // Without maker accounts
    }

    #[test]
    fn test_match_orders_builder_build_with_makers() {
        let program_id = test_program_id();
        let market = Pubkey::new_unique();
        let maker1 = Pubkey::new_unique();
        let maker2 = Pubkey::new_unique();

        let ix = MatchOrdersBuilder::new(program_id)
            .market(market)
            .maker_open_orders(vec![maker1, maker2])
            .build()
            .expect("should build instruction");

        assert_eq!(ix.accounts.len(), 9); // 7 + 2 makers
    }

    #[test]
    fn test_match_orders_builder_build_missing_market() {
        let program_id = test_program_id();

        let result = MatchOrdersBuilder::new(program_id).build();

        assert!(result.is_err());
    }
}
