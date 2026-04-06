# M2 Issue #23: API: WebSocket server for real-time updates

## Overview

Implement the WebSocket server that provides real-time streaming of order book updates, trades, and user-specific order updates to connected clients.

## Acceptance Criteria

- [ ] WebSocket endpoint: `/v1/ws`
- [ ] Channel subscriptions:
  - `book:{market}` — Order book deltas
  - `trades:{market}` — Real-time trades
  - `orders:{owner}` — User's order updates (requires auth)
- [ ] Message types:
  - `subscribe` / `unsubscribe` requests
  - `snapshot` — Initial state on subscription
  - `update` — Incremental updates
  - `error` — Error responses
- [ ] Connection management:
  - Heartbeat/ping-pong
  - Automatic reconnection guidance
  - Connection limits per IP
- [ ] Authentication for user-specific channels
- [ ] Backpressure handling for slow clients
- [ ] Metrics: connections, messages/sec, latency
- [ ] Unit tests for message handling

## Technical Approach

### 1. Project Structure

Add to the `api` crate:

```
api/src/
├── ws/
│   ├── mod.rs           # Module exports
│   ├── handler.rs       # WebSocket connection handler
│   ├── messages.rs      # Message types (subscribe, update, etc.)
│   ├── channels.rs      # Channel management (book, trades, orders)
│   ├── connection.rs    # Connection state and management
│   └── metrics.rs       # WebSocket metrics
```

### 2. Core Types

```rust
// Client -> Server messages
pub enum ClientMessage {
    Subscribe { channel: String },
    Unsubscribe { channel: String },
    Ping,
}

// Server -> Client messages
pub enum ServerMessage {
    Subscribed { channel: String },
    Unsubscribed { channel: String },
    Snapshot { channel: String, data: Value },
    Update { channel: String, data: Value },
    Error { code: String, message: String },
    Pong,
}

// Channel types
pub enum Channel {
    Book { market: [u8; 32] },
    Trades { market: [u8; 32] },
    Orders { owner: [u8; 32] },
}
```

### 3. WebSocket Handler

```rust
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_connection(socket, state))
}

async fn handle_connection(socket: WebSocket, state: AppState) {
    // Split into sender/receiver
    // Handle messages in loop
    // Manage subscriptions
    // Send updates from channels
}
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `api/src/ws/mod.rs` | Create | Module exports |
| `api/src/ws/messages.rs` | Create | Message types |
| `api/src/ws/channels.rs` | Create | Channel management |
| `api/src/ws/handler.rs` | Create | WebSocket handler |
| `api/src/ws/connection.rs` | Create | Connection state |
| `api/src/ws/metrics.rs` | Create | WebSocket metrics |
| `api/src/lib.rs` | Modify | Add ws module |
| `api/src/routes/mod.rs` | Modify | Add WebSocket route |

## Notes

- Use axum's built-in WebSocket support
- Book updates as deltas (price, quantity, side)
- Snapshot on subscribe, then deltas
- JSON message format per protocol spec
- Backpressure via bounded channels
