# REST API Reference

This document provides complete documentation for the Matchbook REST API.

## Base URL

| Environment | URL |
|-------------|-----|
| Production | `https://api.matchbook.taunais.com/v1` |
| Devnet | `https://api.devnet.matchbook.taunais.com/v1` |

## Authentication

### API Key (Optional)

For higher rate limits, include your API key:

```http
GET /v1/markets HTTP/1.1
Host: api.matchbook.taunais.com
X-API-Key: your-api-key
```

### Signed Requests (Trading)

For transaction building endpoints, sign requests with your wallet:

```http
POST /v1/tx/place-order HTTP/1.1
Host: api.matchbook.taunais.com
Content-Type: application/json
X-Wallet: <base58-pubkey>
X-Timestamp: 1706640000
X-Signature: <base58-signature>
```

Signature covers: `{method}:{path}:{timestamp}:{body_hash}`

## Common Headers

| Header | Description |
|--------|-------------|
| `X-API-Key` | API key for authentication |
| `X-Request-Id` | Client-provided request ID for tracing |
| `X-RateLimit-Limit` | Requests allowed per window (response) |
| `X-RateLimit-Remaining` | Requests remaining (response) |

## Rate Limits

| Tier | Requests/min | WebSocket Connections |
|------|--------------|----------------------|
| Free | 60 | 2 |
| Basic | 600 | 10 |
| Pro | 6000 | 50 |

## Error Responses

All errors follow this format:

```json
{
  "error": {
    "code": "INVALID_PARAMETER",
    "message": "Price must be positive",
    "details": {
      "field": "price",
      "value": -100
    }
  }
}
```

### Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `INVALID_PARAMETER` | 400 | Invalid request parameter |
| `MISSING_PARAMETER` | 400 | Required parameter missing |
| `INVALID_MARKET` | 404 | Market not found |
| `INVALID_ORDER` | 404 | Order not found |
| `UNAUTHORIZED` | 401 | Missing or invalid authentication |
| `FORBIDDEN` | 403 | Insufficient permissions |
| `RATE_LIMIT_EXCEEDED` | 429 | Too many requests |
| `INTERNAL_ERROR` | 500 | Server error |

---

## Markets

### List Markets

Get all available markets.

```http
GET /v1/markets
```

**Response:**

```json
{
  "markets": [
    {
      "address": "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
      "base_mint": "So11111111111111111111111111111111111111112",
      "quote_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
      "base_symbol": "SOL",
      "quote_symbol": "USDC",
      "base_decimals": 9,
      "quote_decimals": 6,
      "tick_size": "0.01",
      "lot_size": "0.001",
      "min_order_size": "0.01",
      "taker_fee_bps": 30,
      "maker_fee_bps": -10,
      "state": "active",
      "volume_24h": "1234567.89",
      "price_change_24h": "2.34"
    }
  ]
}
```

### Get Market

Get details for a specific market.

```http
GET /v1/markets/{address}
```

**Parameters:**

| Name | In | Type | Description |
|------|-----|------|-------------|
| `address` | path | string | Market address |

**Response:**

```json
{
  "market": {
    "address": "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
    "base_mint": "So11111111111111111111111111111111111111112",
    "quote_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    "bids_address": "...",
    "asks_address": "...",
    "event_queue": "...",
    "base_vault": "...",
    "quote_vault": "...",
    "tick_size": "0.01",
    "lot_size": "0.001",
    "min_order_size": "0.01",
    "taker_fee_bps": 30,
    "maker_fee_bps": -10,
    "state": "active"
  },
  "stats": {
    "best_bid": "105.50",
    "best_ask": "105.60",
    "last_price": "105.55",
    "volume_24h": "1234567.89",
    "high_24h": "108.00",
    "low_24h": "102.00",
    "price_change_24h": "2.34",
    "trade_count_24h": 45678
  }
}
```

---

## Order Book

### Get Order Book (L2)

Get aggregated order book snapshot.

```http
GET /v1/markets/{address}/orderbook
```

**Parameters:**

