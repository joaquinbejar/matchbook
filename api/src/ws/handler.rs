//! WebSocket connection handler.
//!
//! Provides the main WebSocket upgrade handler and connection loop.

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use super::channels::{Channel, ChannelManager};
use super::connection::Connection;
use super::messages::{ClientMessage, ServerMessage};
use super::metrics::WsMetrics;
use crate::state::AppState;

/// WebSocket state shared across connections.
#[derive(Clone)]
pub struct WsState {
    /// Channel manager.
    pub channels: Arc<ChannelManager>,
    /// Metrics.
    pub metrics: Arc<WsMetrics>,
    /// Application state.
    pub app_state: AppState,
}

impl WsState {
    /// Creates a new WebSocket state.
    #[must_use]
    pub fn new(app_state: AppState) -> Self {
        Self {
            channels: Arc::new(ChannelManager::new()),
            metrics: Arc::new(WsMetrics::new()),
            app_state,
        }
    }
}

/// WebSocket upgrade handler.
///
/// Upgrades an HTTP connection to a WebSocket connection.
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<WsState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_connection(socket, state))
}

/// Handles a WebSocket connection.
async fn handle_connection(socket: WebSocket, state: WsState) {
    state.metrics.record_connection_opened();
    info!("WebSocket connection opened");

    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Create channel for outgoing messages
    let (tx, mut rx) = mpsc::channel::<ServerMessage>(100);

    let mut connection = Connection::new(tx);
    let connection_id = connection.id();

    // Spawn task to forward messages from channel to WebSocket
    let metrics = Arc::clone(&state.metrics);
    let sender_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let json = match serde_json::to_string(&msg) {
                Ok(j) => j,
                Err(e) => {
                    error!("Failed to serialize message: {}", e);
                    continue;
                }
            };

            if ws_sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
            metrics.record_message_sent();
        }
    });

    // Handle incoming messages
    while let Some(result) = ws_receiver.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                warn!("WebSocket error: {}", e);
                state.metrics.record_error();
                break;
            }
        };

        state.metrics.record_message_received();

        match msg {
            Message::Text(text) => {
                if let Err(e) = handle_text_message(&text, &mut connection, &state).await {
                    error!("Error handling message: {}", e);
                    let _ = connection.send(ServerMessage::error("ERROR", e)).await;
                }
            }
            Message::Ping(_data) => {
                let _ = connection.send(ServerMessage::pong()).await;
                debug!("Received ping, sent pong");
            }
            Message::Pong(_) => {
                debug!("Received pong");
            }
            Message::Close(_) => {
                info!("WebSocket close requested");
                break;
            }
            _ => {}
        }
    }

    // Cleanup
    state.channels.unsubscribe_all(connection_id).await;
    state.metrics.record_connection_closed();
    sender_task.abort();

    info!("WebSocket connection closed");
}

/// Handles a text message from the client.
async fn handle_text_message(
    text: &str,
    connection: &mut Connection,
    state: &WsState,
) -> Result<(), String> {
    let msg: ClientMessage =
        serde_json::from_str(text).map_err(|e| format!("Invalid JSON: {}", e))?;

    match msg {
        ClientMessage::Subscribe { channel, token } => {
            handle_subscribe(channel, token, connection, state).await
        }
        ClientMessage::Unsubscribe { channel } => {
            handle_unsubscribe(channel, connection, state).await
        }
        ClientMessage::Ping => {
            connection.send(ServerMessage::pong()).await;
            Ok(())
        }
    }
}

/// Handles a subscribe request.
async fn handle_subscribe(
    channel_str: String,
    token: Option<String>,
    connection: &mut Connection,
    state: &WsState,
) -> Result<(), String> {
    let channel =
        Channel::parse(&channel_str).ok_or_else(|| format!("Invalid channel: {}", channel_str))?;

    // Handle authentication for private channels
    if channel.requires_auth() {
        if let Some(_token) = token {
            // TODO: Validate token and extract owner
            // For now, just reject
            return Err("Authentication not implemented".to_string());
        } else {
            return Err("Authentication required for this channel".to_string());
        }
    }

    // Check if already subscribed
    if connection.is_subscribed(&channel) {
        return Err(format!("Already subscribed to {}", channel_str));
    }

    // Check authorization
    if !connection.can_subscribe(&channel) {
        return Err("Not authorized to subscribe to this channel".to_string());
    }

    // Subscribe
    connection.subscribe(channel.clone());
    state
        .channels
        .subscribe(channel.clone(), connection.id())
        .await;
    state.metrics.record_subscription();

    // Send confirmation
    connection
        .send(ServerMessage::subscribed(&channel_str))
        .await;

    // Send initial snapshot
    let snapshot_data = get_channel_snapshot(&channel, state).await;
    connection
        .send(ServerMessage::snapshot(&channel_str, snapshot_data))
        .await;

    debug!("Subscribed to channel: {}", channel_str);
    Ok(())
}

