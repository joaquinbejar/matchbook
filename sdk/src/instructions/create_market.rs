//! CreateMarket instruction builder.
//!
//! Builds the instruction to create a new market with all associated accounts.

use borsh::BorshSerialize;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    sysvar,
};

use crate::error::SdkError;

use super::pda::MarketPdas;

/// SPL Token program ID.
pub const TOKEN_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// Parameters for creating a market.
#[derive(Debug, Clone, BorshSerialize)]
pub struct CreateMarketParams {
    /// Minimum price increment (quote atoms per tick).
    pub tick_size: u64,
    /// Minimum quantity increment (base atoms per lot).
    pub lot_size: u64,
    /// Minimum order size in lots.
    pub min_order_size: u64,
    /// Taker fee in basis points.
    pub taker_fee_bps: u16,
    /// Maker fee in basis points (can be negative for rebates).
    pub maker_fee_bps: i16,
}

/// Builder for the CreateMarket instruction.
#[derive(Debug, Clone)]
pub struct CreateMarketBuilder {
    program_id: Pubkey,
    payer: Option<Pubkey>,
    base_mint: Option<Pubkey>,
    quote_mint: Option<Pubkey>,
    authority: Option<Pubkey>,
    fee_recipient: Option<Pubkey>,
    params: Option<CreateMarketParams>,
}

impl CreateMarketBuilder {
    /// Creates a new builder.
    #[must_use]
    pub fn new(program_id: Pubkey) -> Self {
        Self {
            program_id,
            payer: None,
            base_mint: None,
            quote_mint: None,
            authority: None,
            fee_recipient: None,
            params: None,
        }
    }

    /// Sets the payer account.
    #[must_use]
    pub fn payer(mut self, payer: Pubkey) -> Self {
        self.payer = Some(payer);
        self
    }

    /// Sets the base mint.
    #[must_use]
    pub fn base_mint(mut self, base_mint: Pubkey) -> Self {
        self.base_mint = Some(base_mint);
        self
    }

    /// Sets the quote mint.
    #[must_use]
    pub fn quote_mint(mut self, quote_mint: Pubkey) -> Self {
        self.quote_mint = Some(quote_mint);
        self
    }

    /// Sets the market authority.
    #[must_use]
    pub fn authority(mut self, authority: Pubkey) -> Self {
        self.authority = Some(authority);
        self
    }

    /// Sets the fee recipient.
    #[must_use]
    pub fn fee_recipient(mut self, fee_recipient: Pubkey) -> Self {
        self.fee_recipient = Some(fee_recipient);
        self
    }

    /// Sets the market parameters.
    #[must_use]
    pub fn params(mut self, params: CreateMarketParams) -> Self {
        self.params = Some(params);
        self
    }

    /// Sets the tick size.
    #[must_use]
    pub fn tick_size(mut self, tick_size: u64) -> Self {
        let params = self.params.get_or_insert(CreateMarketParams {
            tick_size: 0,
            lot_size: 0,
            min_order_size: 0,
            taker_fee_bps: 0,
            maker_fee_bps: 0,
        });
        params.tick_size = tick_size;
        self
    }

    /// Sets the lot size.
    #[must_use]
    pub fn lot_size(mut self, lot_size: u64) -> Self {
        let params = self.params.get_or_insert(CreateMarketParams {
            tick_size: 0,
            lot_size: 0,
            min_order_size: 0,
            taker_fee_bps: 0,
            maker_fee_bps: 0,
        });
        params.lot_size = lot_size;
        self
    }

    /// Sets the minimum order size.
    #[must_use]
    pub fn min_order_size(mut self, min_order_size: u64) -> Self {
        let params = self.params.get_or_insert(CreateMarketParams {
            tick_size: 0,
            lot_size: 0,
            min_order_size: 0,
            taker_fee_bps: 0,
            maker_fee_bps: 0,
        });
        params.min_order_size = min_order_size;
        self
    }

    /// Sets the taker fee in basis points.
    #[must_use]
    pub fn taker_fee_bps(mut self, taker_fee_bps: u16) -> Self {
        let params = self.params.get_or_insert(CreateMarketParams {
            tick_size: 0,
            lot_size: 0,
            min_order_size: 0,
            taker_fee_bps: 0,
            maker_fee_bps: 0,
        });
        params.taker_fee_bps = taker_fee_bps;
        self
    }

    /// Sets the maker fee in basis points.
    #[must_use]
    pub fn maker_fee_bps(mut self, maker_fee_bps: i16) -> Self {
        let params = self.params.get_or_insert(CreateMarketParams {
            tick_size: 0,
            lot_size: 0,
            min_order_size: 0,
            taker_fee_bps: 0,
            maker_fee_bps: 0,
        });
        params.maker_fee_bps = maker_fee_bps;
        self
    }

