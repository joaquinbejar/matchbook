//! ConsumeEvents instruction builder.
//!
//! Builds the instruction to process events from the event queue.

use borsh::BorshSerialize;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

use crate::error::SdkError;

use super::pda::derive_event_queue_address;

/// Parameters for consuming events (on-chain format).
#[derive(Debug, Clone, BorshSerialize)]
struct ConsumeEventsInstructionData {
    /// Maximum events to process.
    limit: u16,
}

/// Builder for the ConsumeEvents instruction.
#[derive(Debug, Clone)]
pub struct ConsumeEventsBuilder {
    program_id: Pubkey,
    market: Option<Pubkey>,
    limit: u16,
    /// Remaining accounts: OpenOrders accounts for users in events.
    user_open_orders: Vec<Pubkey>,
}

impl ConsumeEventsBuilder {
    /// Creates a new builder.
    #[must_use]
    pub fn new(program_id: Pubkey) -> Self {
        Self {
            program_id,
            market: None,
            limit: 10, // Default limit
            user_open_orders: Vec::new(),
        }
    }

    /// Sets the market.
    #[must_use]
    pub fn market(mut self, market: Pubkey) -> Self {
        self.market = Some(market);
        self
    }

    /// Sets the maximum number of events to process.
    #[must_use]
    pub fn limit(mut self, limit: u16) -> Self {
        self.limit = limit;
        self
    }

    /// Adds a user's open orders account.
    #[must_use]
    pub fn add_user_open_orders(mut self, open_orders: Pubkey) -> Self {
        self.user_open_orders.push(open_orders);
        self
    }

    /// Sets all user open orders accounts.
    #[must_use]
    pub fn user_open_orders(mut self, accounts: Vec<Pubkey>) -> Self {
        self.user_open_orders = accounts;
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
        let (event_queue, _) = derive_event_queue_address(&self.program_id, &market);

        // Build accounts list
        let mut accounts = vec![
            AccountMeta::new_readonly(market, false),
            AccountMeta::new(event_queue, false),
        ];

        // Add user open orders as remaining accounts
        for open_orders in &self.user_open_orders {
            accounts.push(AccountMeta::new(*open_orders, false));
        }

        // Serialize instruction data
        let instruction_data = ConsumeEventsInstructionData { limit: self.limit };

        let mut data = vec![8u8; 8]; // Placeholder discriminator for ConsumeEvents
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
    fn test_consume_events_builder_new() {
        let program_id = test_program_id();
        let builder = ConsumeEventsBuilder::new(program_id);
        assert_eq!(builder.program_id, program_id);
        assert_eq!(builder.limit, 10);
    }

    #[test]
    fn test_consume_events_builder_chain() {
        let program_id = test_program_id();
        let market = Pubkey::new_unique();
        let user1 = Pubkey::new_unique();
        let user2 = Pubkey::new_unique();

        let builder = ConsumeEventsBuilder::new(program_id)
            .market(market)
            .limit(20)
            .add_user_open_orders(user1)
            .add_user_open_orders(user2);

        assert_eq!(builder.market, Some(market));
        assert_eq!(builder.limit, 20);
        assert_eq!(builder.user_open_orders.len(), 2);
    }

    #[test]
    fn test_consume_events_builder_build() {
        let program_id = test_program_id();
        let market = Pubkey::new_unique();

        let ix = ConsumeEventsBuilder::new(program_id)
            .market(market)
            .build()
            .expect("should build instruction");

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 2); // Without user accounts
    }

    #[test]
    fn test_consume_events_builder_build_with_users() {
        let program_id = test_program_id();
        let market = Pubkey::new_unique();
        let user1 = Pubkey::new_unique();
        let user2 = Pubkey::new_unique();
        let user3 = Pubkey::new_unique();

        let ix = ConsumeEventsBuilder::new(program_id)
            .market(market)
            .user_open_orders(vec![user1, user2, user3])
            .build()
            .expect("should build instruction");

        assert_eq!(ix.accounts.len(), 5); // 2 + 3 users
    }

    #[test]
    fn test_consume_events_builder_build_missing_market() {
        let program_id = test_program_id();

        let result = ConsumeEventsBuilder::new(program_id).build();

        assert!(result.is_err());
    }
}
