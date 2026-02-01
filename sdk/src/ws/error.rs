//! WebSocket error types.
//!
//! Provides error types for WebSocket client operations.

use std::fmt;

/// WebSocket errors.
#[derive(Debug)]
pub enum WsError {
    /// Connection failed.
    Connection(String),

    /// WebSocket protocol error.
    Protocol(String),

    /// Failed to serialize message.
    Serialization(String),

    /// Failed to deserialize message.
    Deserialization(String),

    /// Server returned an error.
    Server {
        /// Error code.
        code: String,
        /// Error message.
        message: String,
    },

    /// Not connected.
    NotConnected,

    /// Connection closed.
    Closed,

    /// Subscription failed.
    SubscriptionFailed(String),

    /// Invalid configuration.
    InvalidConfig(String),

    /// Send failed.
    SendFailed(String),
}

impl fmt::Display for WsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connection(msg) => write!(f, "connection failed: {}", msg),
            Self::Protocol(msg) => write!(f, "protocol error: {}", msg),
            Self::Serialization(msg) => write!(f, "serialization failed: {}", msg),
            Self::Deserialization(msg) => write!(f, "deserialization failed: {}", msg),
            Self::Server { code, message } => write!(f, "server error [{}]: {}", code, message),
            Self::NotConnected => write!(f, "not connected"),
            Self::Closed => write!(f, "connection closed"),
            Self::SubscriptionFailed(msg) => write!(f, "subscription failed: {}", msg),
            Self::InvalidConfig(msg) => write!(f, "invalid configuration: {}", msg),
            Self::SendFailed(msg) => write!(f, "send failed: {}", msg),
        }
    }
}

impl std::error::Error for WsError {}

impl From<tokio_tungstenite::tungstenite::Error> for WsError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::Protocol(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_error_display() {
        let err = WsError::Connection("timeout".to_string());
        assert_eq!(err.to_string(), "connection failed: timeout");
    }

    #[test]
    fn test_ws_error_server() {
        let err = WsError::Server {
            code: "INVALID_MARKET".to_string(),
            message: "market not found".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "server error [INVALID_MARKET]: market not found"
        );
    }

    #[test]
    fn test_ws_error_not_connected() {
        let err = WsError::NotConnected;
        assert_eq!(err.to_string(), "not connected");
    }

    #[test]
    fn test_ws_error_closed() {
        let err = WsError::Closed;
        assert_eq!(err.to_string(), "connection closed");
    }
}
