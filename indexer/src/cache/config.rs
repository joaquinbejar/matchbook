//! Redis configuration.
//!
//! Provides configuration options for Redis connection and caching.

use serde::{Deserialize, Serialize};

/// Configuration for Redis connection and caching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis connection URL.
    pub url: String,

    /// Connection pool size.
    pub pool_size: u32,

    /// Connection timeout in milliseconds.
    pub connection_timeout_ms: u64,

    /// TTL for market metadata in seconds.
    pub market_ttl_secs: u64,

    /// TTL for order book snapshots in seconds.
    pub orderbook_ttl_secs: u64,

    /// TTL for recent trades in seconds.
    pub trades_ttl_secs: u64,

    /// TTL for user balances in seconds.
    pub balances_ttl_secs: u64,

    /// Whether to enable caching.
    pub enabled: bool,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://127.0.0.1:6379".to_string(),
            pool_size: 10,
            connection_timeout_ms: 5000,
            market_ttl_secs: 60,
            orderbook_ttl_secs: 1,
            trades_ttl_secs: 10,
            balances_ttl_secs: 5,
            enabled: true,
        }
    }
}

impl RedisConfig {
    /// Creates a new configuration with the given URL.
    #[must_use]
    pub fn with_url(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }

    /// Sets the pool size.
    #[must_use]
    pub fn with_pool_size(mut self, size: u32) -> Self {
        self.pool_size = size;
        self
    }

    /// Sets the market TTL.
    #[must_use]
    pub fn with_market_ttl(mut self, secs: u64) -> Self {
        self.market_ttl_secs = secs;
        self
    }

    /// Sets the orderbook TTL.
    #[must_use]
    pub fn with_orderbook_ttl(mut self, secs: u64) -> Self {
        self.orderbook_ttl_secs = secs;
        self
    }

    /// Sets the trades TTL.
    #[must_use]
    pub fn with_trades_ttl(mut self, secs: u64) -> Self {
        self.trades_ttl_secs = secs;
        self
    }

    /// Sets the balances TTL.
    #[must_use]
    pub fn with_balances_ttl(mut self, secs: u64) -> Self {
        self.balances_ttl_secs = secs;
        self
    }

    /// Disables caching.
    #[must_use]
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Validates the configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.url.is_empty() {
            return Err(ConfigError::EmptyUrl);
        }

        if self.pool_size == 0 {
            return Err(ConfigError::InvalidPoolSize);
        }

        if self.connection_timeout_ms == 0 {
            return Err(ConfigError::InvalidTimeout);
        }

        Ok(())
    }
}

/// Configuration errors.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ConfigError {
    /// Empty URL.
    #[error("redis URL cannot be empty")]
    EmptyUrl,

    /// Invalid pool size.
    #[error("pool size must be > 0")]
    InvalidPoolSize,

    /// Invalid timeout.
    #[error("connection timeout must be > 0")]
    InvalidTimeout,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = RedisConfig::default();
        assert_eq!(config.url, "redis://127.0.0.1:6379");
        assert_eq!(config.pool_size, 10);
        assert_eq!(config.market_ttl_secs, 60);
        assert_eq!(config.orderbook_ttl_secs, 1);
        assert!(config.enabled);
    }

    #[test]
    fn test_config_with_url() {
        let config = RedisConfig::with_url("redis://localhost:6380");
        assert_eq!(config.url, "redis://localhost:6380");
    }

    #[test]
    fn test_config_builder() {
        let config = RedisConfig::default()
            .with_pool_size(20)
            .with_market_ttl(120)
            .with_orderbook_ttl(2)
            .with_trades_ttl(30)
            .with_balances_ttl(10);

        assert_eq!(config.pool_size, 20);
        assert_eq!(config.market_ttl_secs, 120);
        assert_eq!(config.orderbook_ttl_secs, 2);
        assert_eq!(config.trades_ttl_secs, 30);
        assert_eq!(config.balances_ttl_secs, 10);
    }

    #[test]
    fn test_config_disabled() {
        let config = RedisConfig::default().disabled();
        assert!(!config.enabled);
    }

    #[test]
    fn test_config_validate_valid() {
        let config = RedisConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_empty_url() {
        let config = RedisConfig {
            url: String::new(),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_zero_pool() {
        let config = RedisConfig {
            pool_size: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_zero_timeout() {
        let config = RedisConfig {
            connection_timeout_ms: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }
}