    /// Returns the derived market PDAs.
    ///
    /// # Errors
    ///
    /// Returns an error if base_mint or quote_mint is not set.
    pub fn get_pdas(&self) -> Result<MarketPdas, SdkError> {
        let base_mint = self
            .base_mint
            .ok_or_else(|| SdkError::InvalidAddress("base_mint not set".to_string()))?;
        let quote_mint = self
            .quote_mint
            .ok_or_else(|| SdkError::InvalidAddress("quote_mint not set".to_string()))?;

        Ok(MarketPdas::derive(
            &self.program_id,
            &base_mint,
            &quote_mint,
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
        let base_mint = self
            .base_mint
            .ok_or_else(|| SdkError::InvalidAddress("base_mint not set".to_string()))?;
        let quote_mint = self
            .quote_mint
            .ok_or_else(|| SdkError::InvalidAddress("quote_mint not set".to_string()))?;
        let authority = self
            .authority
            .ok_or_else(|| SdkError::InvalidAddress("authority not set".to_string()))?;
        let fee_recipient = self
            .fee_recipient
            .ok_or_else(|| SdkError::InvalidAddress("fee_recipient not set".to_string()))?;
        let params = self
            .params
            .ok_or_else(|| SdkError::Serialization("params not set".to_string()))?;

        // Derive PDAs
        let pdas = MarketPdas::derive(&self.program_id, &base_mint, &quote_mint);

        // Build accounts list
        let accounts = vec![
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(base_mint, false),
            AccountMeta::new_readonly(quote_mint, false),
            AccountMeta::new(pdas.market, false),
            AccountMeta::new(pdas.bids, false),
            AccountMeta::new(pdas.asks, false),
            AccountMeta::new(pdas.event_queue, false),
            AccountMeta::new(pdas.base_vault, false),
            AccountMeta::new(pdas.quote_vault, false),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(fee_recipient, false),
            AccountMeta::new_readonly(
                solana_sdk::pubkey!("11111111111111111111111111111111"),
                false,
            ),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(sysvar::rent::ID, false),
        ];

        // Serialize instruction data
        // Instruction discriminator (8 bytes) + params
        let mut data = vec![0u8; 8]; // Placeholder discriminator
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
    fn test_create_market_builder_new() {
        let program_id = test_program_id();
        let builder = CreateMarketBuilder::new(program_id);
        assert_eq!(builder.program_id, program_id);
    }

    #[test]
    fn test_create_market_builder_chain() {
        let program_id = test_program_id();
        let payer = Pubkey::new_unique();
        let base_mint = Pubkey::new_unique();
        let quote_mint = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let fee_recipient = Pubkey::new_unique();

        let builder = CreateMarketBuilder::new(program_id)
            .payer(payer)
            .base_mint(base_mint)
            .quote_mint(quote_mint)
            .authority(authority)
            .fee_recipient(fee_recipient)
            .tick_size(100)
            .lot_size(1000)
            .min_order_size(1)
            .taker_fee_bps(10)
            .maker_fee_bps(-5);

        assert_eq!(builder.payer, Some(payer));
        assert_eq!(builder.base_mint, Some(base_mint));
        assert_eq!(builder.quote_mint, Some(quote_mint));
    }

    #[test]
    fn test_create_market_builder_get_pdas() {
        let program_id = test_program_id();
        let base_mint = Pubkey::new_unique();
        let quote_mint = Pubkey::new_unique();

        let builder = CreateMarketBuilder::new(program_id)
            .base_mint(base_mint)
            .quote_mint(quote_mint);

        let pdas = builder.get_pdas().expect("should derive PDAs");
        assert_ne!(pdas.market, Pubkey::default());
    }

    #[test]
    fn test_create_market_builder_get_pdas_missing_mint() {
        let program_id = test_program_id();
        let builder = CreateMarketBuilder::new(program_id);

        let result = builder.get_pdas();
        assert!(result.is_err());
    }

    #[test]
    fn test_create_market_builder_build() {
        let program_id = test_program_id();
        let payer = Pubkey::new_unique();
        let base_mint = Pubkey::new_unique();
        let quote_mint = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let fee_recipient = Pubkey::new_unique();

        let ix = CreateMarketBuilder::new(program_id)
            .payer(payer)
            .base_mint(base_mint)
            .quote_mint(quote_mint)
            .authority(authority)
            .fee_recipient(fee_recipient)
            .params(CreateMarketParams {
                tick_size: 100,
                lot_size: 1000,
                min_order_size: 1,
                taker_fee_bps: 10,
                maker_fee_bps: -5,
            })
            .build()
            .expect("should build instruction");

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 14);
        assert!(ix.accounts[0].is_signer); // payer
        assert!(ix.accounts[0].is_writable); // payer
    }

    #[test]
    fn test_create_market_builder_build_missing_payer() {
        let program_id = test_program_id();
        let base_mint = Pubkey::new_unique();
        let quote_mint = Pubkey::new_unique();

        let result = CreateMarketBuilder::new(program_id)
            .base_mint(base_mint)
            .quote_mint(quote_mint)
            .build();

        assert!(result.is_err());
    }
}
