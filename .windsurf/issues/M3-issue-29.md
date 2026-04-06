# M3 Issue #29: Rust SDK: WebSocket client for real-time streaming

## Overview

Implement the WebSocket client that connects to the streaming API for real-time order book updates, trades, and user order notifications.

## Acceptance Criteria

### MatchbookWsClient Configuration
- [ ] WebSocket URL
- [ ] Reconnection policy
- [ ] Heartbeat interval
- [ ] Optional authentication

### Subscription Management
- [ ] `subscribe_book(market)` — Order book updates
- [ ] `subscribe_trades(market)` — Trade stream
- [ ] `subscribe_orders(owner)` — User order updates (authenticated)
- [ ] `unsubscribe(channel)`

### Event Handling
- [ ] Async stream-based API for receiving events
- [ ] Message parsing into SDK types

### Connection Lifecycle
- [ ] Automatic reconnection with backoff
- [ ] Heartbeat/ping-pong handling
- [ ] Graceful shutdown

### Requirements
- [ ] Unit tests
- [ ] Documentation with examples

## Technical Approach

### 1. Project Structure

Add to existing `sdk` crate:

```
sdk/src/
├── ws/
│   ├── mod.rs           # Module exports
│   ├── config.rs        # WebSocket configuration
│   ├── client.rs        # WebSocket client implementation
│   ├── messages.rs      # Message types (subscribe, events)
│   └── error.rs         # WebSocket-specific errors
```

### 2. WebSocket Configuration

```rust
pub struct WsConfig {
    pub url: String,
    pub heartbeat_interval: Duration,
    pub reconnect_delay: Duration,
    pub max_reconnect_delay: Duration,
    pub api_key: Option<String>,
}
```

### 3. Message Types

```rust
// Client messages
pub enum ClientMessage {
    Subscribe { channel: Channel, market: String },
    Unsubscribe { channel: Channel, market: String },
    Ping { timestamp: u64 },
}

// Server messages
pub enum ServerMessage {
    Subscribed { channel: Channel, market: String },
    BookSnapshot { market: String, bids: Vec<BookLevel>, asks: Vec<BookLevel> },
    BookUpdate { market: String, bids: Vec<BookChange>, asks: Vec<BookChange> },
    Trade { market: String, trade: Trade },
    OrderUpdate { order: Order },
    Pong { timestamp: u64 },
    Error { code: String, message: String },
}
```

### 4. WebSocket Client

```rust
pub struct MatchbookWsClient {
    config: WsConfig,
    // Internal state
}

impl MatchbookWsClient {
    pub async fn connect(config: WsConfig) -> Result<Self, WsError>;
    pub async fn subscribe_book(&self, market: &str) -> Result<(), WsError>;
    pub async fn subscribe_trades(&self, market: &str) -> Result<(), WsError>;
    pub async fn subscribe_orders(&self) -> Result<(), WsError>;
    pub async fn unsubscribe(&self, channel: Channel, market: &str) -> Result<(), WsError>;
    pub fn events(&self) -> impl Stream<Item = ServerMessage>;
    pub async fn close(&self) -> Result<(), WsError>;
}
```

## Files to Create/Modify

| File | Description |
|------|-------------|
| `sdk/src/ws/mod.rs` | Module exports |
| `sdk/src/ws/config.rs` | WebSocket configuration |
| `sdk/src/ws/messages.rs` | Message types |
| `sdk/src/ws/error.rs` | WebSocket errors |
| `sdk/src/ws/client.rs` | WebSocket client |
| `sdk/src/lib.rs` | Add ws module |
| `sdk/Cargo.toml` | Add tokio-tungstenite, futures dependencies |

## Notes

- Use tokio-tungstenite for WebSocket
- Support async stream-based API
- Handle snapshot + delta pattern
- Thread-safe subscription management
