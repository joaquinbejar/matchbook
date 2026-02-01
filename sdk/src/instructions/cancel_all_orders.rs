//! CancelAllOrders instruction builder.
//!
//! Builds the instruction to cancel all orders for a user in a market.

use borsh::BorshSerialize;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

use crate::error::SdkError;
use crate::types::Side;

use super::pda::{
    derive_asks_address, derive_bids_address, derive_event_queue_address,
    derive_open_orders_address,
};

/// Parameters for cancelling all orders (on-chain format).
#[derive(Debug, Clone, BorshSerialize)]
struct CancelAllOrdersInstructionData {
    /// Side to cancel (0 = both, 1 = bid, 2 = ask).
    side_option: u8,
    /// Maximum orders to cancel per transaction.
    limit: u8,
}

/// Builder for the CancelAllOrders instruction.
#[derive(Debug, Clone)]
pub struct CancelAllOrdersBuilder {
    program_id: Pubkey,
    owner: Option<Pubkey>,
    market: Option<Pubkey>,
    side: Option<Side>,
    limit: u8,
}

impl CancelAllOrdersBuilder {
    /// Creates a new builder.
    #[must_use]
    pub fn new(program_id: Pubkey) -> Self {
        Self {
            program_id,
            owner: None,
            market: None,
            side: None,
            limit: 10, // Default limit
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

    /// Sets the side to cancel (None = both sides).
    #[must_use]
    pub fn side(mut self, side: Option<Side>) -> Self {
        self.side = side;
        self
    }

    /// Sets the maximum number of orders to cancel.
    #[must_use]
    pub fn limit(mut self, limit: u8) -> Self {
        self.limit = limit;
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

        // Derive PDAs
        let (open_orders, _) = derive_open_orders_address(&self.program_id, &market, &owner);
        let (bids, _) = derive_bids_address(&self.program_id, &market);
        let (asks, _) = derive_asks_address(&self.program_id, &market);
        let (event_queue, _) = derive_event_queue_address(&self.program_id, &market);

        // Build accounts list
        let accounts = vec![
            AccountMeta::new_readonly(owner, true),
            AccountMeta::new_readonly(market, false),
            AccountMeta::new(open_orders, false),
            AccountMeta::new(bids, false),
            AccountMeta::new(asks, false),
            AccountMeta::new(event_queue, false),
        ];

        // Encode side option: 0 = both, 1 = bid, 2 = ask
        let side_option = match self.side {
            None => 0,
            Some(Side::Bid) => 1,
            Some(Side::Ask) => 2,
        };

        // Serialize instruction data
        let instruction_data = CancelAllOrdersInstructionData {
            side_option,
            limit: self.limit,
        };

        let mut data = vec![6u8; 8]; // Placeholder discriminator for CancelAllOrders
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
    fn test_cancel_all_orders_builder_new() {
        let program_id = test_program_id();
        let builder = CancelAllOrdersBuilder::new(program_id);
        assert_eq!(builder.program_id, program_id);
        assert_eq!(builder.limit, 10);
    }

    #[test]
    fn test_cancel_all_orders_builder_chain() {
        let program_id = test_program_id();
        let owner = Pubkey::new_unique();
        let market = Pubkey::new_unique();

        let builder = CancelAllOrdersBuilder::new(program_id)
            .owner(owner)
            .market(market)
            .side(Some(Side::Bid))
            .limit(20);

        assert_eq!(builder.owner, Some(owner));
        assert_eq!(builder.side, Some(Side::Bid));
        assert_eq!(builder.limit, 20);
    }

    #[test]
    fn test_cancel_all_orders_builder_build() {
        let program_id = test_program_id();
        let owner = Pubkey::new_unique();
        let market = Pubkey::new_unique();

        let ix = CancelAllOrdersBuilder::new(program_id)
            .owner(owner)
            .market(market)
            .build()
            .expect("should build instruction");

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 6);
        assert!(ix.accounts[0].is_signer); // owner
    }

    #[test]
    fn test_cancel_all_orders_builder_build_with_side() {
        let program_id = test_program_id();
        let owner = Pubkey::new_unique();
        let market = Pubkey::new_unique();

        let ix = CancelAllOrdersBuilder::new(program_id)
            .owner(owner)
            .market(market)
            .side(Some(Side::Ask))
            .limit(5)
            .build()
            .expect("should build instruction");

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 6);
    }

    #[test]
    fn test_cancel_all_orders_builder_build_missing_owner() {
        let program_id = test_program_id();
        let market = Pubkey::new_unique();

        let result = CancelAllOrdersBuilder::new(program_id)
            .market(market)
            .build();

        assert!(result.is_err());
    }
}
