//! Crank service configuration.
//!
//! Provides configuration options for the crank service.

use serde::{Deserialize, Serialize};

/// Configuration for the crank service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrankConfig {
    /// Markets to crank (base58-encoded addresses).
    pub markets: Vec<String>,

    /// Minimum priority fee in lamports.
    pub min_priority_fee: u64,

    /// Maximum priority fee in lamports.
    pub max_priority_fee: u64,

    /// Poll interval in milliseconds.
    pub poll_interval_ms: u64,

    /// Maximum retries for failed transactions.
    pub max_retries: u32,

    /// Maximum matches per transaction.
    pub max_matches_per_tx: u32,

    /// Whether to enable profitability checks.
    pub enable_profitability_check: bool,

    /// Minimum profit threshold in lamports.
    pub min_profit_threshold: u64,

    /// Backoff multiplier for retries.
    pub backoff_multiplier: f64,

    /// Maximum backoff in milliseconds.
    pub max_backoff_ms: u64,
}

impl Default for CrankConfig {
    fn default() -> Self {
        Self {
            markets: Vec::new(),
            min_priority_fee: 1000,
            max_priority_fee: 100_000,
            poll_interval_ms: 100,
            max_retries: 3,
            max_matches_per_tx: 10,
            enable_profitability_check: true,
            min_profit_threshold: 5000,
            backoff_multiplier: 2.0,
            max_backoff_ms: 10_000,
        }
    }
}

impl CrankConfig {
    /// Creates a new configuration with the given markets.
    #[must_use]
    pub fn with_markets(markets: Vec<String>) -> Self {
        Self {
            markets,
            ..Default::default()
        }
    }

    /// Sets the priority fee range.
    #[must_use]
    pub fn with_priority_fees(mut self, min: u64, max: u64) -> Self {
        self.min_priority_fee = min;
        self.max_priority_fee = max;
        self
    }

    /// Sets the poll interval.
    #[must_use]
    pub fn with_poll_interval(mut self, ms: u64) -> Self {
        self.poll_interval_ms = ms;
        self
    }

    /// Sets the maximum retries.
    #[must_use]
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Validates the configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.min_priority_fee > self.max_priority_fee {
            return Err(ConfigError::InvalidPriorityFeeRange);
        }

        if self.poll_interval_ms == 0 {
            return Err(ConfigError::InvalidPollInterval);
        }

        if self.backoff_multiplier < 1.0 {
            return Err(ConfigError::InvalidBackoffMultiplier);
        }

        Ok(())
    }

    /// Parses market addresses from base58 strings.
    ///
    /// # Errors
    ///
    /// Returns an error if any address is invalid.
    pub fn parse_markets(&self) -> Result<Vec<[u8; 32]>, ConfigError> {
        self.markets
            .iter()
            .map(|s| {
                let bytes = bs58::decode(s)
                    .into_vec()
                    .map_err(|_| ConfigError::InvalidMarketAddress(s.clone()))?;

                if bytes.len() != 32 {
                    return Err(ConfigError::InvalidMarketAddress(s.clone()));
                }

                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                Ok(arr)
            })
            .collect()
    }
}

/// Configuration errors.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ConfigError {
    /// Invalid priority fee range.
    #[error("min_priority_fee must be <= max_priority_fee")]
    InvalidPriorityFeeRange,

    /// Invalid poll interval.
    #[error("poll_interval_ms must be > 0")]
    InvalidPollInterval,

    /// Invalid backoff multiplier.
    #[error("backoff_multiplier must be >= 1.0")]
    InvalidBackoffMultiplier,

    /// Invalid market address.
    #[error("invalid market address: {0}")]
    InvalidMarketAddress(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = CrankConfig::default();
        assert!(config.markets.is_empty());
        assert_eq!(config.min_priority_fee, 1000);
        assert_eq!(config.max_priority_fee, 100_000);
        assert_eq!(config.poll_interval_ms, 100);
    }

    #[test]
    fn test_config_with_markets() {
        let config = CrankConfig::with_markets(vec!["market1".to_string()]);
        assert_eq!(config.markets.len(), 1);
    }

    #[test]
    fn test_config_builder() {
        let config = CrankConfig::default()
            .with_priority_fees(500, 50_000)
            .with_poll_interval(200)
            .with_max_retries(5);

        assert_eq!(config.min_priority_fee, 500);
        assert_eq!(config.max_priority_fee, 50_000);
        assert_eq!(config.poll_interval_ms, 200);
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_config_validate_valid() {
        let config = CrankConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_invalid_fee_range() {
        let config = CrankConfig {
            min_priority_fee: 100_000,
            max_priority_fee: 1000,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_invalid_poll_interval() {
        let config = CrankConfig {
            poll_interval_ms: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_invalid_backoff() {
        let config = CrankConfig {
            backoff_multiplier: 0.5,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_parse_markets() {
        let config =
            CrankConfig::with_markets(vec!["11111111111111111111111111111111".to_string()]);
        let markets = config.parse_markets();
        assert!(markets.is_ok());
        assert_eq!(markets.expect("markets").len(), 1);
    }

    #[test]
    fn test_config_parse_markets_invalid() {
        let config = CrankConfig::with_markets(vec!["invalid!".to_string()]);
        let markets = config.parse_markets();
        assert!(markets.is_err());
    }
}
