# M3 Issue #26: Rust SDK: Core types and shared definitions

## Overview

Implement the core types for the Rust SDK that mirror the on-chain types and provide a foundation for client-side operations. These types are shared across HTTP and WebSocket clients.

## Acceptance Criteria

### Core Value Types
- [ ] `Price`, `Quantity` with arithmetic operations
- [ ] `Side` (Bid/Ask)
- [ ] `OrderType` (Limit, PostOnly, IOC, FOK)
- [ ] `TimeInForce` (GTC, IOC, FOK, PostOnly)
- [ ] `OrderStatus` (Open, PartiallyFilled, Filled, Cancelled)

### Entity Types
- [ ] `Market` — Market configuration and state
- [ ] `Order` — Order details
- [ ] `Trade` — Executed trade
- [ ] `BookLevel` — Aggregated price level
- [ ] `OrderBook` — Full book with bids/asks
- [ ] `Balance` — User balances

### Requirements
- [ ] Serialization/deserialization (serde)
- [ ] Conversion to/from on-chain types
- [ ] Display and Debug implementations
- [ ] Comprehensive unit tests
- [ ] Documentation with examples

## Technical Approach

### 1. Project Structure

Create a new `sdk` crate:

```
sdk/
├── Cargo.toml
├── src/
│   ├── lib.rs           # Crate root with exports
│   ├── types/
│   │   ├── mod.rs       # Module exports
│   │   ├── primitives.rs # Price, Quantity, Side, etc.
│   │   ├── market.rs    # Market type
│   │   ├── order.rs     # Order, OrderStatus, OrderType
│   │   ├── book.rs      # BookLevel, OrderBook
│   │   ├── trade.rs     # Trade type
│   │   └── balance.rs   # Balance type
│   └── error.rs         # SDK errors
```

### 2. Core Types Design

```rust
// Newtype wrappers for type safety
pub struct Price(u64);
pub struct Quantity(u64);

// Enums with serde support
pub enum Side { Bid, Ask }
pub enum OrderType { Limit, PostOnly, ImmediateOrCancel, FillOrKill }
pub enum TimeInForce { GoodTilCancelled, ImmediateOrCancel, FillOrKill, PostOnly }
pub enum OrderStatus { Open, PartiallyFilled, Filled, Cancelled, Expired }
```

### 3. Entity Types

- Market: addresses, tick/lot sizes, decimals, fees
- Order: id, owner, side, price, quantity, status
- Trade: id, market, price, quantity, maker/taker, timestamp
- BookLevel: price, quantity, order count
- OrderBook: market, bids, asks, sequence
- Balance: owner, base/quote free/locked

## Files to Create

| File | Description |
|------|-------------|
| `sdk/Cargo.toml` | SDK crate dependencies |
| `sdk/src/lib.rs` | Crate root with exports |
| `sdk/src/error.rs` | SDK error types |
| `sdk/src/types/mod.rs` | Types module exports |
| `sdk/src/types/primitives.rs` | Price, Quantity, Side, enums |
| `sdk/src/types/market.rs` | Market type |
| `sdk/src/types/order.rs` | Order types |
| `sdk/src/types/book.rs` | OrderBook types |
| `sdk/src/types/trade.rs` | Trade type |
| `sdk/src/types/balance.rs` | Balance type |
| `Cargo.toml` | Add sdk to workspace |

## Notes

- Use newtype pattern for Price/Quantity for type safety
- Implement Copy for small value types
- Use rust_decimal for precise decimal handling
- Add serde with camelCase for JSON compatibility
- Implement Display for human-readable output
