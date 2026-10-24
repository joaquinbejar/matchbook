# Matchbook TypeScript SDK Examples

This directory contains example applications demonstrating how to use the Matchbook TypeScript SDK.

## Prerequisites

- Node.js 18 or later
- npm or yarn
- A Matchbook API endpoint (devnet or local)
- Environment variables configured (see below)

## Setup

1. Install dependencies:

```bash
cd ts-sdk
npm install
```

2. Install ts-node for running examples:

```bash
npm install -g ts-node
```

## Environment Variables

All examples use environment variables for configuration:

| Variable | Description | Default |
|----------|-------------|---------|
| `MATCHBOOK_API_URL` | REST API base URL | `https://api.matchbook.example/v1` |
| `MATCHBOOK_WS_URL` | WebSocket URL | `wss://ws.matchbook.example/v1/stream` |
| `MATCHBOOK_MARKET` | Market address to use | Required |
| `MATCHBOOK_API_KEY` | API key for authentication | Required for some examples |

### Market Maker Specific

| Variable | Description | Default |
|----------|-------------|---------|
| `MM_SPREAD_BPS` | Spread in basis points | `10` (0.1%) |
| `MM_ORDER_SIZE` | Order size | `1.0` |

## Examples

### 1. Market Data (`market-data.ts`)

Fetches and displays market information, order book, and recent trades.

```bash
MATCHBOOK_MARKET=ABC123... npx ts-node examples/market-data.ts
```

**What it demonstrates:**
- Fetching all markets
- Getting specific market details
- Fetching order book with depth
- Fetching recent trades
- Calculating spread

### 2. Place Order (`place-order.ts`)

Places a limit order and shows how to monitor order status.

```bash
MATCHBOOK_MARKET=ABC123... \
MATCHBOOK_API_KEY=your-key \
npx ts-node examples/place-order.ts
```

**What it demonstrates:**
- Building order parameters
- Building a place order transaction
- Transaction signing workflow (conceptual)

### 3. Stream Order Book (`stream-book.ts`)

Subscribes to real-time order book updates via WebSocket.

```bash
MATCHBOOK_MARKET=ABC123... npx ts-node examples/stream-book.ts
```

**What it demonstrates:**
- WebSocket configuration
- Subscribing to order book channel
- Handling book snapshots and updates
- Proper cleanup and unsubscription
- Reconnection handling

### 4. Simple Market Maker (`simple-mm.ts`)

A basic market making strategy that quotes both sides of the book.

```bash
MATCHBOOK_MARKET=ABC123... \
MATCHBOOK_API_KEY=your-key \
MM_SPREAD_BPS=10 \
MM_ORDER_SIZE=1.0 \
npx ts-node examples/simple-mm.ts
```

**What it demonstrates:**
- Calculating bid/ask quotes from mid price
- Placing orders on both sides
- Updating quotes periodically
- Proper cleanup on shutdown

⚠️ **Warning:** This is an educational example only. Do NOT use for real trading without significant modifications including:
- Sophisticated pricing models
- Risk management
- Inventory management
- Latency optimization
- Proper error handling

## Running on Devnet

1. Set up a devnet wallet with SOL for transaction fees
2. Configure environment variables to point to devnet endpoints
3. Use a test market address

```bash
export MATCHBOOK_API_URL=https://devnet-api.matchbook.example/v1
export MATCHBOOK_WS_URL=wss://devnet-ws.matchbook.example/v1/stream
export MATCHBOOK_MARKET=<devnet-market-address>
export MATCHBOOK_API_KEY=<your-devnet-api-key>
```

## Error Handling

All examples include basic error handling. In production code, you should:

1. Use typed errors for different failure modes
2. Implement retry logic for transient failures
3. Handle rate limiting gracefully
4. Log errors for debugging

```typescript
import { isRateLimitError, isNotFoundError } from '@matchbook/sdk';

try {
  const market = await client.getMarket(address);
} catch (error) {
  if (isRateLimitError(error)) {
    console.log(`Rate limited, retry after ${error.retryAfter}ms`);
  } else if (isNotFoundError(error)) {
    console.log('Market not found');
  } else {
    throw error;
  }
}
```

## Rate Limiting

Be aware of API rate limits:

- REST API: Typically 100 requests/second
- WebSocket: Limited subscriptions per connection
- Don't spam subscribe/unsubscribe

## Type Safety

The SDK provides full TypeScript support with type guards for runtime validation:

```typescript
import { isMarket, isOrder, assertMarket } from '@matchbook/sdk';

// Type guard (returns boolean)
if (isMarket(data)) {
  console.log(data.address); // TypeScript knows data is Market
}

// Assert function (throws on invalid)
const market = assertMarket(data);
```

## Further Reading

- [SDK Documentation](../README.md)
- [API Protocol Specification](../../.internalDoc/05-protocols.md)
- [Domain Model](../../.internalDoc/02-domain-model.md)
