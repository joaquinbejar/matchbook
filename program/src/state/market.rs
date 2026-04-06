//! Market account structure for the Matchbook CLOB.
//!
//! The Market account is the central account that holds all market-level
//! configuration and state. It references all other market components
//! including order books, event queue, and token vaults.
//!
//! # PDA Derivation
//!
//! Markets are derived using the seeds: `["market", base_mint, quote_mint]`
//!
//! # Example
//!
//! ```ignore
//! let (market_pda, bump) = Market::derive_pda(
//!     &base_mint.key(),
//!     &quote_mint.key(),
//!     &program_id,
//! );
//! ```

use anchor_lang::prelude::*;

/// Seed prefix for Market PDA derivation.
pub const MARKET_SEED: &[u8] = b"market";

/// Market account structure holding all market-level configuration and state.
///
/// This is the central account that references all other market components:
/// - Order books (bids and asks)
/// - Event queue for off-chain indexing
/// - Token vaults for base and quote tokens
///
/// # Space Calculation
///
/// ```text
/// 8 (discriminator) + 1 (bump) + 1 (status) + 32*10 (pubkeys)
/// + 8*5 (u64s) + 2 + 2 (fees) + 64 (reserved) = 438 bytes
/// ```
#[account]
#[derive(Debug, InitSpace)]
pub struct Market {
    /// Bump seed for PDA derivation.
    pub bump: u8,

    /// Current market status.
    pub status: MarketStatus,

    /// Base token mint address.
    pub base_mint: Pubkey,

    /// Quote token mint address.
    pub quote_mint: Pubkey,

    /// Base token vault (PDA-controlled token account).
    pub base_vault: Pubkey,

    /// Quote token vault (PDA-controlled token account).
    pub quote_vault: Pubkey,

    /// Bids order book account address.
    pub bids: Pubkey,

    /// Asks order book account address.
    pub asks: Pubkey,

    /// Event queue account address for off-chain indexing.
    pub event_queue: Pubkey,

    /// Authority that can modify market parameters.
    pub authority: Pubkey,

    /// Fee recipient account for collected fees.
    pub fee_destination: Pubkey,

    /// Minimum base token quantity per lot (in base token smallest units).
    pub base_lot_size: u64,

    /// Minimum quote token quantity per lot (in quote token smallest units).
    pub quote_lot_size: u64,

    /// Minimum price increment in quote atoms per base lot.
    pub tick_size: u64,

    /// Minimum order size in lots.
    pub min_order_size: u64,

    /// Taker fee in basis points (1 bp = 0.01%).
    pub taker_fee_bps: u16,

    /// Maker fee in basis points. Negative values represent rebates.
    pub maker_fee_bps: i16,

    /// Sequence number for event ordering, incremented on each event.
    pub seq_num: u64,

    /// Reserved space for future use.
    #[max_len(64)]
    pub reserved: [u8; 64],
}

impl Market {
    /// Seed prefix for PDA derivation.
    pub const SEED_PREFIX: &'static [u8] = MARKET_SEED;

    /// Derives the Market PDA address from base and quote mints.
    ///
    /// # Arguments
    ///
    /// * `base_mint` - The base token mint address
    /// * `quote_mint` - The quote token mint address
    /// * `program_id` - The program ID
    ///
    /// # Returns
    ///
    /// A tuple of (PDA address, bump seed)
    #[must_use]
    pub fn derive_pda(
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        program_id: &Pubkey,
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[Self::SEED_PREFIX, base_mint.as_ref(), quote_mint.as_ref()],
            program_id,
        )
    }

    /// Returns the seeds for PDA signing with the stored bump.
    ///
    /// # Arguments
    ///
    /// * `base_mint` - The base token mint address
    /// * `quote_mint` - The quote token mint address
    ///
    /// # Returns
    ///
    /// Seeds array for use in CPI signing
    #[must_use]
    pub fn signer_seeds<'a>(
        &'a self,
        base_mint: &'a Pubkey,
        quote_mint: &'a Pubkey,
    ) -> [&'a [u8]; 4] {
        [
            Self::SEED_PREFIX,
            base_mint.as_ref(),
            quote_mint.as_ref(),
            std::slice::from_ref(&self.bump),
        ]
    }

    /// Checks if the market is active and allows all operations.
    #[must_use]
    #[inline]
    pub const fn is_active(&self) -> bool {
        matches!(self.status, MarketStatus::Active)
    }

    /// Checks if the market is paused (only cancellations allowed).
    #[must_use]
    #[inline]
    pub const fn is_paused(&self) -> bool {
        matches!(self.status, MarketStatus::Paused)
    }

    /// Checks if the market is closed (no operations allowed).
    #[must_use]
    #[inline]
    pub const fn is_closed(&self) -> bool {
        matches!(self.status, MarketStatus::Closed)
    }

    /// Increments the sequence number and returns the new value.
    ///
    /// # Returns
    ///
    /// The new sequence number, or None if overflow would occur.
    #[must_use]
    pub fn next_seq_num(&mut self) -> Option<u64> {
        self.seq_num = self.seq_num.checked_add(1)?;
        Some(self.seq_num)
    }
}

