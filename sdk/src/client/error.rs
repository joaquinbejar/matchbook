//! Client error types.
//!
//! Provides error types for HTTP client operations.

use std::fmt;

/// Client errors.
#[derive(Debug)]
pub enum ClientError {
    /// HTTP request failed.
    Request(reqwest::Error),

    /// Failed to deserialize response.
    Deserialization(String),

    /// API returned an error response.
    Api {
        /// Error code.
        code: String,
        /// Error message.
        message: String,
    },

    /// Rate limited (429).
    RateLimited {
        /// Retry after seconds.
        retry_after: Option<u64>,
    },

    /// Resource not found (404).
    NotFound(String),

    /// Unauthorized (401).
    Unauthorized,

    /// Invalid configuration.
    InvalidConfig(String),

    /// Request timeout.
    Timeout,
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Request(e) => write!(f, "HTTP request failed: {}", e),
            Self::Deserialization(msg) => write!(f, "deserialization failed: {}", msg),
            Self::Api { code, message } => write!(f, "API error [{}]: {}", code, message),
            Self::RateLimited { retry_after } => {
                if let Some(secs) = retry_after {
                    write!(f, "rate limited, retry after {} seconds", secs)
                } else {
                    write!(f, "rate limited")
                }
            }
            Self::NotFound(resource) => write!(f, "not found: {}", resource),
            Self::Unauthorized => write!(f, "unauthorized"),
            Self::InvalidConfig(msg) => write!(f, "invalid configuration: {}", msg),
            Self::Timeout => write!(f, "request timeout"),
        }
    }
}

impl std::error::Error for ClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Request(e) => Some(e),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for ClientError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Self::Timeout
        } else {
            Self::Request(err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_error_display() {
        let err = ClientError::Api {
            code: "INVALID_PARAM".to_string(),
            message: "price must be positive".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "API error [INVALID_PARAM]: price must be positive"
        );
    }

    #[test]
    fn test_client_error_rate_limited() {
        let err = ClientError::RateLimited {
            retry_after: Some(30),
        };
        assert_eq!(err.to_string(), "rate limited, retry after 30 seconds");

        let err = ClientError::RateLimited { retry_after: None };
        assert_eq!(err.to_string(), "rate limited");
    }

    #[test]
    fn test_client_error_not_found() {
        let err = ClientError::NotFound("market ABC123".to_string());
        assert_eq!(err.to_string(), "not found: market ABC123");
    }

    #[test]
    fn test_client_error_unauthorized() {
        let err = ClientError::Unauthorized;
        assert_eq!(err.to_string(), "unauthorized");
    }

    #[test]
    fn test_client_error_timeout() {
        let err = ClientError::Timeout;
        assert_eq!(err.to_string(), "request timeout");
    }
}
