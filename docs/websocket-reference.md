# WebSocket API Reference

This document provides complete documentation for the Matchbook WebSocket API.

## Connection

### Endpoint

| Environment | URL |
|-------------|-----|
| Production | `wss://ws.matchbook.taunais.com/v1/stream` |
| Devnet | `wss://ws.devnet.matchbook.taunais.com/v1/stream` |

### Authentication (Optional)

For user-specific channels (e.g., order updates):

```javascript
const ws = new WebSocket('wss://ws.matchbook.taunais.com/v1/stream?api_key=YOUR_KEY');
```

## Message Format

All messages are JSON with a `type` field:

```typescript
interface Message {
  type: string;
  [key: string]: any;
}
```

---

## Client Messages

### Subscribe

Subscribe to a channel.

```json
{
  "type": "subscribe",
  "channel": "book",
  "market": "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
  "depth": 20
}
```

**Channels:**

| Channel | Description | Auth Required |
|---------|-------------|---------------|
| `book` | Order book updates | No |
| `trades` | Trade stream | No |
| `ticker` | Price ticker | No |
| `orders` | User order updates | Yes |

**Channel-specific parameters:**

| Channel | Parameter | Type | Description |
|---------|-----------|------|-------------|
| `book` | `depth` | integer | Number of levels (default: 20) |
| `orders` | `owner` | string | Wallet address to monitor |

### Unsubscribe

Unsubscribe from a channel.

```json
{
  "type": "unsubscribe",
  "channel": "book",
  "market": "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF"
}
```

### Ping

Send a heartbeat.

```json
{
  "type": "ping",
  "timestamp": 1706640000000
}
```

---

## Server Messages

### Subscribed

Confirmation of subscription.

```json
{
  "type": "subscribed",
  "channel": "book",
  "market": "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF"
}
```

### Unsubscribed

Confirmation of unsubscription.

```json
{
  "type": "unsubscribed",
  "channel": "book",
  "market": "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF"
}
```

### Pong

Response to ping.

```json
{
  "type": "pong",
  "timestamp": 1706640000000
}
```

### Error

Error message.

```json
{
  "type": "error",
  "code": "INVALID_MARKET",
  "message": "Market not found",
  "request_id": "abc123"
}
```

---

## Book Channel

### Book Snapshot

Sent immediately after subscribing to the book channel.

```json
{
  "type": "book_snapshot",
  "market": "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
  "slot": 234567890,
  "sequence": 12345678,
  "bids": [
    ["105.50", "100.5"],
    ["105.40", "250.0"],
    ["105.30", "500.0"]
  ],
  "asks": [
    ["105.60", "75.0"],
    ["105.70", "200.0"],
    ["105.80", "350.0"]
  ]
}
```

**Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `market` | string | Market address |
| `slot` | integer | Solana slot number |
| `sequence` | integer | Sequence number for ordering |
| `bids` | array | Bid levels as [price, quantity] |
| `asks` | array | Ask levels as [price, quantity] |

### Book Update

Incremental updates after the snapshot.

```json
{
  "type": "book_update",
  "market": "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
  "slot": 234567891,
  "sequence": 12345679,
  "bids": [
    ["105.50", "150.0"],
    ["105.45", "50.0"]
  ],
  "asks": [
    ["105.60", "0"]
  ]
}
```

**Notes:**
- Quantity of `"0"` means the price level was removed
- Only changed levels are included
- Apply updates in sequence order

---

## Trades Channel

### Trade

New trade executed.

```json
{
  "type": "trade",
  "market": "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
  "id": "abc123",
  "price": "105.55",
  "quantity": "5.0",
  "side": "buy",
  "timestamp": "2026-01-30T12:00:01.234Z"
}
```

**Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Trade ID |
| `price` | string | Trade price |
| `quantity` | string | Trade quantity |
| `side` | string | Taker side: `buy` or `sell` |
| `timestamp` | string | Trade timestamp (ISO 8601) |

---

## Ticker Channel

### Ticker

Price and volume updates.

```json
{
  "type": "ticker",
  "market": "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
  "best_bid": "105.50",
  "best_ask": "105.60",
  "last_price": "105.55",
  "volume_24h": "1234567.89",
  "price_change_24h": "2.34",
  "high_24h": "108.00",
  "low_24h": "102.00",
  "timestamp": "2026-01-30T12:00:00.000Z"
}
```

**Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `best_bid` | string | Best bid price |
| `best_ask` | string | Best ask price |
| `last_price` | string | Last trade price |
| `volume_24h` | string | 24-hour volume |
| `price_change_24h` | string | 24-hour price change (%) |
| `high_24h` | string | 24-hour high |
| `low_24h` | string | 24-hour low |

