//! Channel management for WebSocket subscriptions.
//!
//! Provides channel parsing and subscription management.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::sync::RwLock;

/// Channel type for subscriptions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Channel {
    /// Order book updates for a market.
    Book {
        /// Market address bytes.
        market: [u8; 32],
    },

    /// Trade updates for a market.
    Trades {
        /// Market address bytes.
        market: [u8; 32],
    },

    /// Order updates for a user (requires auth).
    Orders {
        /// Owner address bytes.
        owner: [u8; 32],
    },
}

impl Channel {
    /// Parses a channel string into a Channel.
    ///
    /// Format: `type:address` where type is `book`, `trades`, or `orders`.
    ///
    /// # Errors
    ///
    /// Returns None if the format is invalid or address cannot be decoded.
    #[must_use]
    pub fn parse(channel: &str) -> Option<Self> {
        let parts: Vec<&str> = channel.splitn(2, ':').collect();
        if parts.len() != 2 {
            return None;
        }

        let channel_type = parts[0];
        let address = parts[1];

        // Decode base58 address
        let bytes = bs58::decode(address).into_vec().ok()?;
        if bytes.len() != 32 {
            return None;
        }

        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);

        match channel_type {
            "book" => Some(Self::Book { market: arr }),
            "trades" => Some(Self::Trades { market: arr }),
            "orders" => Some(Self::Orders { owner: arr }),
            _ => None,
        }
    }

    /// Returns the channel type as a string.
    #[must_use]
    pub const fn channel_type(&self) -> &'static str {
        match self {
            Self::Book { .. } => "book",
            Self::Trades { .. } => "trades",
            Self::Orders { .. } => "orders",
        }
    }

    /// Returns true if this channel requires authentication.
    #[must_use]
    pub const fn requires_auth(&self) -> bool {
        matches!(self, Self::Orders { .. })
    }
}

impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Book { market } => write!(f, "book:{}", bs58::encode(market).into_string()),
            Self::Trades { market } => write!(f, "trades:{}", bs58::encode(market).into_string()),
            Self::Orders { owner } => write!(f, "orders:{}", bs58::encode(owner).into_string()),
        }
    }
}

/// Manages channel subscriptions for all connections.
#[derive(Debug, Default)]
pub struct ChannelManager {
    /// Map from channel to set of connection IDs.
    subscriptions: Arc<RwLock<HashMap<Channel, HashSet<u64>>>>,
}

impl ChannelManager {
    /// Creates a new channel manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Subscribes a connection to a channel.
    pub async fn subscribe(&self, channel: Channel, connection_id: u64) {
        let mut subs = self.subscriptions.write().await;
        subs.entry(channel).or_default().insert(connection_id);
    }

    /// Unsubscribes a connection from a channel.
    pub async fn unsubscribe(&self, channel: &Channel, connection_id: u64) {
        let mut subs = self.subscriptions.write().await;
        if let Some(connections) = subs.get_mut(channel) {
            connections.remove(&connection_id);
            if connections.is_empty() {
                subs.remove(channel);
            }
        }
    }

    /// Unsubscribes a connection from all channels.
    pub async fn unsubscribe_all(&self, connection_id: u64) {
        let mut subs = self.subscriptions.write().await;
        let mut empty_channels = Vec::new();

        for (channel, connections) in subs.iter_mut() {
            connections.remove(&connection_id);
            if connections.is_empty() {
                empty_channels.push(channel.clone());
            }
        }

        for channel in empty_channels {
            subs.remove(&channel);
        }
    }

