//! Balance types for the Matchbook SDK.
//!
//! Provides user balance types.

use std::fmt;

use serde::{Deserialize, Serialize};

/// User balances for a market.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Balance {
    /// Owner address (base58 encoded).
    pub owner: String,

    /// Market address (base58 encoded).
    pub market: String,

    /// Free base token balance (available for trading).
    pub base_free: u64,

    /// Locked base token balance (in open orders).
    pub base_locked: u64,

    /// Free quote token balance (available for trading).
    pub quote_free: u64,

    /// Locked quote token balance (in open orders).
    pub quote_locked: u64,

    /// Accumulated referrer rebates.
    #[serde(default)]
    pub referrer_rebates: u64,

    /// Slot of the balance snapshot.
    pub slot: u64,

    /// Timestamp in milliseconds since epoch.
    pub timestamp: u64,
}

impl Balance {
    /// Returns the total base balance (free + locked).
    #[must_use]
    pub fn base_total(&self) -> u64 {
        self.base_free.saturating_add(self.base_locked)
    }

    /// Returns the total quote balance (free + locked).
    #[must_use]
    pub fn quote_total(&self) -> u64 {
        self.quote_free.saturating_add(self.quote_locked)
    }

    /// Returns true if there are any locked balances.
    #[must_use]
    pub fn has_locked(&self) -> bool {
        self.base_locked > 0 || self.quote_locked > 0
    }

    /// Returns true if all balances are zero.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.base_free == 0
            && self.base_locked == 0
            && self.quote_free == 0
            && self.quote_locked == 0
    }

    /// Returns true if there are any referrer rebates to claim.
    #[must_use]
    pub fn has_rebates(&self) -> bool {
        self.referrer_rebates > 0
    }
}

impl fmt::Display for Balance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Balance(base: {}/{}, quote: {}/{})",
            self.base_free, self.base_locked, self.quote_free, self.quote_locked
        )
    }
}

/// Summary of all balances across markets.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceSummary {
    /// Owner address (base58 encoded).
    pub owner: String,

    /// Balances per market.
    pub balances: Vec<Balance>,

    /// Total number of markets with balances.
    pub market_count: u32,
}

impl BalanceSummary {
    /// Returns the total base balance across all markets.
    #[must_use]
    pub fn total_base(&self) -> u64 {
        self.balances.iter().map(|b| b.base_total()).sum()
    }

    /// Returns the total quote balance across all markets.
    #[must_use]
    pub fn total_quote(&self) -> u64 {
        self.balances.iter().map(|b| b.quote_total()).sum()
    }

    /// Returns the total referrer rebates across all markets.
    #[must_use]
    pub fn total_rebates(&self) -> u64 {
        self.balances.iter().map(|b| b.referrer_rebates).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_balance() -> Balance {
        Balance {
            owner: "owner123".to_string(),
            market: "market123".to_string(),
            base_free: 1000,
            base_locked: 200,
            quote_free: 5000,
            quote_locked: 1000,
            referrer_rebates: 50,
            slot: 12345,
            timestamp: 1234567890,
        }
    }

    #[test]
    fn test_balance_base_total() {
        let balance = create_test_balance();
        assert_eq!(balance.base_total(), 1200); // 1000 + 200
    }

    #[test]
    fn test_balance_quote_total() {
        let balance = create_test_balance();
        assert_eq!(balance.quote_total(), 6000); // 5000 + 1000
    }

    #[test]
    fn test_balance_has_locked() {
        let balance = create_test_balance();
        assert!(balance.has_locked());

        let no_locked = Balance {
            base_locked: 0,
            quote_locked: 0,
            ..create_test_balance()
        };
        assert!(!no_locked.has_locked());
    }

    #[test]
    fn test_balance_is_empty() {
        let balance = create_test_balance();
        assert!(!balance.is_empty());

        let empty = Balance {
            base_free: 0,
            base_locked: 0,
            quote_free: 0,
            quote_locked: 0,
            ..create_test_balance()
        };
        assert!(empty.is_empty());
    }

    #[test]
    fn test_balance_has_rebates() {
        let balance = create_test_balance();
        assert!(balance.has_rebates());

        let no_rebates = Balance {
            referrer_rebates: 0,
            ..create_test_balance()
        };
        assert!(!no_rebates.has_rebates());
    }

    #[test]
    fn test_balance_display() {
        let balance = create_test_balance();
        assert_eq!(
            balance.to_string(),
            "Balance(base: 1000/200, quote: 5000/1000)"
        );
    }

    #[test]
    fn test_balance_serde() {
        let balance = create_test_balance();
        let json = serde_json::to_string(&balance).expect("serialize");
        let parsed: Balance = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(parsed.owner, balance.owner);
        assert_eq!(parsed.base_free, balance.base_free);
        assert_eq!(parsed.quote_locked, balance.quote_locked);
    }

    #[test]
    fn test_balance_summary_totals() {
        let summary = BalanceSummary {
            owner: "owner123".to_string(),
            balances: vec![
                Balance {
                    base_free: 100,
                    base_locked: 50,
                    quote_free: 1000,
                    quote_locked: 500,
                    referrer_rebates: 10,
                    ..create_test_balance()
                },
                Balance {
                    base_free: 200,
                    base_locked: 100,
                    quote_free: 2000,
                    quote_locked: 1000,
                    referrer_rebates: 20,
                    ..create_test_balance()
                },
            ],
            market_count: 2,
        };

        assert_eq!(summary.total_base(), 450); // (100+50) + (200+100)
        assert_eq!(summary.total_quote(), 4500); // (1000+500) + (2000+1000)
        assert_eq!(summary.total_rebates(), 30); // 10 + 20
    }
}
