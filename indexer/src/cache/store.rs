//! Redis cache operations.
//!
//! Provides caching for frequently accessed data.

use std::sync::Arc;

use serde::{de::DeserializeOwned, Serialize};
use tracing::{debug, warn};

use super::config::RedisConfig;
use super::metrics::CacheMetrics;

/// Cache key prefixes.
pub mod keys {
    /// Market metadata key prefix.
    pub const MARKET: &str = "market";
    /// Order book snapshot key prefix.
    pub const ORDERBOOK: &str = "orderbook";
    /// Recent trades key prefix.
    pub const TRADES: &str = "trades";
    /// User balances key prefix.
    pub const BALANCES: &str = "balances";
}

/// Redis cache for frequently accessed data.
pub struct RedisCache {
    /// Configuration.
    config: RedisConfig,

    /// Metrics.
    metrics: Arc<CacheMetrics>,

    /// Whether the cache is available.
    available: std::sync::atomic::AtomicBool,
}

impl RedisCache {
    /// Creates a new Redis cache.
    #[must_use]
    pub fn new(config: RedisConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(CacheMetrics::new()),
            available: std::sync::atomic::AtomicBool::new(true),
        }
    }

    /// Creates a cache with shared metrics.
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

    /// Returns true if caching is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Returns true if the cache is available.
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.available.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Marks the cache as unavailable.
    pub fn mark_unavailable(&self) {
        self.available
            .store(false, std::sync::atomic::Ordering::Relaxed);
        warn!("Redis cache marked as unavailable");
    }

    /// Marks the cache as available.
    pub fn mark_available(&self) {
        self.available
            .store(true, std::sync::atomic::Ordering::Relaxed);
        debug!("Redis cache marked as available");
    }

    /// Builds a cache key.
    #[must_use]
    pub fn build_key(prefix: &str, id: &[u8; 32]) -> String {
        format!("{}:{}", prefix, bs58::encode(id).into_string())
    }

    /// Gets a value from the cache.
    ///
    /// Returns `None` if the key doesn't exist or caching is disabled.
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        if !self.is_enabled() || !self.is_available() {
            return None;
        }

        // Placeholder: In real implementation, this would use Redis
        // For now, always return None (cache miss)
        self.metrics.record_miss();
        debug!("Cache miss for key: {}", key);
        None
    }

    /// Sets a value in the cache with TTL.
    ///
    /// Does nothing if caching is disabled.
    pub async fn set<T: Serialize>(&self, key: &str, _value: &T, ttl_secs: u64) {
        if !self.is_enabled() || !self.is_available() {
            return;
        }

        // Placeholder: In real implementation, this would use Redis
        self.metrics.record_set();
        debug!("Cache set for key: {} (TTL: {}s)", key, ttl_secs);
    }

    /// Deletes a value from the cache.
    pub async fn delete(&self, key: &str) {
        if !self.is_enabled() || !self.is_available() {
            return;
        }

        // Placeholder: In real implementation, this would use Redis
        self.metrics.record_delete();
        debug!("Cache delete for key: {}", key);
    }

    /// Gets market metadata.
    pub async fn get_market<T: DeserializeOwned>(&self, market: &[u8; 32]) -> Option<T> {
        let key = Self::build_key(keys::MARKET, market);
        self.get(&key).await
    }

    /// Sets market metadata.
    pub async fn set_market<T: Serialize>(&self, market: &[u8; 32], value: &T) {
        let key = Self::build_key(keys::MARKET, market);
        self.set(&key, value, self.config.market_ttl_secs).await;
    }

    /// Gets order book snapshot.
    pub async fn get_orderbook<T: DeserializeOwned>(&self, market: &[u8; 32]) -> Option<T> {
        let key = Self::build_key(keys::ORDERBOOK, market);
        self.get(&key).await
    }

    /// Sets order book snapshot.
    pub async fn set_orderbook<T: Serialize>(&self, market: &[u8; 32], value: &T) {
        let key = Self::build_key(keys::ORDERBOOK, market);
        self.set(&key, value, self.config.orderbook_ttl_secs).await;
    }

    /// Gets recent trades.
    pub async fn get_trades<T: DeserializeOwned>(&self, market: &[u8; 32]) -> Option<T> {
        let key = Self::build_key(keys::TRADES, market);
        self.get(&key).await
    }

    /// Sets recent trades.
    pub async fn set_trades<T: Serialize>(&self, market: &[u8; 32], value: &T) {
        let key = Self::build_key(keys::TRADES, market);
        self.set(&key, value, self.config.trades_ttl_secs).await;
    }

    /// Gets user balances.
    pub async fn get_balances<T: DeserializeOwned>(&self, owner: &[u8; 32]) -> Option<T> {
        let key = Self::build_key(keys::BALANCES, owner);
        self.get(&key).await
    }

    /// Sets user balances.
    pub async fn set_balances<T: Serialize>(&self, owner: &[u8; 32], value: &T) {
        let key = Self::build_key(keys::BALANCES, owner);
        self.set(&key, value, self.config.balances_ttl_secs).await;
    }

    /// Invalidates market-related cache entries.
    pub async fn invalidate_market(&self, market: &[u8; 32]) {
        self.delete(&Self::build_key(keys::MARKET, market)).await;
        self.delete(&Self::build_key(keys::ORDERBOOK, market)).await;
        self.delete(&Self::build_key(keys::TRADES, market)).await;
    }

    /// Invalidates user-related cache entries.
    pub async fn invalidate_user(&self, owner: &[u8; 32]) {
        self.delete(&Self::build_key(keys::BALANCES, owner)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_new() {
        let config = RedisConfig::default();
        let cache = RedisCache::new(config);
        assert!(cache.is_enabled());
        assert!(cache.is_available());
    }

    #[test]
    fn test_cache_disabled() {
        let config = RedisConfig::default().disabled();
        let cache = RedisCache::new(config);
        assert!(!cache.is_enabled());
    }

    #[test]
    fn test_cache_mark_unavailable() {
        let config = RedisConfig::default();
        let cache = RedisCache::new(config);

        cache.mark_unavailable();
        assert!(!cache.is_available());

        cache.mark_available();
        assert!(cache.is_available());
    }

    #[test]
    fn test_build_key() {
        let id = [1u8; 32];
        let key = RedisCache::build_key("market", &id);
        assert!(key.starts_with("market:"));
        assert!(key.len() > 10);
    }

    #[tokio::test]
    async fn test_cache_get_miss() {
        let config = RedisConfig::default();
        let cache = RedisCache::new(config);

        let result: Option<String> = cache.get("nonexistent").await;
        assert!(result.is_none());
        assert_eq!(cache.metrics().misses(), 1);
    }

    #[tokio::test]
    async fn test_cache_set() {
        let config = RedisConfig::default();
        let cache = RedisCache::new(config);

        cache.set("key", &"value", 60).await;
        assert_eq!(cache.metrics().sets(), 1);
    }

    #[tokio::test]
    async fn test_cache_delete() {
        let config = RedisConfig::default();
        let cache = RedisCache::new(config);

        cache.delete("key").await;
        assert_eq!(cache.metrics().deletes(), 1);
    }

    #[tokio::test]
    async fn test_cache_get_disabled() {
        let config = RedisConfig::default().disabled();
        let cache = RedisCache::new(config);

        let result: Option<String> = cache.get("key").await;
        assert!(result.is_none());
        // Should not record metrics when disabled
        assert_eq!(cache.metrics().misses(), 0);
    }

    #[tokio::test]
    async fn test_cache_set_disabled() {
        let config = RedisConfig::default().disabled();
        let cache = RedisCache::new(config);

        cache.set("key", &"value", 60).await;
        // Should not record metrics when disabled
        assert_eq!(cache.metrics().sets(), 0);
    }

    #[tokio::test]
    async fn test_cache_market_operations() {
        let config = RedisConfig::default();
        let cache = RedisCache::new(config);
        let market = [1u8; 32];

        cache.set_market(&market, &"market_data").await;
        let _: Option<String> = cache.get_market(&market).await;

        assert_eq!(cache.metrics().sets(), 1);
        assert_eq!(cache.metrics().misses(), 1);
    }

    #[tokio::test]
    async fn test_cache_orderbook_operations() {
        let config = RedisConfig::default();
        let cache = RedisCache::new(config);
        let market = [1u8; 32];

        cache.set_orderbook(&market, &"orderbook_data").await;
        let _: Option<String> = cache.get_orderbook(&market).await;

        assert_eq!(cache.metrics().sets(), 1);
    }

    #[tokio::test]
    async fn test_cache_trades_operations() {
        let config = RedisConfig::default();
        let cache = RedisCache::new(config);
        let market = [1u8; 32];

        cache.set_trades(&market, &"trades_data").await;
        let _: Option<String> = cache.get_trades(&market).await;

        assert_eq!(cache.metrics().sets(), 1);
    }

    #[tokio::test]
    async fn test_cache_balances_operations() {
        let config = RedisConfig::default();
        let cache = RedisCache::new(config);
        let owner = [1u8; 32];

        cache.set_balances(&owner, &"balances_data").await;
        let _: Option<String> = cache.get_balances(&owner).await;

        assert_eq!(cache.metrics().sets(), 1);
    }

    #[tokio::test]
    async fn test_cache_invalidate_market() {
        let config = RedisConfig::default();
        let cache = RedisCache::new(config);
        let market = [1u8; 32];

        cache.invalidate_market(&market).await;

        // Should delete 3 keys: market, orderbook, trades
        assert_eq!(cache.metrics().deletes(), 3);
    }

    #[tokio::test]
    async fn test_cache_invalidate_user() {
        let config = RedisConfig::default();
        let cache = RedisCache::new(config);
        let owner = [1u8; 32];

        cache.invalidate_user(&owner).await;

        assert_eq!(cache.metrics().deletes(), 1);
    }
}
