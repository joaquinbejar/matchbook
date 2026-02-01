//! Configuration types for Geyser listener.
//!
//! Provides configuration for connecting to Geyser gRPC endpoints
//! and managing subscription parameters.

use serde::{Deserialize, Serialize};

/// Configuration for the Geyser listener.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeyserConfig {
    /// Geyser gRPC endpoint URL.
    ///
    /// Example: "http://localhost:10000" or "https://grpc.example.com"
    pub endpoint: String,

    /// Authentication token for the Geyser endpoint.
    ///
    /// Some Geyser providers require an x-token header for authentication.
    pub x_token: Option<String>,

    /// Delay between reconnection attempts in milliseconds.
    pub reconnect_delay_ms: u64,

    /// Maximum number of reconnection attempts before giving up.
    ///
    /// Set to 0 for unlimited retries.
    pub max_reconnect_attempts: u32,

    /// Program ID to filter account updates.
    ///
    /// Only accounts owned by this program will be subscribed to.
    pub program_id: String,

    /// Whether to request account data on startup.
    ///
    /// If true, the listener will receive all current account states
    /// before streaming updates.
    pub request_startup_snapshot: bool,

    /// Channel buffer size for account updates.
    ///
    /// Larger buffers provide more backpressure tolerance but use more memory.
    pub channel_buffer_size: usize,

    /// Timeout for gRPC operations in seconds.
    pub timeout_seconds: u64,
}

impl Default for GeyserConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:10000".to_string(),
            x_token: None,
            reconnect_delay_ms: 1000,
            max_reconnect_attempts: 10,
            program_id: String::new(),
            request_startup_snapshot: true,
            channel_buffer_size: 10000,
            timeout_seconds: 30,
        }
    }
}

impl GeyserConfig {
    /// Creates a new configuration with the given endpoint and program ID.
    #[must_use]
    pub fn new(endpoint: impl Into<String>, program_id: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            program_id: program_id.into(),
            ..Default::default()
        }
    }

    /// Sets the authentication token.
    #[must_use]
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.x_token = Some(token.into());
        self
    }

    /// Sets the reconnection delay.
    #[must_use]
    pub const fn with_reconnect_delay(mut self, delay_ms: u64) -> Self {
        self.reconnect_delay_ms = delay_ms;
        self
    }

    /// Sets the maximum reconnection attempts.
    #[must_use]
    pub const fn with_max_reconnect_attempts(mut self, attempts: u32) -> Self {
        self.max_reconnect_attempts = attempts;
        self
    }

    /// Disables startup snapshot.
    #[must_use]
    pub const fn without_startup_snapshot(mut self) -> Self {
        self.request_startup_snapshot = false;
        self
    }

    /// Validates the configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.endpoint.is_empty() {
            return Err(ConfigError::EmptyEndpoint);
        }

        if self.program_id.is_empty() {
            return Err(ConfigError::EmptyProgramId);
        }

        // Validate program ID is valid base58
        if bs58::decode(&self.program_id).into_vec().is_err() {
            return Err(ConfigError::InvalidProgramId(self.program_id.clone()));
        }

        if self.channel_buffer_size == 0 {
            return Err(ConfigError::InvalidBufferSize);
        }

        Ok(())
    }
}

/// Configuration validation errors.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ConfigError {
    /// Endpoint URL is empty.
    #[error("Geyser endpoint URL cannot be empty")]
    EmptyEndpoint,

    /// Program ID is empty.
    #[error("Program ID cannot be empty")]
    EmptyProgramId,

    /// Program ID is not valid base58.
    #[error("Invalid program ID: {0}")]
    InvalidProgramId(String),

    /// Buffer size is zero.
    #[error("Channel buffer size must be greater than 0")]
    InvalidBufferSize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = GeyserConfig::default();
        assert_eq!(config.endpoint, "http://localhost:10000");
        assert!(config.x_token.is_none());
        assert_eq!(config.reconnect_delay_ms, 1000);
        assert_eq!(config.max_reconnect_attempts, 10);
        assert!(config.request_startup_snapshot);
    }

    #[test]
    fn test_config_new() {
        let config = GeyserConfig::new(
            "https://grpc.example.com",
            "MATCHBooK1111111111111111111111111111111111",
        );
        assert_eq!(config.endpoint, "https://grpc.example.com");
        assert_eq!(
            config.program_id,
            "MATCHBooK1111111111111111111111111111111111"
        );
    }

    #[test]
    fn test_config_builder() {
        let config =
            GeyserConfig::new("http://localhost:10000", "11111111111111111111111111111111")
                .with_token("secret-token")
                .with_reconnect_delay(2000)
                .with_max_reconnect_attempts(5)
                .without_startup_snapshot();

        assert_eq!(config.x_token, Some("secret-token".to_string()));
        assert_eq!(config.reconnect_delay_ms, 2000);
        assert_eq!(config.max_reconnect_attempts, 5);
        assert!(!config.request_startup_snapshot);
    }

    #[test]
    fn test_config_validate_valid() {
        let config =
            GeyserConfig::new("http://localhost:10000", "11111111111111111111111111111111");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_empty_endpoint() {
        let config = GeyserConfig::new("", "11111111111111111111111111111111");
        let err = config.validate().unwrap_err();
        assert!(matches!(err, ConfigError::EmptyEndpoint));
    }

    #[test]
    fn test_config_validate_empty_program_id() {
        let config = GeyserConfig::new("http://localhost:10000", "");
        let err = config.validate().unwrap_err();
        assert!(matches!(err, ConfigError::EmptyProgramId));
    }

    #[test]
    fn test_config_validate_invalid_program_id() {
        let config = GeyserConfig::new("http://localhost:10000", "not-valid-base58!");
        let err = config.validate().unwrap_err();
        assert!(matches!(err, ConfigError::InvalidProgramId(_)));
    }

    #[test]
    fn test_config_validate_zero_buffer() {
        let mut config =
            GeyserConfig::new("http://localhost:10000", "11111111111111111111111111111111");
        config.channel_buffer_size = 0;
        let err = config.validate().unwrap_err();
        assert!(matches!(err, ConfigError::InvalidBufferSize));
    }
}
