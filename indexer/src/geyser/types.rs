//! Types for Geyser listener.
//!
//! Defines account update structures and subscription filters.

use serde::{Deserialize, Serialize};

/// Connection state for the Geyser listener.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[repr(u8)]
pub enum ConnectionState {
    /// Not connected to Geyser endpoint.
    #[default]
    Disconnected = 0,
    /// Attempting to connect.
    Connecting = 1,
    /// Connected and subscribed.
    Connected = 2,
    /// Reconnecting after a disconnect.
    Reconnecting = 3,
    /// Connection failed permanently.
    Failed = 4,
}

impl ConnectionState {
    /// Returns true if the listener is connected.
    #[must_use]
    pub const fn is_connected(&self) -> bool {
        matches!(self, Self::Connected)
    }

    /// Returns true if the listener is in a transient state.
    #[must_use]
    pub const fn is_transient(&self) -> bool {
        matches!(self, Self::Connecting | Self::Reconnecting)
    }
}

/// Type of account based on its structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountType {
    /// Market configuration account.
    Market,
    /// Bids order book account.
    Bids,
    /// Asks order book account.
    Asks,
    /// Event queue account.
    EventQueue,
    /// User's open orders account.
    OpenOrders,
    /// Unknown account type.
    Unknown,
}

impl AccountType {
    /// Returns a human-readable name for the account type.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Market => "market",
            Self::Bids => "bids",
            Self::Asks => "asks",
            Self::EventQueue => "event_queue",
            Self::OpenOrders => "open_orders",
            Self::Unknown => "unknown",
        }
    }
}

/// An account update received from Geyser.
#[derive(Debug, Clone)]
pub struct AccountUpdate {
    /// Account public key (32 bytes).
    pub pubkey: [u8; 32],

    /// Slot number when the update occurred.
    pub slot: u64,

    /// Account data.
    pub data: Vec<u8>,

    /// Write version for ordering updates.
    pub write_version: u64,

    /// Whether this update is from the startup snapshot.
    pub is_startup: bool,

    /// Lamports balance of the account.
    pub lamports: u64,

    /// Owner program of the account.
    pub owner: [u8; 32],

    /// Whether the account is executable.
    pub executable: bool,

    /// Rent epoch of the account.
    pub rent_epoch: u64,
}

impl AccountUpdate {
    /// Returns the pubkey as a base58 string.
    #[must_use]
    pub fn pubkey_string(&self) -> String {
        bs58::encode(&self.pubkey).into_string()
    }

    /// Returns the owner as a base58 string.
    #[must_use]
    pub fn owner_string(&self) -> String {
        bs58::encode(&self.owner).into_string()
    }

    /// Returns the size of the account data.
    #[must_use]
    pub fn data_len(&self) -> usize {
        self.data.len()
    }
}

/// Filter for account subscriptions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubscriptionFilter {
    /// Program ID to filter by (owner of accounts).
    pub program_id: Option<String>,

    /// Specific account addresses to subscribe to.
    pub accounts: Vec<String>,

    /// Minimum data size filter.
    pub data_size: Option<u64>,
}

impl SubscriptionFilter {
    /// Creates a filter for a specific program.
    #[must_use]
    pub fn by_program(program_id: impl Into<String>) -> Self {
        Self {
            program_id: Some(program_id.into()),
            ..Default::default()
        }
    }

    /// Creates a filter for specific accounts.
    #[must_use]
    pub fn by_accounts(accounts: Vec<String>) -> Self {
        Self {
            accounts,
            ..Default::default()
        }
    }

    /// Adds a data size filter.
    #[must_use]
    pub const fn with_data_size(mut self, size: u64) -> Self {
        self.data_size = Some(size);
        self
    }

    /// Returns true if the filter is empty (matches everything).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.program_id.is_none() && self.accounts.is_empty() && self.data_size.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_state_default() {
        assert_eq!(ConnectionState::default(), ConnectionState::Disconnected);
    }

    #[test]
    fn test_connection_state_is_connected() {
        assert!(!ConnectionState::Disconnected.is_connected());
        assert!(!ConnectionState::Connecting.is_connected());
        assert!(ConnectionState::Connected.is_connected());
        assert!(!ConnectionState::Reconnecting.is_connected());
        assert!(!ConnectionState::Failed.is_connected());
    }

    #[test]
    fn test_connection_state_is_transient() {
        assert!(!ConnectionState::Disconnected.is_transient());
        assert!(ConnectionState::Connecting.is_transient());
        assert!(!ConnectionState::Connected.is_transient());
        assert!(ConnectionState::Reconnecting.is_transient());
        assert!(!ConnectionState::Failed.is_transient());
    }

    #[test]
    fn test_account_type_as_str() {
        assert_eq!(AccountType::Market.as_str(), "market");
        assert_eq!(AccountType::Bids.as_str(), "bids");
        assert_eq!(AccountType::Asks.as_str(), "asks");
        assert_eq!(AccountType::EventQueue.as_str(), "event_queue");
        assert_eq!(AccountType::OpenOrders.as_str(), "open_orders");
        assert_eq!(AccountType::Unknown.as_str(), "unknown");
    }

    #[test]
    fn test_account_update_pubkey_string() {
        let update = AccountUpdate {
            pubkey: [0u8; 32],
            slot: 100,
            data: vec![],
            write_version: 1,
            is_startup: false,
            lamports: 1000,
            owner: [0u8; 32],
            executable: false,
            rent_epoch: 0,
        };
        assert_eq!(update.pubkey_string(), "11111111111111111111111111111111");
    }

    #[test]
    fn test_subscription_filter_by_program() {
        let filter = SubscriptionFilter::by_program("MATCHBooK1111111111111111111111111111111111");
        assert_eq!(
            filter.program_id,
            Some("MATCHBooK1111111111111111111111111111111111".to_string())
        );
        assert!(filter.accounts.is_empty());
    }

    #[test]
    fn test_subscription_filter_by_accounts() {
        let accounts = vec!["account1".to_string(), "account2".to_string()];
        let filter = SubscriptionFilter::by_accounts(accounts.clone());
        assert!(filter.program_id.is_none());
        assert_eq!(filter.accounts, accounts);
    }

    #[test]
    fn test_subscription_filter_is_empty() {
        assert!(SubscriptionFilter::default().is_empty());
        assert!(!SubscriptionFilter::by_program("test").is_empty());
    }
}
