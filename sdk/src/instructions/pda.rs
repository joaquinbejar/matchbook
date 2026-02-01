//! PDA derivation utilities for Matchbook accounts.
//!
//! Provides functions to derive Program Derived Addresses (PDAs) for all
//! Matchbook accounts.

use solana_sdk::pubkey::Pubkey;

/// Seeds for market PDA derivation.
pub const MARKET_SEED: &[u8] = b"market";

/// Seeds for bids account PDA derivation.
pub const BIDS_SEED: &[u8] = b"bids";

/// Seeds for asks account PDA derivation.
pub const ASKS_SEED: &[u8] = b"asks";

/// Seeds for event queue PDA derivation.
pub const EVENT_QUEUE_SEED: &[u8] = b"event_queue";

/// Seeds for base vault PDA derivation.
pub const BASE_VAULT_SEED: &[u8] = b"base_vault";

/// Seeds for quote vault PDA derivation.
pub const QUOTE_VAULT_SEED: &[u8] = b"quote_vault";

/// Seeds for open orders PDA derivation.
pub const OPEN_ORDERS_SEED: &[u8] = b"open_orders";

/// Derives the market PDA.
///
/// Seeds: `[b"market", base_mint, quote_mint]`
#[must_use]
pub fn derive_market_address(
    program_id: &Pubkey,
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[MARKET_SEED, base_mint.as_ref(), quote_mint.as_ref()],
        program_id,
    )
}

/// Derives the bids account PDA.
///
/// Seeds: `[market, b"bids"]`
#[must_use]
pub fn derive_bids_address(program_id: &Pubkey, market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[market.as_ref(), BIDS_SEED], program_id)
}

/// Derives the asks account PDA.
///
/// Seeds: `[market, b"asks"]`
#[must_use]
pub fn derive_asks_address(program_id: &Pubkey, market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[market.as_ref(), ASKS_SEED], program_id)
}

/// Derives the event queue PDA.
///
/// Seeds: `[market, b"event_queue"]`
#[must_use]
pub fn derive_event_queue_address(program_id: &Pubkey, market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[market.as_ref(), EVENT_QUEUE_SEED], program_id)
}

/// Derives the base vault PDA.
///
/// Seeds: `[market, b"base_vault"]`
#[must_use]
pub fn derive_base_vault_address(program_id: &Pubkey, market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[market.as_ref(), BASE_VAULT_SEED], program_id)
}

/// Derives the quote vault PDA.
///
/// Seeds: `[market, b"quote_vault"]`
#[must_use]
pub fn derive_quote_vault_address(program_id: &Pubkey, market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[market.as_ref(), QUOTE_VAULT_SEED], program_id)
}

/// Derives the open orders PDA.
///
/// Seeds: `[b"open_orders", market, owner]`
#[must_use]
pub fn derive_open_orders_address(
    program_id: &Pubkey,
    market: &Pubkey,
    owner: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[OPEN_ORDERS_SEED, market.as_ref(), owner.as_ref()],
        program_id,
    )
}

/// Collection of all market-related PDAs.
#[derive(Debug, Clone)]
pub struct MarketPdas {
    /// Market address.
    pub market: Pubkey,
    /// Market bump.
    pub market_bump: u8,
    /// Bids address.
    pub bids: Pubkey,
    /// Bids bump.
    pub bids_bump: u8,
    /// Asks address.
    pub asks: Pubkey,
    /// Asks bump.
    pub asks_bump: u8,
    /// Event queue address.
    pub event_queue: Pubkey,
    /// Event queue bump.
    pub event_queue_bump: u8,
    /// Base vault address.
    pub base_vault: Pubkey,
    /// Base vault bump.
    pub base_vault_bump: u8,
    /// Quote vault address.
    pub quote_vault: Pubkey,
    /// Quote vault bump.
    pub quote_vault_bump: u8,
}

impl MarketPdas {
    /// Derives all market PDAs.
    #[must_use]
    pub fn derive(program_id: &Pubkey, base_mint: &Pubkey, quote_mint: &Pubkey) -> Self {
        let (market, market_bump) = derive_market_address(program_id, base_mint, quote_mint);
        let (bids, bids_bump) = derive_bids_address(program_id, &market);
        let (asks, asks_bump) = derive_asks_address(program_id, &market);
        let (event_queue, event_queue_bump) = derive_event_queue_address(program_id, &market);
        let (base_vault, base_vault_bump) = derive_base_vault_address(program_id, &market);
        let (quote_vault, quote_vault_bump) = derive_quote_vault_address(program_id, &market);

        Self {
            market,
            market_bump,
            bids,
            bids_bump,
            asks,
            asks_bump,
            event_queue,
            event_queue_bump,
            base_vault,
            base_vault_bump,
            quote_vault,
            quote_vault_bump,
        }
    }

