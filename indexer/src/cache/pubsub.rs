//! Redis pub/sub operations.
//!
//! Provides pub/sub messaging for real-time updates.

use std::sync::Arc;

use serde::Serialize;
use tracing::{debug, warn};

use super::config::RedisConfig;
use super::metrics::CacheMetrics;

/// Pub/sub channel prefixes.
pub mod channels {
    /// Book update channel prefix.
    pub const BOOK: &str = "book";
    /// Trade notification channel prefix.
    pub const TRADES: &str = "trades";
    /// User order update channel prefix.
    pub const ORDERS: &str = "orders";
}

/// Redis pub/sub for real-time messaging.
pub struct RedisPubSub {
    /// Configuration.
    config: RedisConfig,

    /// Metrics.
    metrics: Arc<CacheMetrics>,

    /// Whether pub/sub is available.
    available: std::sync::atomic::AtomicBool,
}

impl RedisPubSub {
    /// Creates a new Redis pub/sub instance.
    #[must_use]
    pub fn new(config: RedisConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(CacheMetrics::new()),
            available: std::sync::atomic::AtomicBool::new(true),
        }
    }

    /// Creates a pub/sub instance with shared metrics.
    #[must_use]
    pub fn with_metrics(config: RedisConfig, metrics: Arc<CacheMetrics>) -> Self {
        Self {
            config,
            metrics,
            available: std::sync::atomic::AtomicBool::new(true),
        }
    }

    /// Returns the configuration.
    #[must_use]
    pub const fn config(&self) -> &RedisConfig {
        &self.config
    }

    /// Returns the metrics.
    #[must_use]
    pub fn metrics(&self) -> Arc<CacheMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Returns true if pub/sub is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Returns true if pub/sub is available.
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.available.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Marks pub/sub as unavailable.
    pub fn mark_unavailable(&self) {
        self.available
            .store(false, std::sync::atomic::Ordering::Relaxed);
        warn!("Redis pub/sub marked as unavailable");
    }

    /// Marks pub/sub as available.
    pub fn mark_available(&self) {
        self.available
            .store(true, std::sync::atomic::Ordering::Relaxed);
        debug!("Redis pub/sub marked as available");
    }

    /// Builds a channel name.
    #[must_use]
    pub fn build_channel(prefix: &str, id: &[u8; 32]) -> String {
        format!("{}:{}", prefix, bs58::encode(id).into_string())
    }

    /// Publishes a message to a channel.
    ///
    /// Does nothing if pub/sub is disabled.
    pub async fn publish<T: Serialize>(&self, channel: &str, _message: &T) {
        if !self.is_enabled() || !self.is_available() {
            return;
        }

        // Placeholder: In real implementation, this would use Redis PUBLISH
        self.metrics.record_publish();
        debug!("Published to channel: {}", channel);
    }

    /// Publishes a book update.
    pub async fn publish_book_update<T: Serialize>(&self, market: &[u8; 32], update: &T) {
        let channel = Self::build_channel(channels::BOOK, market);
        self.publish(&channel, update).await;
    }

    /// Publishes a trade notification.
    pub async fn publish_trade<T: Serialize>(&self, market: &[u8; 32], trade: &T) {
        let channel = Self::build_channel(channels::TRADES, market);
        self.publish(&channel, trade).await;
    }

    /// Publishes an order update.
    pub async fn publish_order_update<T: Serialize>(&self, owner: &[u8; 32], update: &T) {
        let channel = Self::build_channel(channels::ORDERS, owner);
        self.publish(&channel, update).await;
    }
}

/// A subscription to a Redis channel.
pub struct Subscription {
    /// Channel name.
    channel: String,

    /// Whether the subscription is active.
    active: std::sync::atomic::AtomicBool,
}

impl Subscription {
    /// Creates a new subscription.
    #[must_use]
    pub fn new(channel: impl Into<String>) -> Self {
        Self {
            channel: channel.into(),
            active: std::sync::atomic::AtomicBool::new(true),
        }
    }

    /// Returns the channel name.
    #[must_use]
    pub fn channel(&self) -> &str {
        &self.channel
    }

    /// Returns true if the subscription is active.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Cancels the subscription.
    pub fn cancel(&self) {
        self.active
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pubsub_new() {
        let config = RedisConfig::default();
        let pubsub = RedisPubSub::new(config);
        assert!(pubsub.is_enabled());
        assert!(pubsub.is_available());
    }

    #[test]
    fn test_pubsub_disabled() {
        let config = RedisConfig::default().disabled();
        let pubsub = RedisPubSub::new(config);
        assert!(!pubsub.is_enabled());
    }

    #[test]
    fn test_pubsub_mark_unavailable() {
        let config = RedisConfig::default();
        let pubsub = RedisPubSub::new(config);

        pubsub.mark_unavailable();
        assert!(!pubsub.is_available());

        pubsub.mark_available();
        assert!(pubsub.is_available());
    }

    #[test]
    fn test_build_channel() {
        let id = [1u8; 32];
        let channel = RedisPubSub::build_channel("book", &id);
        assert!(channel.starts_with("book:"));
        assert!(channel.len() > 10);
    }

    #[tokio::test]
    async fn test_pubsub_publish() {
        let config = RedisConfig::default();
        let pubsub = RedisPubSub::new(config);

        pubsub.publish("test_channel", &"message").await;
        assert_eq!(pubsub.metrics().publishes(), 1);
    }

    #[tokio::test]
    async fn test_pubsub_publish_disabled() {
        let config = RedisConfig::default().disabled();
        let pubsub = RedisPubSub::new(config);

        pubsub.publish("test_channel", &"message").await;
        // Should not record metrics when disabled
        assert_eq!(pubsub.metrics().publishes(), 0);
    }

    #[tokio::test]
    async fn test_pubsub_publish_book_update() {
        let config = RedisConfig::default();
        let pubsub = RedisPubSub::new(config);
        let market = [1u8; 32];

        pubsub.publish_book_update(&market, &"update").await;
        assert_eq!(pubsub.metrics().publishes(), 1);
    }

    #[tokio::test]
    async fn test_pubsub_publish_trade() {
        let config = RedisConfig::default();
        let pubsub = RedisPubSub::new(config);
        let market = [1u8; 32];

        pubsub.publish_trade(&market, &"trade").await;
        assert_eq!(pubsub.metrics().publishes(), 1);
    }

    #[tokio::test]
    async fn test_pubsub_publish_order_update() {
        let config = RedisConfig::default();
        let pubsub = RedisPubSub::new(config);
        let owner = [1u8; 32];

        pubsub.publish_order_update(&owner, &"order").await;
        assert_eq!(pubsub.metrics().publishes(), 1);
    }

    #[test]
    fn test_subscription_new() {
        let sub = Subscription::new("test_channel");
        assert_eq!(sub.channel(), "test_channel");
        assert!(sub.is_active());
    }

    #[test]
    fn test_subscription_cancel() {
        let sub = Subscription::new("test_channel");

        sub.cancel();
        assert!(!sub.is_active());
    }
}
