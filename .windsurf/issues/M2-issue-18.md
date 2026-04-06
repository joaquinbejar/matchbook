# M2 Issue #18: Indexer: Geyser listener for account subscriptions

## Overview

Implement the Geyser listener component that subscribes to Solana account updates in real-time. This is the entry point for all on-chain data into the off-chain system.

## Acceptance Criteria

- [ ] Geyser plugin client connection management
- [ ] Subscribe to account updates for:
  - Market accounts
  - OrderBook accounts (Bids/Asks)
  - EventQueue accounts
  - OpenOrders accounts (optional, for user-specific tracking)
- [ ] Handle connection drops and automatic reconnection
- [ ] Backfill mechanism for missed updates during downtime
- [ ] Slot tracking for consistency
- [ ] Metrics: updates/sec, lag, connection status
- [ ] Configuration for RPC endpoints and filters
- [ ] Unit tests with mock Geyser responses

## Technical Approach

### 1. Project Structure

Add to the `indexer` crate:

```
indexer/src/
├── geyser/
│   ├── mod.rs           # Module exports
│   ├── config.rs        # Configuration types
│   ├── listener.rs      # GeyserListener implementation
│   ├── types.rs         # AccountUpdate, SubscriptionFilter types
│   └── metrics.rs       # Metrics tracking
└── lib.rs               # Updated with geyser module
```

### 2. Core Types

```rust
pub struct GeyserConfig {
    pub endpoint: String,
    pub x_token: Option<String>,
    pub reconnect_delay_ms: u64,
    pub max_reconnect_attempts: u32,
    pub program_id: String,
}

pub struct AccountUpdate {
    pub pubkey: [u8; 32],
    pub slot: u64,
    pub data: Vec<u8>,
    pub write_version: u64,
    pub is_startup: bool,
}

pub enum AccountType {
    Market,
    Bids,
    Asks,
    EventQueue,
    OpenOrders,
    Unknown,
}
```

### 3. GeyserListener

```rust
pub struct GeyserListener {
    config: GeyserConfig,
    sender: mpsc::Sender<AccountUpdate>,
    metrics: GeyserMetrics,
    last_slot: AtomicU64,
    connection_state: AtomicU8,
}

impl GeyserListener {
    pub async fn connect(&mut self) -> Result<()>;
    pub async fn subscribe(&mut self) -> Result<()>;
    pub async fn reconnect(&mut self) -> Result<()>;
    pub fn last_slot(&self) -> u64;
    pub fn is_connected(&self) -> bool;
}
```

### 4. Metrics

```rust
pub struct GeyserMetrics {
    pub updates_received: AtomicU64,
    pub updates_per_second: AtomicU64,
    pub last_update_slot: AtomicU64,
    pub connection_status: AtomicU8,
    pub reconnect_count: AtomicU64,
    pub lag_slots: AtomicI64,
}
```

### 5. Dependencies

Add to `indexer/Cargo.toml`:
- `yellowstone-grpc-client` for Geyser gRPC
- `yellowstone-grpc-proto` for protobuf types
- `tonic` for gRPC
- `tokio` (already present)
- `tracing` (already present)

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `indexer/Cargo.toml` | Modify | Add yellowstone-grpc dependencies |
| `indexer/src/lib.rs` | Modify | Add geyser module |
| `indexer/src/geyser/mod.rs` | Create | Module exports |
| `indexer/src/geyser/config.rs` | Create | Configuration types |
| `indexer/src/geyser/listener.rs` | Create | GeyserListener implementation |
| `indexer/src/geyser/types.rs` | Create | AccountUpdate and related types |
| `indexer/src/geyser/metrics.rs` | Create | Metrics tracking |

## Notes

- Use Yellowstone gRPC for Geyser plugin access
- Filter by program ID to reduce noise
- Track slot numbers to detect gaps
- Implement exponential backoff for reconnection
- Use channels for backpressure handling
