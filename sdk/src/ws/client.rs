//! WebSocket client implementation.
//!
//! Provides the main WebSocket client for real-time streaming.

use std::collections::HashSet;
use std::sync::Arc;

use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use super::config::WsConfig;
use super::error::WsError;
use super::messages::{Channel, ClientMessage, ServerMessage, Subscription};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WsSink = SplitSink<WsStream, Message>;
type WsSource = SplitStream<WsStream>;

/// WebSocket client for real-time streaming.
#[derive(Debug)]
pub struct MatchbookWsClient {
    config: WsConfig,
    sink: Arc<Mutex<Option<WsSink>>>,
    subscriptions: Arc<RwLock<HashSet<Subscription>>>,
    event_tx: mpsc::Sender<ServerMessage>,
    event_rx: Arc<Mutex<mpsc::Receiver<ServerMessage>>>,
    connected: Arc<RwLock<bool>>,
}

impl MatchbookWsClient {
    /// Creates a new WebSocket client with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn new(config: WsConfig) -> Result<Self, WsError> {
        config.validate()?;

        let (event_tx, event_rx) = mpsc::channel(1000);

        Ok(Self {
            config,
            sink: Arc::new(Mutex::new(None)),
            subscriptions: Arc::new(RwLock::new(HashSet::new())),
            event_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
            connected: Arc::new(RwLock::new(false)),
        })
    }

    /// Creates a new client with default configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn with_defaults() -> Result<Self, WsError> {
        Self::new(WsConfig::default())
    }

    /// Creates a new client with the given URL.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn with_url(url: impl Into<String>) -> Result<Self, WsError> {
        Self::new(WsConfig::new(url))
    }

    /// Returns the client configuration.
    #[must_use]
    pub fn config(&self) -> &WsConfig {
        &self.config
    }

    /// Returns true if connected.
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// Connects to the WebSocket server.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection fails.
    pub async fn connect(&self) -> Result<(), WsError> {
        let url = self.config.connection_url();

        let (ws_stream, _) = tokio_tungstenite::connect_async(&url)
            .await
            .map_err(|e| WsError::Connection(e.to_string()))?;

        let (sink, source) = ws_stream.split();

        *self.sink.lock().await = Some(sink);
        *self.connected.write().await = true;

        self.spawn_reader(source);
        self.spawn_heartbeat();

        Ok(())
    }

    /// Spawns the message reader task.
    fn spawn_reader(&self, mut source: WsSource) {
        let event_tx = self.event_tx.clone();
        let connected = self.connected.clone();

        tokio::spawn(async move {
            while let Some(result) = source.next().await {
                match result {
                    Ok(Message::Text(text)) => {
                        if let Ok(msg) = serde_json::from_str::<ServerMessage>(&text) {
                            let _ = event_tx.send(msg).await;
                        }
                    }
                    Ok(Message::Close(_)) => {
                        *connected.write().await = false;
                        break;
                    }
                    Err(_) => {
                        *connected.write().await = false;
                        break;
                    }
                    _ => {}
                }
            }
        });
    }

    /// Spawns the heartbeat task.
    fn spawn_heartbeat(&self) {
        let sink = self.sink.clone();
        let connected = self.connected.clone();
        let interval = self.config.heartbeat_interval;

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;

                if !*connected.read().await {
                    break;
                }

                let msg = ClientMessage::Ping {
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0),
                };

                if let Ok(json) = serde_json::to_string(&msg) {
                    if let Some(ref mut s) = *sink.lock().await {
                        let _ = s.send(Message::Text(json.into())).await;
                    }
                }
            }
        });
    }

    /// Sends a message to the server.
    async fn send(&self, msg: &ClientMessage) -> Result<(), WsError> {
        let json = serde_json::to_string(msg).map_err(|e| WsError::Serialization(e.to_string()))?;

        let mut sink_guard = self.sink.lock().await;
        let sink = sink_guard.as_mut().ok_or(WsError::NotConnected)?;

        sink.send(Message::Text(json.into()))
            .await
            .map_err(|e| WsError::SendFailed(e.to_string()))?;

        Ok(())
    }

    /// Subscribes to order book updates for a market.
    ///
    /// # Arguments
    ///
    /// * `market` - The market address
    /// * `depth` - Optional depth (number of levels)
    ///
    /// # Errors
    ///
    /// Returns an error if the subscription fails.
    pub async fn subscribe_book(&self, market: &str, depth: Option<u32>) -> Result<(), WsError> {
        let msg = ClientMessage::Subscribe {
            channel: Channel::Book,
            market: Some(market.to_string()),
            depth,
        };

        self.send(&msg).await?;

        self.subscriptions
            .write()
            .await
            .insert(Subscription::book(market));

        Ok(())
    }

    /// Subscribes to trade stream for a market.
    ///
    /// # Arguments
    ///
    /// * `market` - The market address
    ///
    /// # Errors
    ///
    /// Returns an error if the subscription fails.
    pub async fn subscribe_trades(&self, market: &str) -> Result<(), WsError> {
        let msg = ClientMessage::Subscribe {
            channel: Channel::Trades,
            market: Some(market.to_string()),
            depth: None,
        };

        self.send(&msg).await?;

        self.subscriptions
            .write()
            .await
            .insert(Subscription::trades(market));

        Ok(())
    }

    /// Subscribes to user order updates (requires authentication).
    ///
    /// # Errors
    ///
    /// Returns an error if the subscription fails.
    pub async fn subscribe_orders(&self) -> Result<(), WsError> {
        let msg = ClientMessage::Subscribe {
            channel: Channel::Orders,
            market: None,
            depth: None,
        };

        self.send(&msg).await?;

        self.subscriptions
            .write()
            .await
            .insert(Subscription::orders());

        Ok(())
    }

    /// Subscribes to ticker updates for a market.
    ///
    /// # Arguments
    ///
    /// * `market` - The market address
    ///
    /// # Errors
    ///
    /// Returns an error if the subscription fails.
    pub async fn subscribe_ticker(&self, market: &str) -> Result<(), WsError> {
        let msg = ClientMessage::Subscribe {
            channel: Channel::Ticker,
            market: Some(market.to_string()),
            depth: None,
        };

        self.send(&msg).await?;

        self.subscriptions
            .write()
            .await
            .insert(Subscription::ticker(market));

        Ok(())
    }

    /// Unsubscribes from a channel.
    ///
    /// # Arguments
    ///
    /// * `channel` - The channel to unsubscribe from
    /// * `market` - The market address (None for user-specific channels)
    ///
    /// # Errors
    ///
    /// Returns an error if the unsubscription fails.
    pub async fn unsubscribe(&self, channel: Channel, market: Option<&str>) -> Result<(), WsError> {
        let msg = ClientMessage::Unsubscribe {
            channel,
            market: market.map(String::from),
        };

        self.send(&msg).await?;

        self.subscriptions
            .write()
            .await
            .remove(&Subscription::new(channel, market.map(String::from)));

        Ok(())
    }

    /// Returns the next event from the server.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection is closed.
    pub async fn next_event(&self) -> Result<ServerMessage, WsError> {
        self.event_rx
            .lock()
            .await
            .recv()
            .await
            .ok_or(WsError::Closed)
    }

    /// Returns the current subscriptions.
    pub async fn subscriptions(&self) -> Vec<Subscription> {
        self.subscriptions.read().await.iter().cloned().collect()
    }

    /// Closes the connection gracefully.
    ///
    /// # Errors
    ///
    /// Returns an error if the close fails.
    pub async fn close(&self) -> Result<(), WsError> {
        *self.connected.write().await = false;

        if let Some(ref mut sink) = *self.sink.lock().await {
            let _ = sink.send(Message::Close(None)).await;
        }

        *self.sink.lock().await = None;
        self.subscriptions.write().await.clear();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_new() {
        let config = WsConfig::new("wss://example.com/ws");
        let client = MatchbookWsClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_with_defaults() {
        let client = MatchbookWsClient::with_defaults();
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_with_url() {
        let client = MatchbookWsClient::with_url("wss://example.com/ws");
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_invalid_config() {
        let config = WsConfig::new("");
        let client = MatchbookWsClient::new(config);
        assert!(client.is_err());
    }

    #[test]
    fn test_client_config_access() {
        let config = WsConfig::new("wss://example.com/ws").with_api_key("test-key");
        let client = MatchbookWsClient::new(config).expect("client creation");
        assert_eq!(client.config().url, "wss://example.com/ws");
        assert_eq!(client.config().api_key, Some("test-key".to_string()));
    }

    #[tokio::test]
    async fn test_client_not_connected_initially() {
        let client = MatchbookWsClient::with_defaults().expect("client creation");
        assert!(!client.is_connected().await);
    }

    #[tokio::test]
    async fn test_client_subscriptions_empty_initially() {
        let client = MatchbookWsClient::with_defaults().expect("client creation");
        assert!(client.subscriptions().await.is_empty());
    }
}