/// Handles an unsubscribe request.
async fn handle_unsubscribe(
    channel_str: String,
    connection: &mut Connection,
    state: &WsState,
) -> Result<(), String> {
    let channel =
        Channel::parse(&channel_str).ok_or_else(|| format!("Invalid channel: {}", channel_str))?;

    if !connection.is_subscribed(&channel) {
        return Err(format!("Not subscribed to {}", channel_str));
    }

    connection.unsubscribe(&channel);
    state.channels.unsubscribe(&channel, connection.id()).await;
    state.metrics.record_unsubscription();

    connection
        .send(ServerMessage::unsubscribed(&channel_str))
        .await;

    debug!("Unsubscribed from channel: {}", channel_str);
    Ok(())
}

/// Gets the initial snapshot for a channel.
async fn get_channel_snapshot(channel: &Channel, state: &WsState) -> serde_json::Value {
    match channel {
        Channel::Book { market } => {
            let book_builder = state.app_state.book_builder.read().await;
            if let Some(snapshot) = book_builder.get_snapshot(market, 20) {
                serde_json::to_value(snapshot).unwrap_or(serde_json::json!({}))
            } else {
                serde_json::json!({
                    "bids": [],
                    "asks": [],
                    "slot": 0,
                    "seq": 0
                })
            }
        }
        Channel::Trades { .. } => {
            // TODO: Get recent trades from database
            serde_json::json!({
                "trades": []
            })
        }
        Channel::Orders { .. } => {
            // TODO: Get user's open orders from database
            serde_json::json!({
                "orders": []
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_state_new() {
        let app_state = AppState::default();
        let ws_state = WsState::new(app_state);
        assert_eq!(ws_state.metrics.connections_opened(), 0);
    }

    #[tokio::test]
    async fn test_handle_subscribe_invalid_channel() {
        let app_state = AppState::default();
        let ws_state = WsState::new(app_state);
        let (tx, _rx) = mpsc::channel(10);
        let mut connection = Connection::new(tx);

        let result =
            handle_subscribe("invalid".to_string(), None, &mut connection, &ws_state).await;

        assert!(result.is_err());
        assert!(result.expect_err("error").contains("Invalid channel"));
    }

    #[tokio::test]
    async fn test_handle_subscribe_requires_auth() {
        let app_state = AppState::default();
        let ws_state = WsState::new(app_state);
        let (tx, _rx) = mpsc::channel(10);
        let mut connection = Connection::new(tx);

        let result = handle_subscribe(
            "orders:11111111111111111111111111111111".to_string(),
            None,
            &mut connection,
            &ws_state,
        )
        .await;

        assert!(result.is_err());
        assert!(result
            .expect_err("error")
            .contains("Authentication required"));
    }

    #[tokio::test]
    async fn test_handle_subscribe_success() {
        let app_state = AppState::default();
        let ws_state = WsState::new(app_state);
        let (tx, mut rx) = mpsc::channel(10);
        let mut connection = Connection::new(tx);

        let result = handle_subscribe(
            "book:11111111111111111111111111111111".to_string(),
            None,
            &mut connection,
            &ws_state,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(ws_state.metrics.subscriptions(), 1);

        // Should receive subscribed and snapshot messages
        let msg1 = rx.recv().await;
        assert!(msg1.is_some());

        let msg2 = rx.recv().await;
        assert!(msg2.is_some());
    }

    #[tokio::test]
    async fn test_handle_unsubscribe_not_subscribed() {
        let app_state = AppState::default();
        let ws_state = WsState::new(app_state);
        let (tx, _rx) = mpsc::channel(10);
        let mut connection = Connection::new(tx);

        let result = handle_unsubscribe(
            "book:11111111111111111111111111111111".to_string(),
            &mut connection,
            &ws_state,
        )
        .await;

        assert!(result.is_err());
        assert!(result.expect_err("error").contains("Not subscribed"));
    }
}
