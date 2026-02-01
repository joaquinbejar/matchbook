//! PlaceOrder instruction builder.
//!
//! Builds the instruction to place a new order on the book.

use borsh::BorshSerialize;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

use crate::error::SdkError;
use crate::types::{OrderType, Price, Quantity, SelfTradeBehavior, Side};

use super::create_market::TOKEN_PROGRAM_ID;
use super::pda::{
    derive_asks_address, derive_base_vault_address, derive_bids_address,
    derive_event_queue_address, derive_open_orders_address, derive_quote_vault_address,
};

/// Parameters for placing an order (on-chain format).
#[derive(Debug, Clone, BorshSerialize)]
struct PlaceOrderInstructionData {
    side: u8,
    price_lots: u64,
    quantity_lots: u64,
    order_type: u8,
    client_order_id: u64,
    self_trade_behavior: u8,
    expiry_slot: u64,
}

/// Builder for the PlaceOrder instruction.
#[derive(Debug, Clone)]
pub struct PlaceOrderBuilder {
    program_id: Pubkey,
    owner: Option<Pubkey>,
    market: Option<Pubkey>,
    user_token_account: Option<Pubkey>,
    side: Option<Side>,
    price: Option<Price>,
    quantity: Option<Quantity>,
    order_type: OrderType,
    client_order_id: u64,
    self_trade_behavior: SelfTradeBehavior,
    expiry_slot: Option<u64>,
}

