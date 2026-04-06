# M3 Issue #32: SDK examples: Market data, trading, and market making

## Overview

Create comprehensive example applications demonstrating how to use the Rust and TypeScript SDKs for common use cases: fetching market data, placing trades, and building a simple market maker.

## Acceptance Criteria

### Rust Examples
- [ ] `examples/market_data.rs` — Fetch and display market info, order book, trades
- [ ] `examples/place_order.rs` — Place a limit order and monitor status
- [ ] `examples/cancel_order.rs` — Cancel an existing order
- [ ] `examples/stream_book.rs` — WebSocket subscription for book updates
- [ ] `examples/simple_mm.rs` — Basic market maker (quote both sides)

### TypeScript Examples
- [ ] `examples/market-data.ts` — Fetch market data
- [ ] `examples/place-order.ts` — Place and monitor order
- [ ] `examples/stream-book.ts` — WebSocket book streaming
- [ ] `examples/simple-mm.ts` — Basic market maker

### Requirements
- [ ] Clear comments explaining each step
- [ ] Error handling
- [ ] Configuration via environment variables
- [ ] README with setup instructions
- [ ] Examples work on devnet
- [ ] CI verification that examples compile/run

## Technical Approach

### 1. Rust Examples Structure

```
sdk/examples/
├── market_data.rs    # Fetch market info, orderbook, trades
├── place_order.rs    # Place limit order
├── cancel_order.rs   # Cancel order
├── stream_book.rs    # WebSocket book streaming
└── simple_mm.rs      # Simple market maker
```

### 2. TypeScript Examples Structure

```
ts-sdk/examples/
├── market-data.ts    # Fetch market data
├── place-order.ts    # Place order
├── stream-book.ts    # WebSocket streaming
├── simple-mm.ts      # Simple market maker
└── README.md         # Setup instructions
```

### 3. Environment Variables

```bash
MATCHBOOK_API_URL=https://api.matchbook.example/v1
MATCHBOOK_WS_URL=wss://ws.matchbook.example/v1/stream
MATCHBOOK_API_KEY=your-api-key
MATCHBOOK_MARKET=market-address
MATCHBOOK_WALLET=path/to/wallet.json
```

## Files to Create

| File | Description |
|------|-------------|
| `sdk/examples/market_data.rs` | Rust market data example |
| `sdk/examples/place_order.rs` | Rust place order example |
| `sdk/examples/cancel_order.rs` | Rust cancel order example |
| `sdk/examples/stream_book.rs` | Rust WebSocket example |
| `sdk/examples/simple_mm.rs` | Rust market maker example |
| `sdk/examples/README.md` | Rust examples documentation |
| `ts-sdk/examples/market-data.ts` | TypeScript market data example |
| `ts-sdk/examples/place-order.ts` | TypeScript place order example |
| `ts-sdk/examples/stream-book.ts` | TypeScript WebSocket example |
| `ts-sdk/examples/simple-mm.ts` | TypeScript market maker example |
| `ts-sdk/examples/README.md` | TypeScript examples documentation |

## Notes

- Use devnet for all examples
- Market maker example is educational, not production-ready
- Include rate limiting awareness
- Show proper cleanup (unsubscribe, close connections)
