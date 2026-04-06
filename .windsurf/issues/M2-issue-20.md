# M2 Issue #20: Indexer: Book builder for order book reconstruction

## Overview

Implement the book builder component that maintains an in-memory representation of the order book and computes deltas for real-time updates to clients.

## Acceptance Criteria

- [ ] In-memory order book representation per market
- [ ] Efficient update application from parsed account data
- [ ] Delta computation (added/removed/changed levels)
- [ ] Aggregated book levels (price → total quantity)
- [ ] Best bid/ask tracking
- [ ] Spread calculation
- [ ] Snapshot generation for new WebSocket subscribers
- [ ] Configurable depth limits for API responses
- [ ] Metrics: book depth, spread, update latency
- [ ] Unit tests for book operations and delta computation

## Technical Approach

### 1. Project Structure

Add to the `indexer` crate:

```
indexer/src/
├── book/
│   ├── mod.rs           # Module exports
│   ├── types.rs         # PriceLevel, BookChange, BookUpdate types
│   ├── orderbook.rs     # FullOrderBook implementation
│   ├── builder.rs       # BookBuilder implementation
│   └── metrics.rs       # Book metrics
└── lib.rs               # Updated with book module
```

### 2. Core Types

```rust
pub struct PriceLevel {
    pub price: u64,
    pub quantity: u64,
    pub order_count: u32,
}

pub struct BookChange {
    pub side: Side,
    pub price: u64,
    pub new_quantity: u64,  // 0 = level removed
    pub order_count: u32,
}

pub enum BookUpdate {
    Snapshot { market: [u8; 32], bids: Vec<PriceLevel>, asks: Vec<PriceLevel>, slot: u64, seq: u64 },
    Delta { market: [u8; 32], changes: Vec<BookChange>, slot: u64, seq: u64 },
}
```

### 3. FullOrderBook

```rust
pub struct FullOrderBook {
    pub market: [u8; 32],
    pub bids: BTreeMap<u64, PriceLevel>,  // price -> level (descending for bids)
    pub asks: BTreeMap<u64, PriceLevel>,  // price -> level (ascending for asks)
    pub last_slot: u64,
    pub seq: u64,
}

impl FullOrderBook {
    pub fn best_bid(&self) -> Option<&PriceLevel>;
    pub fn best_ask(&self) -> Option<&PriceLevel>;
    pub fn spread(&self) -> Option<u64>;
    pub fn aggregate_bids(&self, depth: usize) -> Vec<PriceLevel>;
    pub fn aggregate_asks(&self, depth: usize) -> Vec<PriceLevel>;
    pub fn apply_orders(&mut self, side: Side, orders: &[ParsedOrder]) -> Vec<BookChange>;
}
```

### 4. BookBuilder

```rust
pub struct BookBuilder {
    books: HashMap<[u8; 32], FullOrderBook>,
    metrics: Arc<BookMetrics>,
}

impl BookBuilder {
    pub fn apply_update(&mut self, market: [u8; 32], side: Side, orders: Vec<ParsedOrder>, slot: u64) -> Vec<BookChange>;
    pub fn get_snapshot(&self, market: &[u8; 32], depth: usize) -> Option<BookUpdate>;
    pub fn get_book(&self, market: &[u8; 32]) -> Option<&FullOrderBook>;
}
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `indexer/src/lib.rs` | Modify | Add book module |
| `indexer/src/book/mod.rs` | Create | Module exports |
| `indexer/src/book/types.rs` | Create | PriceLevel, BookChange, BookUpdate |
| `indexer/src/book/orderbook.rs` | Create | FullOrderBook implementation |
| `indexer/src/book/builder.rs` | Create | BookBuilder implementation |
| `indexer/src/book/metrics.rs` | Create | Book metrics |

## Notes

- Use BTreeMap for sorted price levels
- Bids: highest price first (reverse iteration)
- Asks: lowest price first (normal iteration)
- Delta = diff between previous and current state
- Thread-safe reads via Arc<RwLock<>> if needed
