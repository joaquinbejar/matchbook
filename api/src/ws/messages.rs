//! WebSocket message types.
//!
//! Defines the message format for client-server communication.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Message sent from client to server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ClientMessage {
    /// Subscribe to a channel.
    Subscribe {
        /// Channel name (e.g., "book:market_address").
        channel: String,
        /// Optional authentication token for private channels.
        #[serde(skip_serializing_if = "Option::is_none")]
        token: Option<String>,
    },

    /// Unsubscribe from a channel.
    Unsubscribe {
        /// Channel name.
        channel: String,
    },

    /// Ping message for keepalive.
    Ping,
}

/// Message sent from server to client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ServerMessage {
    /// Subscription confirmed.
    Subscribed {
        /// Channel name.
        channel: String,
    },

    /// Unsubscription confirmed.
    Unsubscribed {
        /// Channel name.
        channel: String,
    },

    /// Initial snapshot on subscription.
    Snapshot {
        /// Channel name.
        channel: String,
        /// Snapshot data.
        data: Value,
    },

    /// Incremental update.
    Update {
        /// Channel name.
        channel: String,
        /// Update data.
        data: Value,
    },

    /// Error response.
    Error {
        /// Error code.
        code: String,
        /// Error message.
        message: String,
    },

    /// Pong response to ping.
    Pong,
}

impl ServerMessage {
    /// Creates a subscribed message.
    #[must_use]
    pub fn subscribed(channel: impl Into<String>) -> Self {
        Self::Subscribed {
            channel: channel.into(),
        }
    }

    /// Creates an unsubscribed message.
    #[must_use]
    pub fn unsubscribed(channel: impl Into<String>) -> Self {
        Self::Unsubscribed {
            channel: channel.into(),
        }
    }

    /// Creates a snapshot message.
    #[must_use]
    pub fn snapshot(channel: impl Into<String>, data: Value) -> Self {
        Self::Snapshot {
            channel: channel.into(),
            data,
        }
    }

    /// Creates an update message.
    #[must_use]
    pub fn update(channel: impl Into<String>, data: Value) -> Self {
        Self::Update {
            channel: channel.into(),
            data,
        }
    }

    /// Creates an error message.
    #[must_use]
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error {
            code: code.into(),
            message: message.into(),
        }
    }

    /// Creates a pong message.
    #[must_use]
    pub const fn pong() -> Self {
        Self::Pong
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_subscribe() {
        let msg = ClientMessage::Subscribe {
            channel: "book:market123".to_string(),
            token: None,
        };
        let json = serde_json::to_string(&msg).expect("serialize");
        assert!(json.contains("subscribe"));
        assert!(json.contains("book:market123"));
    }

    #[test]
    fn test_client_message_unsubscribe() {
        let msg = ClientMessage::Unsubscribe {
            channel: "trades:market123".to_string(),
        };
        let json = serde_json::to_string(&msg).expect("serialize");
        assert!(json.contains("unsubscribe"));
    }

    #[test]
    fn test_client_message_ping() {
        let msg = ClientMessage::Ping;
        let json = serde_json::to_string(&msg).expect("serialize");
        assert!(json.contains("ping"));
    }

    #[test]
    fn test_server_message_subscribed() {
        let msg = ServerMessage::subscribed("book:market123");
        let json = serde_json::to_string(&msg).expect("serialize");
        assert!(json.contains("subscribed"));
        assert!(json.contains("book:market123"));
    }

    #[test]
    fn test_server_message_snapshot() {
        let msg = ServerMessage::snapshot("book:market123", serde_json::json!({"bids": []}));
        let json = serde_json::to_string(&msg).expect("serialize");
        assert!(json.contains("snapshot"));
        assert!(json.contains("bids"));
    }

    #[test]
    fn test_server_message_update() {
        let msg = ServerMessage::update("trades:market123", serde_json::json!({"price": 100}));
        let json = serde_json::to_string(&msg).expect("serialize");
        assert!(json.contains("update"));
        assert!(json.contains("price"));
    }

    #[test]
    fn test_server_message_error() {
        let msg = ServerMessage::error("INVALID_CHANNEL", "Channel not found");
        let json = serde_json::to_string(&msg).expect("serialize");
        assert!(json.contains("error"));
        assert!(json.contains("INVALID_CHANNEL"));
    }

    #[test]
    fn test_server_message_pong() {
        let msg = ServerMessage::pong();
        let json = serde_json::to_string(&msg).expect("serialize");
        assert!(json.contains("pong"));
    }

    #[test]
    fn test_deserialize_subscribe() {
        let json = r#"{"type":"subscribe","channel":"book:market123"}"#;
        let msg: ClientMessage = serde_json::from_str(json).expect("deserialize");
        match msg {
            ClientMessage::Subscribe { channel, token } => {
                assert_eq!(channel, "book:market123");
                assert!(token.is_none());
            }
            _ => panic!("Expected Subscribe"),
        }
    }

    #[test]
    fn test_deserialize_subscribe_with_token() {
        let json = r#"{"type":"subscribe","channel":"orders:owner123","token":"secret"}"#;
        let msg: ClientMessage = serde_json::from_str(json).expect("deserialize");
        match msg {
            ClientMessage::Subscribe { channel, token } => {
                assert_eq!(channel, "orders:owner123");
                assert_eq!(token, Some("secret".to_string()));
            }
            _ => panic!("Expected Subscribe"),
        }
    }
}
