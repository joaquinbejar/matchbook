//! Client configuration.
//!
//! Provides configuration options for the HTTP client.

use std::time::Duration;

/// Default base URL for the API.
pub const DEFAULT_BASE_URL: &str = "https://api.matchbook.example/v1";

/// Default request timeout in seconds.
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Default maximum retries.
pub const DEFAULT_MAX_RETRIES: u32 = 3;

/// Client configuration.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Base URL for the API.
    pub base_url: String,

    /// Request timeout.
    pub timeout: Duration,

    /// Maximum number of retries for failed requests.
    pub max_retries: u32,

    /// Optional API key for authentication.
    pub api_key: Option<String>,

    /// User agent string.
    pub user_agent: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            max_retries: DEFAULT_MAX_RETRIES,
            api_key: None,
            user_agent: format!("matchbook-sdk/{}", env!("CARGO_PKG_VERSION")),
        }
    }
}

impl ClientConfig {
    /// Creates a new configuration with the given base URL.
    #[must_use]
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            ..Default::default()
        }
    }

    /// Sets the request timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets the maximum number of retries.
    #[must_use]
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Sets the API key.
    #[must_use]
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Sets the user agent.
    #[must_use]
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// Validates the configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn validate(&self) -> Result<(), super::error::ClientError> {
        if self.base_url.is_empty() {
            return Err(super::error::ClientError::InvalidConfig(
                "base_url cannot be empty".to_string(),
            ));
        }

        if !self.base_url.starts_with("http://") && !self.base_url.starts_with("https://") {
            return Err(super::error::ClientError::InvalidConfig(
                "base_url must start with http:// or https://".to_string(),
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
        let config = ClientConfig::default();
        assert_eq!(config.base_url, DEFAULT_BASE_URL);
        assert_eq!(config.timeout, Duration::from_secs(DEFAULT_TIMEOUT_SECS));
        assert_eq!(config.max_retries, DEFAULT_MAX_RETRIES);
        assert!(config.api_key.is_none());
    }

    #[test]
    fn test_config_new() {
        let config = ClientConfig::new("https://api.example.com");
        assert_eq!(config.base_url, "https://api.example.com");
    }

    #[test]
    fn test_config_builder() {
        let config = ClientConfig::new("https://api.example.com")
            .with_timeout(Duration::from_secs(60))
            .with_max_retries(5)
            .with_api_key("my-api-key")
            .with_user_agent("my-app/1.0");

        assert_eq!(config.base_url, "https://api.example.com");
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.api_key, Some("my-api-key".to_string()));
        assert_eq!(config.user_agent, "my-app/1.0");
    }

    #[test]
    fn test_config_validate_valid() {
        let config = ClientConfig::new("https://api.example.com");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_empty_url() {
        let config = ClientConfig::new("");
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_invalid_scheme() {
        let config = ClientConfig::new("ftp://api.example.com");
        assert!(config.validate().is_err());
    }
}