/// Market operational status.
///
/// Controls which operations are allowed on the market.
#[derive(
    AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, Default, InitSpace,
)]
#[repr(u8)]
pub enum MarketStatus {
    /// Normal operation - all actions allowed.
    #[default]
    Active,

    /// Paused - only cancellations and withdrawals allowed.
    Paused,

    /// Closed - market is permanently closed, only withdrawals allowed.
    Closed,
}

impl MarketStatus {
    /// Returns true if placing new orders is allowed.
    #[must_use]
    #[inline]
    pub const fn allows_new_orders(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Returns true if cancelling orders is allowed.
    #[must_use]
    #[inline]
    pub const fn allows_cancellations(&self) -> bool {
        matches!(self, Self::Active | Self::Paused)
    }

    /// Returns true if withdrawals are allowed.
    #[must_use]
    #[inline]
    pub const fn allows_withdrawals(&self) -> bool {
        // Withdrawals are always allowed
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_pda_derivation() {
        let base_mint = Pubkey::new_unique();
        let quote_mint = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();

        let (pda1, bump1) = Market::derive_pda(&base_mint, &quote_mint, &program_id);
        let (pda2, bump2) = Market::derive_pda(&base_mint, &quote_mint, &program_id);

        // Same inputs should produce same outputs
        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);

        // Different mints should produce different PDAs
        let different_base = Pubkey::new_unique();
        let (pda3, _) = Market::derive_pda(&different_base, &quote_mint, &program_id);
        assert_ne!(pda1, pda3);

        // Swapped mints should produce different PDAs
        let (pda4, _) = Market::derive_pda(&quote_mint, &base_mint, &program_id);
        assert_ne!(pda1, pda4);
    }

    #[test]
    fn test_market_pda_is_valid() {
        let base_mint = Pubkey::new_unique();
        let quote_mint = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();

        let (pda, bump) = Market::derive_pda(&base_mint, &quote_mint, &program_id);

        // Verify the PDA can be recreated with the bump
        let recreated = Pubkey::create_program_address(
            &[
                Market::SEED_PREFIX,
                base_mint.as_ref(),
                quote_mint.as_ref(),
                &[bump],
            ],
            &program_id,
        );

        assert!(recreated.is_ok());
        assert_eq!(pda, recreated.unwrap_or_default());
    }

    #[test]
    fn test_market_status_default() {
        let status = MarketStatus::default();
        assert_eq!(status, MarketStatus::Active);
    }

    #[test]
    fn test_market_status_allows_new_orders() {
        assert!(MarketStatus::Active.allows_new_orders());
        assert!(!MarketStatus::Paused.allows_new_orders());
        assert!(!MarketStatus::Closed.allows_new_orders());
    }

    #[test]
    fn test_market_status_allows_cancellations() {
        assert!(MarketStatus::Active.allows_cancellations());
        assert!(MarketStatus::Paused.allows_cancellations());
        assert!(!MarketStatus::Closed.allows_cancellations());
    }

    #[test]
    fn test_market_status_allows_withdrawals() {
        assert!(MarketStatus::Active.allows_withdrawals());
        assert!(MarketStatus::Paused.allows_withdrawals());
        assert!(MarketStatus::Closed.allows_withdrawals());
    }

    #[test]
    fn test_market_is_active() {
        let mut market = create_test_market(MarketStatus::Active);
        assert!(market.is_active());
        assert!(!market.is_paused());
        assert!(!market.is_closed());

        market.status = MarketStatus::Paused;
        assert!(!market.is_active());
        assert!(market.is_paused());
        assert!(!market.is_closed());

        market.status = MarketStatus::Closed;
        assert!(!market.is_active());
        assert!(!market.is_paused());
        assert!(market.is_closed());
    }

    #[test]
    fn test_market_next_seq_num() {
        let mut market = create_test_market(MarketStatus::Active);
        market.seq_num = 0;

        assert_eq!(market.next_seq_num(), Some(1));
        assert_eq!(market.seq_num, 1);

        assert_eq!(market.next_seq_num(), Some(2));
        assert_eq!(market.seq_num, 2);
    }

    #[test]
    fn test_market_next_seq_num_overflow() {
        let mut market = create_test_market(MarketStatus::Active);
        market.seq_num = u64::MAX;

        assert_eq!(market.next_seq_num(), None);
        assert_eq!(market.seq_num, u64::MAX);
    }

    /// Helper function to create a test market with default values.
    fn create_test_market(status: MarketStatus) -> Market {
        Market {
            bump: 255,
            status,
            base_mint: Pubkey::default(),
            quote_mint: Pubkey::default(),
            base_vault: Pubkey::default(),
            quote_vault: Pubkey::default(),
            bids: Pubkey::default(),
            asks: Pubkey::default(),
            event_queue: Pubkey::default(),
            authority: Pubkey::default(),
            fee_destination: Pubkey::default(),
            base_lot_size: 1,
            quote_lot_size: 1,
            tick_size: 1,
            min_order_size: 1,
            taker_fee_bps: 30,
            maker_fee_bps: -10,
            seq_num: 0,
            reserved: [0u8; 64],
        }
    }
}
