# @matchbook/sdk

TypeScript SDK for the Matchbook Central Limit Order Book (CLOB) on Solana.

## Installation

```bash
npm install @matchbook/sdk
```

## Quick Start

### HTTP Client

```typescript
import { MatchbookClient } from '@matchbook/sdk';

const client = new MatchbookClient({
  baseUrl: 'https://api.matchbook.taunais.com/v1',
  apiKey: 'your-api-key', // Optional
});

// Get all markets
const markets = await client.getMarkets();

// Get order book
const orderbook = await client.getOrderbook('ABC123...', 20);

// Get user orders
const orders = await client.getOrders('9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM');

// Build a place order transaction
const tx = await client.buildPlaceOrderTx({
  market: 'ABC123...',
  side: 'bid',
  price: '105.50',
  quantity: '100',
  orderType: 'limit',
});
```

### WebSocket Client

```typescript
import { MatchbookWsClient } from '@matchbook/sdk';

const ws = new MatchbookWsClient({
  wsUrl: 'wss://ws.matchbook.taunais.com/v1/stream',
  apiKey: 'your-api-key', // Required for order updates
});

await ws.connect();

// Subscribe to order book updates
const bookSubId = ws.subscribeBook('ABC123...', (data) => {
  if (data.type === 'book_snapshot') {
    console.log('Full book:', data.bids, data.asks);
  } else {
    console.log('Book update:', data.bids, data.asks);
  }
});

// Subscribe to trades
const tradeSubId = ws.subscribeTrades('ABC123...', (trade) => {
  console.log('Trade:', trade.price, trade.quantity, trade.side);
});

// Subscribe to your order updates (requires authentication)
const orderSubId = ws.subscribeOrders((order) => {
  console.log('Order update:', order.orderId, order.status);
});

// Unsubscribe
ws.unsubscribe(bookSubId);

// Disconnect
ws.disconnect();
```

## API Reference

### Types

#### Core Types

```typescript
type Price = string;      // Price as string for precision
type Quantity = string;   // Quantity as string for precision
type Side = 'bid' | 'ask';
type OrderType = 'limit' | 'postOnly' | 'ioc' | 'fok';
type TimeInForce = 'gtc' | 'ioc' | 'fok' | 'postOnly';
type OrderStatus = 'open' | 'partiallyFilled' | 'filled' | 'cancelled' | 'expired';
```

#### Entity Types

- `Market` - Market configuration and state
- `Order` - Order details
- `Trade` - Trade record
- `BookLevel` - Aggregated price level
- `OrderBook` - Full order book with bids/asks
- `Balance` - User balance

### HTTP Client Methods

#### Market Data

- `getMarkets()` - Get all markets
- `getMarket(address)` - Get market by address
- `getOrderbook(market, depth?)` - Get order book
- `getTrades(market, options?)` - Get recent trades

#### User Data

- `getOrders(owner, market?)` - Get user orders
- `getUserTrades(owner, market?)` - Get user trades
- `getBalances(owner)` - Get user balances

#### Transaction Building

- `buildPlaceOrderTx(params)` - Build place order transaction
- `buildCancelOrderTx(params)` - Build cancel order transaction
- `buildDepositTx(params)` - Build deposit transaction
- `buildWithdrawTx(params)` - Build withdraw transaction

### WebSocket Client Methods

- `connect()` - Connect to WebSocket server
- `disconnect()` - Disconnect from server
- `subscribeBook(market, callback, depth?)` - Subscribe to order book
- `subscribeTrades(market, callback)` - Subscribe to trades
- `subscribeOrders(callback)` - Subscribe to order updates
- `unsubscribe(subscriptionId)` - Unsubscribe from channel

### Type Guards

Runtime validation functions for API responses:

```typescript
import { isMarket, isOrder, isTrade, assertMarket } from '@matchbook/sdk';

// Type guards return boolean
if (isMarket(data)) {
  console.log(data.address); // TypeScript knows data is Market
}

// Assert functions throw on invalid data
const market = assertMarket(data); // Throws if invalid
```

### Error Handling

```typescript
import {
  ClientError,
  HttpError,
  ApiError,
  TimeoutError,
  RateLimitError,
  NotFoundError,
  UnauthorizedError,
  isClientError,
  isRateLimitError,
} from '@matchbook/sdk';

try {
  const market = await client.getMarket('invalid');
} catch (error) {
  if (isRateLimitError(error)) {
    console.log('Rate limited, retry after:', error.retryAfter);
  } else if (error instanceof NotFoundError) {
    console.log('Market not found');
  } else if (isClientError(error)) {
    console.log('Client error:', error.message);
  }
}
```

## Configuration

### HTTP Client

```typescript
interface ClientConfig {
  baseUrl?: string;    // REST API base URL
  wsUrl?: string;      // WebSocket URL
  timeout?: number;    // Request timeout (ms)
  apiKey?: string;     // API key for authentication
  headers?: Record<string, string>; // Custom headers
}
```

### WebSocket Client

The WebSocket client uses the same configuration. It automatically:

- Sends heartbeat pings every 30 seconds
- Reconnects with exponential backoff on disconnect
- Resubscribes to all channels after reconnection

## License

MIT
