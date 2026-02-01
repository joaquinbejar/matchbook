//! WebSocket message types.
//!
//! Defines the message types for WebSocket communication.

use serde::{Deserialize, Serialize};

use crate::types::{BookLevel, Price, Quantity, Side};

/// WebSocket channel types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    /// Order book updates.
    Book,
    /// Trade stream.
    Trades,
    /// User order updates (requires authentication).
    Orders,
    /// Price ticker.
    Ticker,
}

impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Book => write!(f, "book"),
            Self::Trades => write!(f, "trades"),
            Self::Orders => write!(f, "orders"),
            Self::Ticker => write!(f, "ticker"),
        }
    }
}

/// Client-to-server messages.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ClientMessage {
    /// Subscribe to a channel.
    Subscribe {
        /// Channel to subscribe to.
        channel: Channel,
        /// Market address.
        #[serde(skip_serializing_if = "Option::is_none")]
        market: Option<String>,
        /// Depth for book channel.
        #[serde(skip_serializing_if = "Option::is_none")]
        depth: Option<u32>,
    },
    /// Unsubscribe from a channel.
    Unsubscribe {
        /// Channel to unsubscribe from.
        channel: Channel,
        /// Market address.
        #[serde(skip_serializing_if = "Option::is_none")]
        market: Option<String>,
    },
    /// Ping message for heartbeat.
    Ping {
        /// Timestamp in milliseconds.
        timestamp: u64,
    },
}

/// Server-to-client messages.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Subscription confirmed.
    Subscribed {
        /// Channel subscribed to.
        channel: Channel,
        /// Market address.
        market: Option<String>,
    },
    /// Unsubscription confirmed.
    Unsubscribed {
        /// Channel unsubscribed from.
        channel: Channel,
        /// Market address.
        market: Option<String>,
    },
    /// Order book snapshot.
    BookSnapshot {
        /// Market address.
        market: String,
        /// Slot number.
        slot: u64,
        /// Sequence number.
        sequence: u64,
        /// Bid levels.
        bids: Vec<BookLevel>,
        /// Ask levels.
        asks: Vec<BookLevel>,
    },
    /// Order book update (delta).
    BookUpdate {
        /// Market address.
        market: String,
        /// Slot number.
        slot: u64,
        /// Sequence number.
        sequence: u64,
        /// Bid changes.
        bids: Vec<BookChange>,
        /// Ask changes.
        asks: Vec<BookChange>,
    },
    /// Trade event.
    Trade {
        /// Market address.
        market: String,
        /// Trade ID.
        id: String,
        /// Trade price.
        price: Price,
        /// Trade quantity.
        quantity: Quantity,
        /// Taker side.
        side: Side,
        /// Timestamp.
        timestamp: String,
    },
    /// Ticker update.
    Ticker {
        /// Market address.
        market: String,
        /// Best bid price.
        best_bid: Option<Price>,
        /// Best ask price.
        best_ask: Option<Price>,
        /// Last trade price.
        last_price: Option<Price>,
        /// 24h volume.
        volume_24h: Option<String>,
        /// 24h price change percentage.
        price_change_24h: Option<String>,
        /// Timestamp.
        timestamp: String,
    },
    /// Order update (for authenticated users).
    OrderUpdate {
        /// Market address.
        market: String,
        /// Order ID.
        order_id: String,
        /// Client order ID.
        client_order_id: Option<u64>,
        /// Order status.
        status: String,
        /// Filled quantity.
        filled_quantity: Quantity,
        /// Remaining quantity.
        remaining_quantity: Quantity,
        /// Average fill price.
        average_price: Option<Price>,
        /// Timestamp.
        timestamp: String,
    },
    /// Pong response.
    Pong {
        /// Timestamp in milliseconds.
        timestamp: u64,
    },
    /// Error message.
    Error {
        /// Error code.
        code: String,
        /// Error message.
        message: String,
        /// Request ID if applicable.
        request_id: Option<String>,
    },
}

/// Book level change (for delta updates).
#[derive(Debug, Clone, Deserialize)]
pub struct BookChange {
    /// Price level.
    pub price: Price,
    /// New quantity (0 means level removed).
    pub quantity: Quantity,
}

impl BookChange {
    /// Returns true if this change removes the level.
    #[must_use]
    pub fn is_removal(&self) -> bool {
        self.quantity.is_zero()
    }
}

