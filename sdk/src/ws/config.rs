//! WebSocket configuration.
//!
//! Provides configuration options for the WebSocket client.

use std::time::Duration;

/// Default WebSocket URL.
pub const DEFAULT_WS_URL: &str = "wss://ws.matchbook.example/v1/stream";

/// Default heartbeat interval in seconds.
pub const DEFAULT_HEARTBEAT_SECS: u64 = 30;

/// Default reconnect delay in seconds.
pub const DEFAULT_RECONNECT_DELAY_SECS: u64 = 1;

/// Maximum reconnect delay in seconds.
pub const MAX_RECONNECT_DELAY_SECS: u64 = 30;

/// WebSocket configuration.
#[derive(Debug, Clone)]
pub struct WsConfig {
    /// WebSocket URL.
    pub url: String,

    /// Heartbeat interval.
    pub heartbeat_interval: Duration,

    /// Initial reconnect delay.
    pub reconnect_delay: Duration,

    /// Maximum reconnect delay.
    pub max_reconnect_delay: Duration,

    /// Optional API key for authentication.
    pub api_key: Option<String>,

    /// Maximum reconnection attempts (None = unlimited).
    pub max_reconnect_attempts: Option<u32>,
}

impl Default for WsConfig {
    fn default() -> Self {
        Self {
            url: DEFAULT_WS_URL.to_string(),
            heartbeat_interval: Duration::from_secs(DEFAULT_HEARTBEAT_SECS),
            reconnect_delay: Duration::from_secs(DEFAULT_RECONNECT_DELAY_SECS),
            max_reconnect_delay: Duration::from_secs(MAX_RECONNECT_DELAY_SECS),
            api_key: None,
            max_reconnect_attempts: None,
        }
    }
}

impl WsConfig {
    /// Creates a new configuration with the given URL.
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }

    /// Sets the heartbeat interval.
    #[must_use]
    pub fn with_heartbeat_interval(mut self, interval: Duration) -> Self {
        self.heartbeat_interval = interval;
        self
    }

    /// Sets the initial reconnect delay.
    #[must_use]
    pub fn with_reconnect_delay(mut self, delay: Duration) -> Self {
        self.reconnect_delay = delay;
        self
    }

    /// Sets the maximum reconnect delay.
    #[must_use]
    pub fn with_max_reconnect_delay(mut self, delay: Duration) -> Self {
        self.max_reconnect_delay = delay;
        self
    }

    /// Sets the API key.
    #[must_use]
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Sets the maximum reconnection attempts.
    #[must_use]
    pub fn with_max_reconnect_attempts(mut self, attempts: u32) -> Self {
        self.max_reconnect_attempts = Some(attempts);
        self
    }

    /// Returns the connection URL with API key if set.
    #[must_use]
    pub fn connection_url(&self) -> String {
        match &self.api_key {
            Some(key) => {
                if self.url.contains('?') {
                    format!("{}&api_key={}", self.url, key)
                } else {
                    format!("{}?api_key={}", self.url, key)
                }
            }
            None => self.url.clone(),
        }
    }

    /// Validates the configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn validate(&self) -> Result<(), super::error::WsError> {
        if self.url.is_empty() {
            return Err(super::error::WsError::InvalidConfig(
                "url cannot be empty".to_string(),
            ));
        }

        if !self.url.starts_with("ws://") && !self.url.starts_with("wss://") {
            return Err(super::error::WsError::InvalidConfig(
                "url must start with ws:// or wss://".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = WsConfig::default();
        assert_eq!(config.url, DEFAULT_WS_URL);
        assert_eq!(
            config.heartbeat_interval,
            Duration::from_secs(DEFAULT_HEARTBEAT_SECS)
        );
        assert!(config.api_key.is_none());
    }

    #[test]
    fn test_config_new() {
        let config = WsConfig::new("wss://example.com/ws");
        assert_eq!(config.url, "wss://example.com/ws");
    }

    #[test]
    fn test_config_builder() {
        let config = WsConfig::new("wss://example.com/ws")
            .with_heartbeat_interval(Duration::from_secs(60))
            .with_reconnect_delay(Duration::from_secs(2))
            .with_max_reconnect_delay(Duration::from_secs(60))
            .with_api_key("my-api-key")
            .with_max_reconnect_attempts(5);

        assert_eq!(config.url, "wss://example.com/ws");
        assert_eq!(config.heartbeat_interval, Duration::from_secs(60));
        assert_eq!(config.reconnect_delay, Duration::from_secs(2));
        assert_eq!(config.max_reconnect_delay, Duration::from_secs(60));
        assert_eq!(config.api_key, Some("my-api-key".to_string()));
        assert_eq!(config.max_reconnect_attempts, Some(5));
    }

    #[test]
    fn test_config_connection_url_no_key() {
        let config = WsConfig::new("wss://example.com/ws");
        assert_eq!(config.connection_url(), "wss://example.com/ws");
    }

    #[test]
    fn test_config_connection_url_with_key() {
        let config = WsConfig::new("wss://example.com/ws").with_api_key("test-key");
        assert_eq!(
            config.connection_url(),
            "wss://example.com/ws?api_key=test-key"
        );
    }

    #[test]
    fn test_config_connection_url_with_existing_params() {
        let config = WsConfig::new("wss://example.com/ws?foo=bar").with_api_key("test-key");
        assert_eq!(
            config.connection_url(),
            "wss://example.com/ws?foo=bar&api_key=test-key"
        );
    }

    #[test]
    fn test_config_validate_valid() {
        let config = WsConfig::new("wss://example.com/ws");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_empty_url() {
        let config = WsConfig::new("");
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_invalid_scheme() {
        let config = WsConfig::new("https://example.com/ws");
        assert!(config.validate().is_err());
    }
}