| Name | In | Type | Default | Description |
|------|-----|------|---------|-------------|
| `address` | path | string | - | Market address |
| `depth` | query | integer | 20 | Number of price levels (max: 500) |

**Response:**

```json
{
  "market": "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
  "slot": 234567890,
  "timestamp": "2026-01-30T12:00:00.000Z",
  "sequence": 12345678,
  "bids": [
    { "price": "105.50", "quantity": "100.5", "orders": 5 },
    { "price": "105.40", "quantity": "250.0", "orders": 12 }
  ],
  "asks": [
    { "price": "105.60", "quantity": "75.0", "orders": 3 },
    { "price": "105.70", "quantity": "200.0", "orders": 7 }
  ]
}
```

### Get Order Book (L3)

Get full order book with individual orders.

```http
GET /v1/markets/{address}/orderbook/l3
```

**Response:**

```json
{
  "market": "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
  "slot": 234567890,
  "bids": [
    {
      "order_id": "340282366920938463463374607431768211455",
      "price": "105.50",
      "quantity": "10.5",
      "owner": "ABC123...",
      "timestamp": "2026-01-30T11:58:30.000Z"
    }
  ],
  "asks": [
    {
      "order_id": "340282366920938463463374607431768211456",
      "price": "105.60",
      "quantity": "25.0",
      "owner": "DEF456...",
      "timestamp": "2026-01-30T11:59:00.000Z"
    }
  ]
}
```

---

## Trades

### Get Recent Trades

Get recent trades for a market.

```http
GET /v1/markets/{address}/trades
```

**Parameters:**

| Name | In | Type | Default | Description |
|------|-----|------|---------|-------------|
| `address` | path | string | - | Market address |
| `limit` | query | integer | 100 | Number of trades (max: 1000) |
| `before` | query | string | - | Cursor for pagination |
| `after` | query | string | - | Cursor for pagination |

**Response:**

```json
{
  "trades": [
    {
      "id": "abc123",
      "price": "105.55",
      "quantity": "5.0",
      "side": "buy",
      "timestamp": "2026-01-30T12:00:01.234Z",
      "maker_order_id": "...",
      "taker_order_id": "..."
    }
  ],
  "pagination": {
    "has_more": true,
    "next_cursor": "eyJ0cyI6MTcwNjY0MDAwMH0"
  }
}
```

### Get Candles (OHLCV)

Get candlestick data.

```http
GET /v1/markets/{address}/candles
```

**Parameters:**

| Name | In | Type | Default | Description |
|------|-----|------|---------|-------------|
| `address` | path | string | - | Market address |
| `interval` | query | string | - | `1m`, `5m`, `15m`, `1h`, `4h`, `1d` |
| `start_time` | query | string | - | Start timestamp (ISO 8601) |
| `end_time` | query | string | - | End timestamp (ISO 8601) |
| `limit` | query | integer | 100 | Number of candles (max: 1500) |

**Response:**

```json
{
  "candles": [
    {
      "timestamp": "2026-01-30T12:00:00.000Z",
      "open": "105.50",
      "high": "105.80",
      "low": "105.40",
      "close": "105.60",
      "volume": "12345.67",
      "trade_count": 234
    }
  ]
}
```

---

## User Accounts

### Get User Orders

Get user's open orders.

```http
GET /v1/accounts/{owner}/orders
```

**Parameters:**

| Name | In | Type | Description |
|------|-----|------|-------------|
| `owner` | path | string | Wallet address |
| `market` | query | string | Filter by market (optional) |

**Response:**

```json
{
  "orders": [
    {
      "order_id": "340282366920938463463374607431768211455",
      "market": "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
      "side": "bid",
      "price": "105.00",
      "original_quantity": "100.0",
      "remaining_quantity": "75.0",
      "filled_quantity": "25.0",
      "order_type": "limit",
      "status": "open",
      "client_order_id": 12345,
      "placed_at": "2026-01-30T11:00:00.000Z"
    }
  ]
}
```

### Get User Trades

Get user's trade history.

