# M2 Issue #21: Indexer: Event processor for fills and cancellations

## Overview

Implement the event processor that consumes events from the on-chain event queue and persists trade/cancellation data to the database.

## Acceptance Criteria

- [ ] Process Fill events:
  - Extract maker, taker, price, quantity, fees
  - Persist to trades table
  - Update OHLCV candles (1m, 5m, 15m, 1h, 4h, 1d)
  - Publish to trade stream for WebSocket
- [ ] Process Out events:
  - Update order status to cancelled
  - Record cancellation reason and timestamp
- [ ] Idempotent processing (handle replays)
- [ ] Sequence number tracking for ordering
- [ ] Batch inserts for performance
- [ ] Metrics: events/sec, processing latency, error rate
- [ ] Unit tests for event processing logic

## Technical Approach

### 1. Project Structure

Add to the `indexer` crate:

```
indexer/src/
├── events/
│   ├── mod.rs           # Module exports
│   ├── types.rs         # ProcessedFill, ProcessedOut, OrderUpdate types
│   ├── processor.rs     # EventProcessor implementation
│   ├── cursor.rs        # EventCursor for tracking progress
│   └── metrics.rs       # Event processor metrics
└── lib.rs               # Updated with events module
```

### 2. Core Types

```rust
pub struct ProcessedFill {
    pub market: [u8; 32],
    pub maker_order_id: u128,
    pub taker_order_id: u128,
    pub maker_address: [u8; 32],
    pub taker_address: [u8; 32],
    pub price: u64,
    pub quantity: u64,
    pub taker_side: Side,
    pub taker_fee: u64,
    pub maker_rebate: u64,
    pub slot: u64,
    pub seq_num: u64,
    pub timestamp: DateTime<Utc>,
}

pub struct ProcessedOut {
    pub market: [u8; 32],
    pub order_id: u128,
    pub owner: [u8; 32],
    pub reason: OutReason,
    pub slot: u64,
    pub seq_num: u64,
    pub timestamp: DateTime<Utc>,
}

pub enum OrderUpdate {
    Filled { ... },
    Cancelled { ... },
}
```

### 3. EventProcessor

```rust
pub struct EventProcessor {
    cursors: HashMap<[u8; 32], EventCursor>,
    metrics: Arc<EventMetrics>,
}

impl EventProcessor {
    pub fn process_events(&mut self, market: [u8; 32], events: &[ParsedEvent], head_seq: u64) -> ProcessResult;
    pub fn get_cursor(&self, market: &[u8; 32]) -> Option<&EventCursor>;
    pub fn is_processed(&self, market: &[u8; 32], seq_num: u64) -> bool;
}
```

### 4. EventCursor

```rust
pub struct EventCursor {
    pub head_seq_num: u64,
    pub last_processed: u64,
}
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `indexer/src/lib.rs` | Modify | Add events module |
| `indexer/src/events/mod.rs` | Create | Module exports |
| `indexer/src/events/types.rs` | Create | ProcessedFill, ProcessedOut, OrderUpdate |
| `indexer/src/events/processor.rs` | Create | EventProcessor implementation |
| `indexer/src/events/cursor.rs` | Create | EventCursor for tracking |
| `indexer/src/events/metrics.rs` | Create | Event processor metrics |

## Notes

- Idempotency via sequence number tracking
- Skip already-processed events
- Batch processing for performance
- Metrics for monitoring throughput