/// Subscription info for tracking active subscriptions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Subscription {
    /// Channel.
    pub channel: Channel,
    /// Market address (None for user-specific channels).
    pub market: Option<String>,
}

impl Subscription {
    /// Creates a new subscription.
    #[must_use]
    pub fn new(channel: Channel, market: Option<String>) -> Self {
        Self { channel, market }
    }

    /// Creates a book subscription.
    #[must_use]
    pub fn book(market: impl Into<String>) -> Self {
        Self::new(Channel::Book, Some(market.into()))
    }

    /// Creates a trades subscription.
    #[must_use]
    pub fn trades(market: impl Into<String>) -> Self {
        Self::new(Channel::Trades, Some(market.into()))
    }

    /// Creates an orders subscription.
    #[must_use]
    pub fn orders() -> Self {
        Self::new(Channel::Orders, None)
    }

    /// Creates a ticker subscription.
    #[must_use]
    pub fn ticker(market: impl Into<String>) -> Self {
        Self::new(Channel::Ticker, Some(market.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_display() {
        assert_eq!(Channel::Book.to_string(), "book");
        assert_eq!(Channel::Trades.to_string(), "trades");
        assert_eq!(Channel::Orders.to_string(), "orders");
        assert_eq!(Channel::Ticker.to_string(), "ticker");
    }

    #[test]
    fn test_client_message_subscribe_serialize() {
        let msg = ClientMessage::Subscribe {
            channel: Channel::Book,
            market: Some("ABC123".to_string()),
            depth: Some(20),
        };
        let json = serde_json::to_string(&msg).expect("serialize");
        assert!(json.contains("\"type\":\"subscribe\""));
        assert!(json.contains("\"channel\":\"book\""));
        assert!(json.contains("\"market\":\"ABC123\""));
        assert!(json.contains("\"depth\":20"));
    }

    #[test]
    fn test_client_message_ping_serialize() {
        let msg = ClientMessage::Ping {
            timestamp: 1706640000000,
        };
        let json = serde_json::to_string(&msg).expect("serialize");
        assert!(json.contains("\"type\":\"ping\""));
        assert!(json.contains("\"timestamp\":1706640000000"));
    }

    #[test]
    fn test_server_message_subscribed_deserialize() {
        let json = r#"{"type":"subscribed","channel":"book","market":"ABC123"}"#;
        let msg: ServerMessage = serde_json::from_str(json).expect("deserialize");
        match msg {
            ServerMessage::Subscribed { channel, market } => {
                assert_eq!(channel, Channel::Book);
                assert_eq!(market, Some("ABC123".to_string()));
            }
            _ => panic!("unexpected message type"),
        }
    }

    #[test]
    fn test_server_message_pong_deserialize() {
        let json = r#"{"type":"pong","timestamp":1706640000000}"#;
        let msg: ServerMessage = serde_json::from_str(json).expect("deserialize");
        match msg {
            ServerMessage::Pong { timestamp } => {
                assert_eq!(timestamp, 1706640000000);
            }
            _ => panic!("unexpected message type"),
        }
    }

    #[test]
    fn test_server_message_error_deserialize() {
        let json = r#"{"type":"error","code":"INVALID_MARKET","message":"market not found","request_id":null}"#;
        let msg: ServerMessage = serde_json::from_str(json).expect("deserialize");
        match msg {
            ServerMessage::Error { code, message, .. } => {
                assert_eq!(code, "INVALID_MARKET");
                assert_eq!(message, "market not found");
            }
            _ => panic!("unexpected message type"),
        }
    }

    #[test]
    fn test_book_change_is_removal() {
        let change = BookChange {
            price: Price::new(100),
            quantity: Quantity::zero(),
        };
        assert!(change.is_removal());

        let change = BookChange {
            price: Price::new(100),
            quantity: Quantity::new(50),
        };
        assert!(!change.is_removal());
    }

    #[test]
    fn test_subscription_constructors() {
        let sub = Subscription::book("ABC123");
        assert_eq!(sub.channel, Channel::Book);
        assert_eq!(sub.market, Some("ABC123".to_string()));

        let sub = Subscription::trades("ABC123");
        assert_eq!(sub.channel, Channel::Trades);

        let sub = Subscription::orders();
        assert_eq!(sub.channel, Channel::Orders);
        assert!(sub.market.is_none());
    }
}