impl PlaceOrderBuilder {
    /// Creates a new builder.
    #[must_use]
    pub fn new(program_id: Pubkey) -> Self {
        Self {
            program_id,
            owner: None,
            market: None,
            user_token_account: None,
            side: None,
            price: None,
            quantity: None,
            order_type: OrderType::Limit,
            client_order_id: 0,
            self_trade_behavior: SelfTradeBehavior::default(),
            expiry_slot: None,
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

    /// Sets the user's token account for immediate deposit.
    #[must_use]
    pub fn user_token_account(mut self, account: Pubkey) -> Self {
        self.user_token_account = Some(account);
        self
    }

    /// Sets the order side.
    #[must_use]
    pub fn side(mut self, side: Side) -> Self {
        self.side = Some(side);
        self
    }

    /// Sets the price in ticks.
    #[must_use]
    pub fn price(mut self, price: Price) -> Self {
        self.price = Some(price);
        self
    }

    /// Sets the quantity in lots.
    #[must_use]
    pub fn quantity(mut self, quantity: Quantity) -> Self {
        self.quantity = Some(quantity);
        self
    }

    /// Sets the order type.
    #[must_use]
    pub fn order_type(mut self, order_type: OrderType) -> Self {
        self.order_type = order_type;
        self
    }

    /// Sets the client order ID.
    #[must_use]
    pub fn client_order_id(mut self, id: u64) -> Self {
        self.client_order_id = id;
        self
    }

    /// Sets the self-trade behavior.
    #[must_use]
    pub fn self_trade_behavior(mut self, stb: SelfTradeBehavior) -> Self {
        self.self_trade_behavior = stb;
        self
    }

    /// Sets the expiry slot.
    #[must_use]
    pub fn expiry_slot(mut self, slot: u64) -> Self {
        self.expiry_slot = Some(slot);
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
        let side = self
            .side
            .ok_or_else(|| SdkError::Serialization("side not set".to_string()))?;
        let price = self
            .price
            .ok_or_else(|| SdkError::InvalidPrice("price not set".to_string()))?;
        let quantity = self
            .quantity
            .ok_or_else(|| SdkError::InvalidQuantity("quantity not set".to_string()))?;

        // Derive PDAs
        let (open_orders, _) = derive_open_orders_address(&self.program_id, &market, &owner);
        let (bids, _) = derive_bids_address(&self.program_id, &market);
        let (asks, _) = derive_asks_address(&self.program_id, &market);
        let (event_queue, _) = derive_event_queue_address(&self.program_id, &market);
        let (base_vault, _) = derive_base_vault_address(&self.program_id, &market);
        let (quote_vault, _) = derive_quote_vault_address(&self.program_id, &market);

        // Build accounts list
        let mut accounts = vec![
            AccountMeta::new_readonly(owner, true),
            AccountMeta::new_readonly(market, false),
            AccountMeta::new(open_orders, false),
            AccountMeta::new(bids, false),
            AccountMeta::new(asks, false),
            AccountMeta::new(event_queue, false),
            AccountMeta::new(base_vault, false),
            AccountMeta::new(quote_vault, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        ];

        // Add optional user token account
        if let Some(user_token_account) = self.user_token_account {
            accounts.push(AccountMeta::new(user_token_account, false));
        }

        // Serialize instruction data
        let instruction_data = PlaceOrderInstructionData {
            side: side.into(),
            price_lots: price.value(),
            quantity_lots: quantity.value(),
            order_type: self.order_type.into(),
            client_order_id: self.client_order_id,
            self_trade_behavior: self.self_trade_behavior.into(),
            expiry_slot: self.expiry_slot.unwrap_or(0),
        };

        let mut data = vec![4u8; 8]; // Placeholder discriminator for PlaceOrder
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
    fn test_place_order_builder_new() {
        let program_id = test_program_id();
        let builder = PlaceOrderBuilder::new(program_id);
        assert_eq!(builder.program_id, program_id);
        assert_eq!(builder.order_type, OrderType::Limit);
    }

    #[test]
    fn test_place_order_builder_chain() {
        let program_id = test_program_id();
        let owner = Pubkey::new_unique();
        let market = Pubkey::new_unique();

        let builder = PlaceOrderBuilder::new(program_id)
            .owner(owner)
            .market(market)
            .side(Side::Bid)
            .price(Price::new(1000))
            .quantity(Quantity::new(100))
            .order_type(OrderType::PostOnly)
            .client_order_id(12345)
            .self_trade_behavior(SelfTradeBehavior::CancelMaker)
            .expiry_slot(100_000);

        assert_eq!(builder.owner, Some(owner));
        assert_eq!(builder.side, Some(Side::Bid));
        assert_eq!(builder.order_type, OrderType::PostOnly);
        assert_eq!(builder.client_order_id, 12345);
    }

    #[test]
    fn test_place_order_builder_build() {
        let program_id = test_program_id();
        let owner = Pubkey::new_unique();
        let market = Pubkey::new_unique();

        let ix = PlaceOrderBuilder::new(program_id)
            .owner(owner)
            .market(market)
            .side(Side::Bid)
            .price(Price::new(1000))
            .quantity(Quantity::new(100))
            .build()
            .expect("should build instruction");

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 9); // Without user_token_account
        assert!(ix.accounts[0].is_signer); // owner
    }

    #[test]
    fn test_place_order_builder_build_with_token_account() {
        let program_id = test_program_id();
        let owner = Pubkey::new_unique();
        let market = Pubkey::new_unique();
        let user_token = Pubkey::new_unique();

        let ix = PlaceOrderBuilder::new(program_id)
            .owner(owner)
            .market(market)
            .side(Side::Ask)
            .price(Price::new(2000))
            .quantity(Quantity::new(50))
            .user_token_account(user_token)
            .build()
            .expect("should build instruction");

        assert_eq!(ix.accounts.len(), 10); // With user_token_account
    }

    #[test]
    fn test_place_order_builder_build_missing_side() {
        let program_id = test_program_id();
        let owner = Pubkey::new_unique();
        let market = Pubkey::new_unique();

        let result = PlaceOrderBuilder::new(program_id)
            .owner(owner)
            .market(market)
            .price(Price::new(1000))
            .quantity(Quantity::new(100))
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_place_order_builder_build_missing_price() {
        let program_id = test_program_id();
        let owner = Pubkey::new_unique();
        let market = Pubkey::new_unique();

        let result = PlaceOrderBuilder::new(program_id)
            .owner(owner)
            .market(market)
            .side(Side::Bid)
            .quantity(Quantity::new(100))
            .build();

        assert!(result.is_err());
    }
}