    /// Derives market PDAs from an existing market address.
    #[must_use]
    pub fn from_market(program_id: &Pubkey, market: &Pubkey) -> Self {
        let (bids, bids_bump) = derive_bids_address(program_id, market);
        let (asks, asks_bump) = derive_asks_address(program_id, market);
        let (event_queue, event_queue_bump) = derive_event_queue_address(program_id, market);
        let (base_vault, base_vault_bump) = derive_base_vault_address(program_id, market);
        let (quote_vault, quote_vault_bump) = derive_quote_vault_address(program_id, market);

        Self {
            market: *market,
            market_bump: 0, // Unknown when deriving from market address
            bids,
            bids_bump,
            asks,
            asks_bump,
            event_queue,
            event_queue_bump,
            base_vault,
            base_vault_bump,
            quote_vault,
            quote_vault_bump,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_program_id() -> Pubkey {
        Pubkey::new_unique()
    }

    fn test_mint() -> Pubkey {
        Pubkey::new_unique()
    }

    #[test]
    fn test_derive_market_address() {
        let program_id = test_program_id();
        let base_mint = test_mint();
        let quote_mint = test_mint();

        let (market, bump) = derive_market_address(&program_id, &base_mint, &quote_mint);

        assert_ne!(market, Pubkey::default());
        let _ = bump; // Bump is always valid u8

        // Same inputs should give same output
        let (market2, bump2) = derive_market_address(&program_id, &base_mint, &quote_mint);
        assert_eq!(market, market2);
        assert_eq!(bump, bump2);
    }

    #[test]
    fn test_derive_bids_address() {
        let program_id = test_program_id();
        let market = test_mint();

        let (bids, bump) = derive_bids_address(&program_id, &market);

        assert_ne!(bids, Pubkey::default());
        let _ = bump; // Bump is always valid u8
    }

    #[test]
    fn test_derive_asks_address() {
        let program_id = test_program_id();
        let market = test_mint();

        let (asks, bump) = derive_asks_address(&program_id, &market);

        assert_ne!(asks, Pubkey::default());
        let _ = bump; // Bump is always valid u8
    }

    #[test]
    fn test_derive_event_queue_address() {
        let program_id = test_program_id();
        let market = test_mint();

        let (event_queue, bump) = derive_event_queue_address(&program_id, &market);

        assert_ne!(event_queue, Pubkey::default());
        let _ = bump; // Bump is always valid u8
    }

    #[test]
    fn test_derive_base_vault_address() {
        let program_id = test_program_id();
        let market = test_mint();

        let (base_vault, bump) = derive_base_vault_address(&program_id, &market);

        assert_ne!(base_vault, Pubkey::default());
        let _ = bump; // Bump is always valid u8
    }

    #[test]
    fn test_derive_quote_vault_address() {
        let program_id = test_program_id();
        let market = test_mint();

        let (quote_vault, bump) = derive_quote_vault_address(&program_id, &market);

        assert_ne!(quote_vault, Pubkey::default());
        let _ = bump; // Bump is always valid u8
    }

    #[test]
    fn test_derive_open_orders_address() {
        let program_id = test_program_id();
        let market = test_mint();
        let owner = test_mint();

        let (open_orders, bump) = derive_open_orders_address(&program_id, &market, &owner);

        assert_ne!(open_orders, Pubkey::default());
        let _ = bump; // Bump is always valid u8

        // Different owner should give different address
        let owner2 = test_mint();
        let (open_orders2, _) = derive_open_orders_address(&program_id, &market, &owner2);
        assert_ne!(open_orders, open_orders2);
    }

    #[test]
    fn test_market_pdas_derive() {
        let program_id = test_program_id();
        let base_mint = test_mint();
        let quote_mint = test_mint();

        let pdas = MarketPdas::derive(&program_id, &base_mint, &quote_mint);

        assert_ne!(pdas.market, Pubkey::default());
        assert_ne!(pdas.bids, Pubkey::default());
        assert_ne!(pdas.asks, Pubkey::default());
        assert_ne!(pdas.event_queue, Pubkey::default());
        assert_ne!(pdas.base_vault, Pubkey::default());
        assert_ne!(pdas.quote_vault, Pubkey::default());

        // All addresses should be unique
        let addresses = [
            pdas.market,
            pdas.bids,
            pdas.asks,
            pdas.event_queue,
            pdas.base_vault,
            pdas.quote_vault,
        ];
        for i in 0..addresses.len() {
            for j in (i + 1)..addresses.len() {
                assert_ne!(addresses[i], addresses[j]);
            }
        }
    }

    #[test]
    fn test_market_pdas_from_market() {
        let program_id = test_program_id();
        let market = test_mint();

        let pdas = MarketPdas::from_market(&program_id, &market);

        assert_eq!(pdas.market, market);
        assert_ne!(pdas.bids, Pubkey::default());
        assert_ne!(pdas.asks, Pubkey::default());
    }
}
