//! CancelOrder instruction builder.
//!
//! Builds the instruction to cancel a specific order by ID.

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

/// Parameters for cancelling an order (on-chain format).
#[derive(Debug, Clone, BorshSerialize)]
struct CancelOrderInstructionData {
    order_id: u128,
    side: u8,
}

/// Builder for the CancelOrder instruction.
#[derive(Debug, Clone)]
pub struct CancelOrderBuilder {
    program_id: Pubkey,
    owner: Option<Pubkey>,
    market: Option<Pubkey>,
    order_id: Option<u128>,
    side: Option<Side>,
}

impl CancelOrderBuilder {
    /// Creates a new builder.
    #[must_use]
    pub fn new(program_id: Pubkey) -> Self {
        Self {
            program_id,
            owner: None,
            market: None,
            order_id: None,
            side: None,
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

    /// Sets the order ID to cancel.
    #[must_use]
    pub fn order_id(mut self, order_id: u128) -> Self {
        self.order_id = Some(order_id);
        self
    }

    /// Sets the order side.
    #[must_use]
    pub fn side(mut self, side: Side) -> Self {
        self.side = Some(side);
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
        let order_id = self
            .order_id
            .ok_or_else(|| SdkError::Serialization("order_id not set".to_string()))?;
        let side = self
            .side
            .ok_or_else(|| SdkError::Serialization("side not set".to_string()))?;

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

        // Serialize instruction data
        let instruction_data = CancelOrderInstructionData {
            order_id,
            side: side.into(),
        };

        let mut data = vec![5u8; 8]; // Placeholder discriminator for CancelOrder
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
    fn test_cancel_order_builder_new() {
        let program_id = test_program_id();
        let builder = CancelOrderBuilder::new(program_id);
        assert_eq!(builder.program_id, program_id);
    }

    #[test]
    fn test_cancel_order_builder_chain() {
        let program_id = test_program_id();
        let owner = Pubkey::new_unique();
        let market = Pubkey::new_unique();

        let builder = CancelOrderBuilder::new(program_id)
            .owner(owner)
            .market(market)
            .order_id(12345678901234567890)
            .side(Side::Bid);

        assert_eq!(builder.owner, Some(owner));
        assert_eq!(builder.order_id, Some(12345678901234567890));
        assert_eq!(builder.side, Some(Side::Bid));
    }

    #[test]
    fn test_cancel_order_builder_build() {
        let program_id = test_program_id();
        let owner = Pubkey::new_unique();
        let market = Pubkey::new_unique();

        let ix = CancelOrderBuilder::new(program_id)
            .owner(owner)
            .market(market)
            .order_id(12345678901234567890)
            .side(Side::Bid)
            .build()
            .expect("should build instruction");

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 6);
        assert!(ix.accounts[0].is_signer); // owner
    }

    #[test]
    fn test_cancel_order_builder_build_missing_order_id() {
        let program_id = test_program_id();
        let owner = Pubkey::new_unique();
        let market = Pubkey::new_unique();

        let result = CancelOrderBuilder::new(program_id)
            .owner(owner)
            .market(market)
            .side(Side::Bid)
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_cancel_order_builder_build_missing_side() {
        let program_id = test_program_id();
        let owner = Pubkey::new_unique();
        let market = Pubkey::new_unique();

        let result = CancelOrderBuilder::new(program_id)
            .owner(owner)
            .market(market)
            .order_id(12345678901234567890)
            .build();

        assert!(result.is_err());
    }
}