    /// Returns the connection IDs subscribed to a channel.
    pub async fn subscribers(&self, channel: &Channel) -> Vec<u64> {
        let subs = self.subscriptions.read().await;
        subs.get(channel)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Returns the number of subscribers for a channel.
    pub async fn subscriber_count(&self, channel: &Channel) -> usize {
        let subs = self.subscriptions.read().await;
        subs.get(channel).map(|s| s.len()).unwrap_or(0)
    }

    /// Returns the total number of subscriptions.
    pub async fn total_subscriptions(&self) -> usize {
        let subs = self.subscriptions.read().await;
        subs.values().map(|s| s.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_parse_book() {
        let channel = Channel::parse("book:11111111111111111111111111111111");
        assert!(channel.is_some());
        let channel = channel.expect("channel");
        assert_eq!(channel.channel_type(), "book");
        assert!(!channel.requires_auth());
    }

    #[test]
    fn test_channel_parse_trades() {
        let channel = Channel::parse("trades:11111111111111111111111111111111");
        assert!(channel.is_some());
        let channel = channel.expect("channel");
        assert_eq!(channel.channel_type(), "trades");
        assert!(!channel.requires_auth());
    }

    #[test]
    fn test_channel_parse_orders() {
        let channel = Channel::parse("orders:11111111111111111111111111111111");
        assert!(channel.is_some());
        let channel = channel.expect("channel");
        assert_eq!(channel.channel_type(), "orders");
        assert!(channel.requires_auth());
    }

    #[test]
    fn test_channel_parse_invalid_type() {
        let channel = Channel::parse("invalid:11111111111111111111111111111111");
        assert!(channel.is_none());
    }

    #[test]
    fn test_channel_parse_invalid_format() {
        let channel = Channel::parse("book");
        assert!(channel.is_none());
    }

    #[test]
    fn test_channel_parse_invalid_address() {
        let channel = Channel::parse("book:invalid!");
        assert!(channel.is_none());
    }

    #[test]
    fn test_channel_to_string() {
        let market = [1u8; 32];
        let channel = Channel::Book { market };
        let s = channel.to_string();
        assert!(s.starts_with("book:"));

        // Parse back
        let parsed = Channel::parse(&s);
        assert!(parsed.is_some());
        assert_eq!(parsed.expect("parsed"), channel);
    }

    #[tokio::test]
    async fn test_channel_manager_subscribe() {
        let manager = ChannelManager::new();
        let channel = Channel::Book { market: [1u8; 32] };

        manager.subscribe(channel.clone(), 1).await;
        manager.subscribe(channel.clone(), 2).await;

        let subs = manager.subscribers(&channel).await;
        assert_eq!(subs.len(), 2);
        assert!(subs.contains(&1));
        assert!(subs.contains(&2));
    }

    #[tokio::test]
    async fn test_channel_manager_unsubscribe() {
        let manager = ChannelManager::new();
        let channel = Channel::Book { market: [1u8; 32] };

        manager.subscribe(channel.clone(), 1).await;
        manager.subscribe(channel.clone(), 2).await;
        manager.unsubscribe(&channel, 1).await;

        let subs = manager.subscribers(&channel).await;
        assert_eq!(subs.len(), 1);
        assert!(subs.contains(&2));
    }

    #[tokio::test]
    async fn test_channel_manager_unsubscribe_all() {
        let manager = ChannelManager::new();
        let channel1 = Channel::Book { market: [1u8; 32] };
        let channel2 = Channel::Trades { market: [2u8; 32] };

        manager.subscribe(channel1.clone(), 1).await;
        manager.subscribe(channel2.clone(), 1).await;
        manager.unsubscribe_all(1).await;

        assert_eq!(manager.subscriber_count(&channel1).await, 0);
        assert_eq!(manager.subscriber_count(&channel2).await, 0);
    }

    #[tokio::test]
    async fn test_channel_manager_total_subscriptions() {
        let manager = ChannelManager::new();
        let channel1 = Channel::Book { market: [1u8; 32] };
        let channel2 = Channel::Trades { market: [2u8; 32] };

        manager.subscribe(channel1.clone(), 1).await;
        manager.subscribe(channel1.clone(), 2).await;
        manager.subscribe(channel2.clone(), 1).await;

        assert_eq!(manager.total_subscriptions().await, 3);
    }
}