```http
GET /v1/accounts/{owner}/trades
```

**Parameters:**

| Name | In | Type | Description |
|------|-----|------|-------------|
| `owner` | path | string | Wallet address |
| `market` | query | string | Filter by market (optional) |
| `start_time` | query | string | Start timestamp (optional) |
| `end_time` | query | string | End timestamp (optional) |
| `limit` | query | integer | Number of trades (default: 100) |

**Response:**

```json
{
  "trades": [
    {
      "id": "xyz789",
      "market": "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
      "order_id": "...",
      "side": "buy",
      "role": "taker",
      "price": "105.55",
      "quantity": "10.0",
      "fee": "0.31665",
      "fee_token": "USDC",
      "timestamp": "2026-01-30T11:30:00.000Z"
    }
  ]
}
```

### Get User Balances

Get user's balances in markets.

```http
GET /v1/accounts/{owner}/balances
```

**Response:**

```json
{
  "balances": [
    {
      "market": "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
      "open_orders_account": "...",
      "base": {
        "locked": "50.0",
        "free": "10.0",
        "total": "60.0"
      },
      "quote": {
        "locked": "5000.0",
        "free": "500.0",
        "total": "5500.0"
      }
    }
  ]
}
```

---

## Transaction Building

### Place Order

Build a place order transaction.

```http
POST /v1/tx/place-order
```

**Request:**

```json
{
  "market": "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
  "owner": "ABC123...",
  "side": "bid",
  "price": "105.00",
  "quantity": "10.0",
  "order_type": "limit",
  "time_in_force": "gtc",
  "client_order_id": 12345,
  "self_trade_behavior": "cancel_taker"
}
```

**Request Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `market` | string | Yes | Market address |
| `owner` | string | Yes | Wallet address |
| `side` | string | Yes | `bid` or `ask` |
| `price` | string | Yes | Order price |
| `quantity` | string | Yes | Order quantity |
| `order_type` | string | Yes | `limit`, `market`, `post_only` |
| `time_in_force` | string | No | `gtc`, `ioc`, `fok` (default: `gtc`) |
| `client_order_id` | integer | No | Your order ID |
| `self_trade_behavior` | string | No | `cancel_taker`, `cancel_maker`, `abort` |

**Response:**

```json
{
  "transaction": "base64-encoded-transaction",
  "message": "base64-encoded-message",
  "signers": ["ABC123..."],
  "estimated_fee": 5000,
  "accounts_to_create": [],
  "warnings": []
}
```

### Cancel Order

Build a cancel order transaction.

```http
POST /v1/tx/cancel-order
```

**Request:**

```json
{
  "market": "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
  "owner": "ABC123...",
  "order_id": "340282366920938463463374607431768211455",
  "side": "bid"
}
```

### Cancel All Orders

Build a cancel all orders transaction.

```http
POST /v1/tx/cancel-all
```

**Request:**

```json
{
  "market": "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
  "owner": "ABC123...",
  "side": "bid"
}
```

### Settle Funds

Build a settle funds transaction.

```http
POST /v1/tx/settle
```

**Request:**

```json
{
  "market": "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF",
  "owner": "ABC123..."
}
```

---

## Data Types

### Numeric Precision

All prices and quantities are strings to preserve precision:

```json
{
  "price": "105.50000000",
  "quantity": "10.123456789"
}
```

### Timestamps

ISO 8601 format with milliseconds:

```json
{
  "timestamp": "2026-01-30T12:00:00.123Z"
}
```

### Addresses

Base58-encoded Solana public keys:

```json
{
  "market": "7dLVkUfBVfCGkFhSXDCq1ukM9usathSgS716t643iFGF"
}
```

### Order IDs

128-bit integers as decimal strings:

```json
{
  "order_id": "340282366920938463463374607431768211455"
}
```

---

## Related Documentation

- [WebSocket Reference](./websocket-reference.md) - Real-time streaming API
- [SDK Guide](./sdk-guide.md) - SDK usage examples
- [Getting Started](./getting-started.md) - Integration guide
