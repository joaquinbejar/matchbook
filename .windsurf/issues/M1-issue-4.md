# M1 Issue #4: EventQueue Account (Ring Buffer)

## Overview

Implement the EventQueue account as a ring buffer for storing fill and cancel events. This enables asynchronous event processing and decouples matching from settlement.

## Acceptance Criteria

- [ ] `EventQueueHeader` struct with ring buffer metadata:
  - `bump`: u8
  - `market`: Pubkey
  - `head`: u32 (next event to consume)
  - `count`: u32 (number of pending events)
  - `seq_num`: u64 (monotonic sequence)
- [ ] `Event` enum with variants:
  - `Fill`: maker, taker, price, quantity, maker_fee, taker_fee, side, timestamp
  - `Out`: owner, order_id, side, quantity_released, timestamp
- [ ] PDA derivation: `["event_queue", market]`
- [ ] Push and pop operations with wrap-around
- [ ] Capacity check (reject if queue full)
- [ ] Unit tests for ring buffer operations

## Technical Approach

### 1. Project Structure

```
program/src/
├── state/
│   ├── mod.rs              # Update exports
│   ├── market.rs           # Existing
│   ├── orderbook.rs        # Existing
│   └── event_queue.rs      # New - EventQueue structures
```

### 2. Event Types

Based on `.internalDoc/03-onchain-design.md`:

```rust
/// Event type discriminator
#[repr(u8)]
pub enum EventType {
    Fill = 0,
    Out = 1,
}

/// Fill event - emitted when orders match
pub struct FillEvent {
    pub taker_side: Side,
    pub maker: Pubkey,
    pub maker_order_id: u128,
    pub maker_client_order_id: u64,
    pub taker: Pubkey,
    pub taker_order_id: u128,
    pub taker_client_order_id: u64,
    pub price: u64,
    pub quantity: u64,
    pub taker_fee: u64,
    pub maker_rebate: u64,
}

/// Out event - emitted when order leaves the book
pub struct OutEvent {
    pub side: Side,
    pub owner: Pubkey,
    pub order_id: u128,
    pub client_order_id: u64,
    pub base_released: u64,
    pub quote_released: u64,
    pub reason: OutReason,
}

/// Reason for order removal
#[repr(u8)]
pub enum OutReason {
    Cancelled = 0,
    Filled = 1,
    Expired = 2,
}
```

### 3. EventQueue Header

```rust
#[account(zero_copy)]
pub struct EventQueueHeader {
    pub bump: u8,
    pub padding: [u8; 7],
    pub market: Pubkey,
    pub head: u32,
    pub count: u32,
    pub seq_num: u64,
    pub reserved: [u8; 64],
}
```

### 4. Ring Buffer Operations

```rust
impl EventQueueHeader {
    /// Push an event to the queue
    pub fn push(&mut self, event: Event, events: &mut [Event]) -> Result<u64> {
        if self.is_full(events.len()) {
            return Err(error!(ErrorCode::EventQueueFull));
        }
        let tail = self.tail(events.len());
        events[tail] = event;
        self.count += 1;
        self.seq_num += 1;
        Ok(self.seq_num)
    }

    /// Pop an event from the queue
    pub fn pop(&mut self, events: &[Event]) -> Option<Event> {
        if self.is_empty() {
            return None;
        }
        let event = events[self.head as usize];
        self.head = (self.head + 1) % events.len() as u32;
        self.count -= 1;
        Some(event)
    }

    /// Calculate tail index
    fn tail(&self, capacity: usize) -> usize {
        ((self.head as usize) + (self.count as usize)) % capacity
    }
}
```

### 5. Space Calculation

```
Header: 1 + 7 + 32 + 4 + 4 + 8 + 64 = 120 bytes
Per event: ~128 bytes (using tagged enum)

For 2048 events:
  2048 * 128 + 120 = 262,264 bytes ≈ 256 KB
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `program/src/state/event_queue.rs` | Create | EventQueue structures and operations |
| `program/src/state/mod.rs` | Modify | Export event_queue types |
| `program/src/lib.rs` | Modify | Re-export event_queue types |

## Tests Needed

- `test_event_queue_pda_derivation` - Verify PDA seeds
- `test_push_single_event` - Push one event
- `test_push_pop_fifo` - Verify FIFO ordering
- `test_wrap_around` - Test ring buffer wrap-around
- `test_queue_full` - Verify capacity check
- `test_queue_empty_pop` - Pop from empty queue returns None
- `test_fill_event_creation` - Create fill event
- `test_out_event_creation` - Create out event

## Notes

- Ring buffer provides O(1) push/pop operations
- Events consumed by ConsumeEvents instruction (future issue)
- Queue full blocks matching until events are consumed
- Sequence number is monotonically increasing for ordering
