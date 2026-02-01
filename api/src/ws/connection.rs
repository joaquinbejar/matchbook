//! WebSocket connection state management.
//!
//! Provides connection tracking and state management.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::mpsc;

use super::channels::Channel;
use super::messages::ServerMessage;

/// Global connection ID counter.
static CONNECTION_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generates a unique connection ID.
#[must_use]
pub fn next_connection_id() -> u64 {
    CONNECTION_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// WebSocket connection state.
#[derive(Debug)]
pub struct Connection {
    /// Unique connection ID.
    pub id: u64,

    /// Channels this connection is subscribed to.
    pub subscriptions: HashSet<Channel>,

    /// Sender for outgoing messages.
    pub sender: mpsc::Sender<ServerMessage>,

    /// Whether the connection is authenticated.
    pub authenticated: bool,

    /// Authenticated owner address (if any).
    pub owner: Option<[u8; 32]>,
}

impl Connection {
    /// Creates a new connection.
    #[must_use]
    pub fn new(sender: mpsc::Sender<ServerMessage>) -> Self {
        Self {
            id: next_connection_id(),
            subscriptions: HashSet::new(),
            sender,
            authenticated: false,
            owner: None,
        }
    }

    /// Returns the connection ID.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Returns true if the connection is authenticated.
    #[must_use]
    pub const fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    /// Authenticates the connection with an owner address.
    pub fn authenticate(&mut self, owner: [u8; 32]) {
        self.authenticated = true;
        self.owner = Some(owner);
    }

    /// Adds a subscription.
    pub fn subscribe(&mut self, channel: Channel) {
        self.subscriptions.insert(channel);
    }

    /// Removes a subscription.
    pub fn unsubscribe(&mut self, channel: &Channel) {
        self.subscriptions.remove(channel);
    }

    /// Returns true if subscribed to the channel.
    #[must_use]
    pub fn is_subscribed(&self, channel: &Channel) -> bool {
        self.subscriptions.contains(channel)
    }

    /// Returns the number of subscriptions.
    #[must_use]
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.len()
    }

    /// Sends a message to this connection.
    ///
    /// Returns true if the message was sent successfully.
    pub async fn send(&self, message: ServerMessage) -> bool {
        self.sender.send(message).await.is_ok()
    }

    /// Checks if the connection can subscribe to a channel.
    ///
    /// Private channels require authentication.
    #[must_use]
    pub fn can_subscribe(&self, channel: &Channel) -> bool {
        if channel.requires_auth() {
            if !self.authenticated {
                return false;
            }
            // For orders channel, must match authenticated owner
            if let Channel::Orders { owner } = channel {
                return self.owner.as_ref() == Some(owner);
            }
        }
        true
    }
}

/// Connection manager for tracking all active connections.
#[derive(Debug, Default)]
pub struct ConnectionManager {
    /// Active connection count.
    connection_count: Arc<AtomicU64>,
}

impl ConnectionManager {
    /// Creates a new connection manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            connection_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Increments the connection count.
    pub fn on_connect(&self) {
        self.connection_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrements the connection count.
    pub fn on_disconnect(&self) {
        self.connection_count.fetch_sub(1, Ordering::Relaxed);
    }

    /// Returns the current connection count.
    #[must_use]
    pub fn connection_count(&self) -> u64 {
        self.connection_count.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_connection_id() {
        let id1 = next_connection_id();
        let id2 = next_connection_id();
        assert!(id2 > id1);
    }

    #[tokio::test]
    async fn test_connection_new() {
        let (tx, _rx) = mpsc::channel(10);
        let conn = Connection::new(tx);
        assert!(conn.id > 0);
        assert!(!conn.is_authenticated());
        assert_eq!(conn.subscription_count(), 0);
    }

    #[tokio::test]
    async fn test_connection_authenticate() {
        let (tx, _rx) = mpsc::channel(10);
        let mut conn = Connection::new(tx);
        let owner = [1u8; 32];

        conn.authenticate(owner);

        assert!(conn.is_authenticated());
        assert_eq!(conn.owner, Some(owner));
    }

    #[tokio::test]
    async fn test_connection_subscribe() {
        let (tx, _rx) = mpsc::channel(10);
        let mut conn = Connection::new(tx);
        let channel = Channel::Book { market: [1u8; 32] };

        conn.subscribe(channel.clone());

        assert!(conn.is_subscribed(&channel));
        assert_eq!(conn.subscription_count(), 1);
    }

    #[tokio::test]
    async fn test_connection_unsubscribe() {
        let (tx, _rx) = mpsc::channel(10);
        let mut conn = Connection::new(tx);
        let channel = Channel::Book { market: [1u8; 32] };

        conn.subscribe(channel.clone());
        conn.unsubscribe(&channel);

        assert!(!conn.is_subscribed(&channel));
        assert_eq!(conn.subscription_count(), 0);
    }

    #[tokio::test]
    async fn test_connection_can_subscribe_public() {
        let (tx, _rx) = mpsc::channel(10);
        let conn = Connection::new(tx);
        let channel = Channel::Book { market: [1u8; 32] };

        assert!(conn.can_subscribe(&channel));
    }

    #[tokio::test]
    async fn test_connection_can_subscribe_private_unauthenticated() {
        let (tx, _rx) = mpsc::channel(10);
        let conn = Connection::new(tx);
        let channel = Channel::Orders { owner: [1u8; 32] };

        assert!(!conn.can_subscribe(&channel));
    }

    #[tokio::test]
    async fn test_connection_can_subscribe_private_authenticated() {
        let (tx, _rx) = mpsc::channel(10);
        let mut conn = Connection::new(tx);
        let owner = [1u8; 32];
        let channel = Channel::Orders { owner };

        conn.authenticate(owner);

        assert!(conn.can_subscribe(&channel));
    }

    #[tokio::test]
    async fn test_connection_can_subscribe_private_wrong_owner() {
        let (tx, _rx) = mpsc::channel(10);
        let mut conn = Connection::new(tx);
        let owner = [1u8; 32];
        let other_owner = [2u8; 32];
        let channel = Channel::Orders { owner: other_owner };

        conn.authenticate(owner);

        assert!(!conn.can_subscribe(&channel));
    }

    #[tokio::test]
    async fn test_connection_send() {
        let (tx, mut rx) = mpsc::channel(10);
        let conn = Connection::new(tx);

        let sent = conn.send(ServerMessage::pong()).await;
        assert!(sent);

        let msg = rx.recv().await;
        assert!(msg.is_some());
    }

    #[test]
    fn test_connection_manager_new() {
        let manager = ConnectionManager::new();
        assert_eq!(manager.connection_count(), 0);
    }

    #[test]
    fn test_connection_manager_connect_disconnect() {
        let manager = ConnectionManager::new();

        manager.on_connect();
        assert_eq!(manager.connection_count(), 1);

        manager.on_connect();
        assert_eq!(manager.connection_count(), 2);

        manager.on_disconnect();
        assert_eq!(manager.connection_count(), 1);
    }
}