---

## Orders Channel (Authenticated)

### Subscribe to Orders

```json
{
  "type": "subscribe",
  "channel": "orders",
  "owner": "ABC123..."
}
```

### Order Update

Updates to user's orders.

```json
{
  "type": "order_update",
  "market": "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
  "order_id": "340282366920938463463374607431768211455",
  "client_order_id": 12345,
  "status": "partial_fill",
  "side": "bid",
  "price": "105.00",
  "original_quantity": "100.0",
  "filled_quantity": "25.0",
  "remaining_quantity": "75.0",
  "average_price": "105.50",
  "timestamp": "2026-01-30T12:00:01.234Z"
}
```

**Order Status Values:**

| Status | Description |
|--------|-------------|
| `open` | Order is active on the book |
| `partial_fill` | Order partially filled |
| `filled` | Order completely filled |
| `cancelled` | Order cancelled by user |
| `expired` | Order expired (IOC/FOK) |

### Fill

Individual fill on user's order.

```json
{
  "type": "fill",
  "market": "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
  "order_id": "340282366920938463463374607431768211455",
  "client_order_id": 12345,
  "trade_id": "xyz789",
  "price": "105.50",
  "quantity": "10.0",
  "role": "maker",
  "fee": "0.10550",
  "fee_token": "USDC",
  "timestamp": "2026-01-30T12:00:01.234Z"
}
```

---

## Connection Management

### Heartbeat

Send a `ping` message every 30 seconds to keep the connection alive:

```javascript
setInterval(() => {
  ws.send(JSON.stringify({ type: 'ping', timestamp: Date.now() }));
}, 30000);
```

The server will disconnect after 60 seconds of inactivity.

### Reconnection

On disconnect, implement exponential backoff:

```javascript
let reconnectDelay = 1000;
const maxDelay = 30000;

function reconnect() {
  setTimeout(() => {
    ws = new WebSocket(url);
    ws.onopen = () => {
      reconnectDelay = 1000; // Reset on success
      resubscribe();
    };
    ws.onclose = () => {
      reconnectDelay = Math.min(reconnectDelay * 2, maxDelay);
      reconnect();
    };
  }, reconnectDelay);
}
```

### Sequence Numbers

Book updates include sequence numbers. If you receive an out-of-order message:

1. Unsubscribe from the channel
2. Resubscribe to get a fresh snapshot

```javascript
let expectedSequence = null;

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  
  if (msg.type === 'book_snapshot') {
    expectedSequence = msg.sequence;
  } else if (msg.type === 'book_update') {
    if (msg.sequence !== expectedSequence + 1) {
      // Sequence gap detected, resubscribe
      ws.send(JSON.stringify({ type: 'unsubscribe', channel: 'book', market }));
      ws.send(JSON.stringify({ type: 'subscribe', channel: 'book', market }));
      return;
    }
    expectedSequence = msg.sequence;
  }
};
```

---

## Example: Order Book Maintenance

```typescript
interface OrderBook {
  bids: Map<string, string>;
  asks: Map<string, string>;
  sequence: number;
}

const orderbooks = new Map<string, OrderBook>();

function handleMessage(msg: any) {
  switch (msg.type) {
    case 'book_snapshot':
      orderbooks.set(msg.market, {
        bids: new Map(msg.bids),
        asks: new Map(msg.asks),
        sequence: msg.sequence,
      });
      break;
      
    case 'book_update':
      const book = orderbooks.get(msg.market);
      if (!book || msg.sequence !== book.sequence + 1) {
        // Resubscribe
        return;
      }
      
      for (const [price, qty] of msg.bids) {
        if (qty === '0') {
          book.bids.delete(price);
        } else {
          book.bids.set(price, qty);
        }
      }
      
      for (const [price, qty] of msg.asks) {
        if (qty === '0') {
          book.asks.delete(price);
        } else {
          book.asks.set(price, qty);
        }
      }
      
      book.sequence = msg.sequence;
      break;
  }
}
```

---

## Error Codes

| Code | Description |
|------|-------------|
| `INVALID_MESSAGE` | Malformed JSON or missing fields |
| `INVALID_CHANNEL` | Unknown channel name |
| `INVALID_MARKET` | Market not found |
| `UNAUTHORIZED` | Authentication required |
| `SUBSCRIPTION_LIMIT` | Too many subscriptions |
| `RATE_LIMIT` | Too many messages |

---

## Related Documentation

- [API Reference](./api-reference.md) - REST API documentation
- [SDK Guide](./sdk-guide.md) - SDK usage examples
- [Getting Started](./getting-started.md) - Integration guide
