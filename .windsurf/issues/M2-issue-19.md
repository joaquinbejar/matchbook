# M2 Issue #19: Indexer: Account parser for on-chain state deserialization

## Overview

Implement the account parser that deserializes raw Solana account data into structured Rust types. This component transforms binary account data into usable domain objects.

## Acceptance Criteria

- [ ] Parser for Market account data
- [ ] Parser for OrderBookSide (Bids/Asks) with B+ tree traversal
- [ ] Parser for EventQueue with ring buffer iteration
- [ ] Parser for OpenOrders account
- [ ] Handle account discriminators (Anchor format)
- [ ] Graceful handling of malformed/unknown accounts
- [ ] Version detection for future upgrades
- [ ] Metrics: parse time, error rate
- [ ] Unit tests with real account data snapshots

## Technical Approach

### 1. Project Structure

Add to the `indexer` crate:

```
indexer/src/
├── parser/
│   ├── mod.rs           # Module exports
│   ├── types.rs         # ParsedAccount, ParseError types
│   ├── discriminators.rs # Account discriminator constants
│   ├── market.rs        # Market account parser
│   ├── orderbook.rs     # OrderBook parser with tree traversal
│   ├── event_queue.rs   # EventQueue parser with ring buffer
│   ├── open_orders.rs   # OpenOrders parser
│   └── metrics.rs       # Parser metrics
└── lib.rs               # Updated with parser module
```

### 2. Core Types

```rust
pub enum ParsedAccount {
    Market(ParsedMarket),
    OrderBookSide { market: [u8; 32], side: Side, orders: Vec<ParsedOrder> },
    EventQueue { market: [u8; 32], events: Vec<ParsedEvent> },
    OpenOrders(ParsedOpenOrders),
    Unknown { discriminator: [u8; 8] },
}

pub struct ParsedMarket {
    pub address: [u8; 32],
    pub base_mint: [u8; 32],
    pub quote_mint: [u8; 32],
    pub base_lot_size: u64,
    pub quote_lot_size: u64,
    pub tick_size: u64,
    pub taker_fee_bps: u16,
    pub maker_fee_bps: i16,
    pub status: u8,
}

pub struct ParsedOrder {
    pub order_id: u128,
    pub owner: [u8; 32],
    pub price: u64,
    pub quantity: u64,
    pub client_order_id: u64,
}

pub struct ParsedEvent {
    pub event_type: EventType,
    pub slot: u64,
    pub timestamp: u64,
    // Event-specific data
}
```

### 3. AccountParser

```rust
pub struct AccountParser {
    metrics: Arc<ParserMetrics>,
}

impl AccountParser {
    pub fn parse(&self, data: &[u8]) -> Result<ParsedAccount, ParseError>;
    pub fn parse_market(&self, data: &[u8]) -> Result<ParsedMarket, ParseError>;
    pub fn parse_orderbook(&self, data: &[u8]) -> Result<Vec<ParsedOrder>, ParseError>;
    pub fn parse_event_queue(&self, data: &[u8]) -> Result<Vec<ParsedEvent>, ParseError>;
    pub fn parse_open_orders(&self, data: &[u8]) -> Result<ParsedOpenOrders, ParseError>;
}
```

### 4. Discriminators

Use Anchor-style 8-byte discriminators derived from account type names.

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `indexer/src/lib.rs` | Modify | Add parser module |
| `indexer/src/parser/mod.rs` | Create | Module exports |
| `indexer/src/parser/types.rs` | Create | ParsedAccount, ParseError types |
| `indexer/src/parser/discriminators.rs` | Create | Account discriminator constants |
| `indexer/src/parser/market.rs` | Create | Market account parser |
| `indexer/src/parser/orderbook.rs` | Create | OrderBook parser |
| `indexer/src/parser/event_queue.rs` | Create | EventQueue parser |
| `indexer/src/parser/open_orders.rs` | Create | OpenOrders parser |
| `indexer/src/parser/metrics.rs` | Create | Parser metrics |

## Notes

- Share types with on-chain program where possible
- Use zero-copy deserialization for large accounts
- Handle partial updates (only changed fields)
- Graceful error handling for malformed data
