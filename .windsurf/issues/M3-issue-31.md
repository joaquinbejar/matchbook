# M3 Issue #31: TypeScript SDK: HTTP and WebSocket client implementation

## Overview

Implement the HTTP and WebSocket client for the TypeScript SDK, providing a complete client library for JavaScript/TypeScript applications to interact with Matchbook.

## Acceptance Criteria

### MatchbookClient Configuration
- [ ] REST API base URL
- [ ] WebSocket URL
- [ ] Request timeout
- [ ] Optional authentication

### REST Methods
- [ ] `getMarkets()`, `getMarket(address)`
- [ ] `getOrderbook(market, depth)`
- [ ] `getTrades(market, options)`
- [ ] `getOrders(owner, market)`
- [ ] `getUserTrades(owner, market)`
- [ ] `getBalances(owner)`

### Transaction Building Methods
- [ ] `buildPlaceOrderTx(...)`
- [ ] `buildCancelOrderTx(...)`
- [ ] `buildDepositTx(...)`
- [ ] `buildWithdrawTx(...)`

### WebSocket Methods
- [ ] `subscribeBook(market, callback)`
- [ ] `subscribeTrades(market, callback)`
- [ ] `subscribeOrders(owner, callback)`
- [ ] `unsubscribe(subscriptionId)`

### Requirements
- [ ] Connection management and reconnection
- [ ] Error handling with typed errors
- [ ] Unit tests with mocked APIs
- [ ] Documentation and README

## Technical Approach

### 1. Project Structure

Extend existing `ts-sdk` with client implementation:

```
ts-sdk/src/
├── client/
│   ├── index.ts           # Client exports
│   ├── config.ts          # Client configuration
│   ├── http.ts            # HTTP client implementation
│   ├── websocket.ts       # WebSocket client implementation
│   └── errors.ts          # Client-specific errors
```

### 2. Client Configuration

```typescript
interface ClientConfig {
  baseUrl: string;
  wsUrl: string;
  timeout?: number;
  apiKey?: string;
}
```

### 3. HTTP Client

```typescript
class MatchbookClient {
  constructor(config: ClientConfig);
  
  // Market data
  getMarkets(): Promise<Market[]>;
  getMarket(address: string): Promise<Market>;
  getOrderbook(market: string, depth?: number): Promise<OrderBook>;
  getTrades(market: string, options?: TradeFilter): Promise<Trade[]>;
  
  // User data
  getOrders(owner: string, market?: string): Promise<Order[]>;
  getUserTrades(owner: string, market?: string): Promise<Trade[]>;
  getBalances(owner: string): Promise<Balance[]>;
  
  // Transaction building
  buildPlaceOrderTx(params: PlaceOrderParams): Promise<BuildTransactionResponse>;
  buildCancelOrderTx(params: CancelOrderParams): Promise<BuildTransactionResponse>;
  buildDepositTx(params: DepositParams): Promise<BuildTransactionResponse>;
  buildWithdrawTx(params: WithdrawParams): Promise<BuildTransactionResponse>;
}
```

### 4. WebSocket Client

```typescript
class MatchbookWsClient {
  constructor(config: ClientConfig);
  
  connect(): Promise<void>;
  disconnect(): void;
  
  subscribeBook(market: string, callback: (data: OrderBookUpdate) => void): string;
  subscribeTrades(market: string, callback: (data: Trade) => void): string;
  subscribeOrders(callback: (data: Order) => void): string;
  unsubscribe(subscriptionId: string): void;
}
```

## Files to Create/Modify

| File | Description |
|------|-------------|
| `ts-sdk/src/client/config.ts` | Client configuration |
| `ts-sdk/src/client/errors.ts` | Client-specific errors |
| `ts-sdk/src/client/http.ts` | HTTP client |
| `ts-sdk/src/client/websocket.ts` | WebSocket client |
| `ts-sdk/src/client/index.ts` | Client exports |
| `ts-sdk/src/index.ts` | Add client exports |
| `ts-sdk/tests/client.test.ts` | Client tests |
| `ts-sdk/README.md` | Documentation |

## Notes

- Use native fetch for HTTP (works in Node.js 18+ and browsers)
- Use ws package for WebSocket (Node.js) with browser fallback
- Support both environments
