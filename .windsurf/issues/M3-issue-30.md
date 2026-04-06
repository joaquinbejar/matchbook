# M3 Issue #30: TypeScript SDK: Types and definitions

## Overview

Implement TypeScript type definitions that mirror the Rust SDK types, providing type safety for JavaScript/TypeScript clients integrating with Matchbook.

## Acceptance Criteria

### Core Types
- [ ] `Price`, `Quantity` (as string or bigint for precision)
- [ ] `Side`: 'bid' | 'ask'
- [ ] `OrderType`: 'limit' | 'postOnly' | 'ioc' | 'fok'
- [ ] `TimeInForce`: 'gtc' | 'ioc' | 'fok' | 'postOnly'
- [ ] `OrderStatus`: 'open' | 'partiallyFilled' | 'filled' | 'cancelled'

### Entity Types
- [ ] `Market` — Market configuration
- [ ] `Order` — Order details
- [ ] `Trade` — Trade record
- [ ] `BookLevel` — Price level
- [ ] `OrderBook` — Full book
- [ ] `Balance` — User balance

### API Response Types
- [ ] REST endpoint responses
- [ ] WebSocket message types

### Requirements
- [ ] Type guards and validation functions
- [ ] JSDoc documentation
- [ ] Unit tests for type guards

## Technical Approach

### 1. Project Structure

Create new `ts-sdk` directory:

```
ts-sdk/
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts           # Main exports
│   ├── types/
│   │   ├── index.ts       # Type exports
│   │   ├── primitives.ts  # Price, Quantity, Side, etc.
│   │   ├── market.ts      # Market type
│   │   ├── order.ts       # Order, OrderType, OrderStatus
│   │   ├── trade.ts       # Trade type
│   │   ├── book.ts        # BookLevel, OrderBook
│   │   └── balance.ts     # Balance type
│   ├── api/
│   │   ├── index.ts       # API type exports
│   │   ├── rest.ts        # REST response types
│   │   └── websocket.ts   # WebSocket message types
│   └── guards/
│       ├── index.ts       # Guard exports
│       └── validators.ts  # Type guards and validators
└── tests/
    └── guards.test.ts     # Type guard tests
```

### 2. Core Types

```typescript
// Primitives
export type Price = string;  // String for precision
export type Quantity = string;
export type Side = 'bid' | 'ask';
export type OrderType = 'limit' | 'postOnly' | 'ioc' | 'fok';
export type TimeInForce = 'gtc' | 'ioc' | 'fok' | 'postOnly';
export type OrderStatus = 'open' | 'partiallyFilled' | 'filled' | 'cancelled' | 'expired';
```

### 3. Entity Types

```typescript
interface Market {
  address: string;
  baseMint: string;
  quoteMint: string;
  baseSymbol?: string;
  quoteSymbol?: string;
  tickSize: Price;
  lotSize: Quantity;
  makerFee: number;
  takerFee: number;
}

interface Order {
  id: string;
  market: string;
  owner: string;
  side: Side;
  price: Price;
  quantity: Quantity;
  filledQuantity: Quantity;
  orderType: OrderType;
  status: OrderStatus;
  createdAt: string;
}
```

### 4. Type Guards

```typescript
export function isSide(value: unknown): value is Side;
export function isOrderType(value: unknown): value is OrderType;
export function isMarket(value: unknown): value is Market;
export function isOrder(value: unknown): value is Order;
```

## Files to Create

| File | Description |
|------|-------------|
| `ts-sdk/package.json` | Package configuration |
| `ts-sdk/tsconfig.json` | TypeScript configuration |
| `ts-sdk/src/index.ts` | Main exports |
| `ts-sdk/src/types/*.ts` | Type definitions |
| `ts-sdk/src/api/*.ts` | API response types |
| `ts-sdk/src/guards/*.ts` | Type guards |
| `ts-sdk/tests/*.test.ts` | Unit tests |

## Notes

- Use TypeScript strict mode
- Use string for Price/Quantity (JSON-safe, precision)
- Export both ESM and CommonJS
- JSDoc documentation on all exports
