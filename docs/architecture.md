# Architecture

This document describes the system architecture of Matchbook, a high-performance Central Limit Order Book (CLOB) on Solana.

## Overview

Matchbook consists of three main layers:

1. **On-chain Program**: Solana smart contract managing order books and matching
2. **Off-chain Services**: Indexer, API server, and crank for data access and automation
3. **Client SDKs**: Rust and TypeScript libraries for integration

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                 Clients                                      │
│              (Web Apps, Mobile Apps, Trading Bots, Market Makers)            │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
          ┌───────────────────────────┼───────────────────────────┐
          │                           │                           │
          ▼                           ▼                           ▼
   ┌─────────────┐            ┌─────────────┐            ┌─────────────┐
   │  REST API   │            │  WebSocket  │            │   Direct    │
   │   Server    │            │   Server    │            │  On-chain   │
   │   :8080     │            │   :8081     │            │   Access    │
   └──────┬──────┘            └──────┬──────┘            └──────┬──────┘
          │                          │                          │
          └──────────────────────────┼──────────────────────────┘
                                     │
                    ┌────────────────┴────────────────┐
                    │                                 │
                    ▼                                 ▼
          ┌─────────────────┐               ┌─────────────────┐
          │     Indexer     │               │  Solana Program │
          │    (Geyser)     │◄──────────────│   (On-chain)    │
          └────────┬────────┘               └────────┬────────┘
                   │                                 │
                   ▼                                 │
          ┌─────────────────┐                        │
          │    Database     │                        │
          │  (TimescaleDB)  │                        │
          └─────────────────┘                        │
                                                     │
          ┌─────────────────┐                        │
          │  Crank Service  │────────────────────────┘
          │  (Matching)     │
          └─────────────────┘
```

## Components

### On-chain Program

The Solana program is the trust layer that:

- Manages market state and order books
- Validates and executes order placement
- Performs order matching
- Handles fund custody in vaults
- Emits events for off-chain indexing

**Key Accounts:**

| Account | Description |
|---------|-------------|
| `Market` | Market configuration and state |
| `OrderBook` | Bids and asks stored in a red-black tree |
| `EventQueue` | Queue of matching events to be processed |
| `OpenOrders` | Per-user account tracking orders and balances |
| `Vault` | Token accounts holding deposited funds |

**Instructions:**

| Instruction | Description |
|-------------|-------------|
| `CreateMarket` | Initialize a new trading pair |
| `CreateOpenOrders` | Create user's trading account |
| `Deposit` | Deposit tokens for trading |
| `PlaceOrder` | Submit a new order |
| `CancelOrder` | Cancel an existing order |
| `MatchOrders` | Execute matching (called by crank) |
| `ConsumeEvents` | Process event queue |
| `Withdraw` | Withdraw available funds |

### Indexer Service

The indexer connects to Solana via Geyser plugin to:

- Stream account updates in real-time
- Parse program events and state changes
- Store historical data in TimescaleDB
- Provide data to API server

**Data Flow:**

```
Solana Validator
       │
       │ Geyser gRPC
       ▼
┌─────────────┐
│   Indexer   │
│             │
│ ┌─────────┐ │
│ │ Parser  │ │
│ └────┬────┘ │
│      │      │
│ ┌────▼────┐ │
│ │ Writer  │ │
│ └────┬────┘ │
└──────┼──────┘
       │
       ▼
┌─────────────┐
│  Database   │
│             │
│ - Markets   │
│ - Orders    │
│ - Trades    │
│ - Events    │
└─────────────┘
```

### API Server

The API server provides:

- **REST API**: Queries, snapshots, transaction building
- **WebSocket API**: Real-time streaming updates

**REST Endpoints:**

- `GET /v1/markets` - List markets
- `GET /v1/markets/{address}/orderbook` - Order book snapshot
- `GET /v1/markets/{address}/trades` - Recent trades
- `POST /v1/tx/place-order` - Build order transaction
- `POST /v1/tx/cancel-order` - Build cancel transaction

**WebSocket Channels:**

- `book` - Order book updates
- `trades` - Trade stream
- `ticker` - Price ticker
- `orders` - User order updates (authenticated)

### Crank Service

The crank automates order matching:

1. Monitors markets for matchable orders
2. Builds and submits `MatchOrders` transactions
3. Consumes events from the event queue
4. Earns rewards for successful matches

**Crank Flow:**

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Monitor   │────▶│   Match     │────▶│   Consume   │
│   Markets   │     │   Orders    │     │   Events    │
└─────────────┘     └─────────────┘     └─────────────┘
       │                   │                   │
       │                   │                   │
       ▼                   ▼                   ▼
  Check for           Submit TX           Process
  crossed             to Solana           event queue
  orders
```

## Data Flow

### Order Placement

```
1. Client builds order parameters
2. Client calls POST /v1/tx/place-order
3. API returns unsigned transaction
4. Client signs with wallet
5. Client submits to Solana
6. Program validates and places order
7. Indexer detects account change
8. Database updated
9. WebSocket broadcasts update
```

### Order Matching

```
1. Crank detects crossed orders (bid >= ask)
2. Crank builds MatchOrders transaction
3. Transaction submitted to Solana
4. Program executes matching:
   - Updates order quantities
   - Transfers funds between vaults
   - Emits Fill events to queue
5. Crank calls ConsumeEvents
6. Events processed, balances updated
7. Indexer records trades
8. WebSocket broadcasts trade
```

## Order Lifecycle

```
                    ┌─────────┐
                    │  New    │
                    └────┬────┘
                         │
                    PlaceOrder
                         │
                    ┌────▼────┐
              ┌─────│  Open   │─────┐
              │     └────┬────┘     │
              │          │          │
         CancelOrder   Match    Expire
              │          │          │
              ▼          ▼          ▼
        ┌─────────┐ ┌─────────┐ ┌─────────┐
        │Cancelled│ │ Filled  │ │ Expired │
        └─────────┘ └─────────┘ └─────────┘
                         │
                    ┌────┴────┐
                    │         │
                    ▼         ▼
              ┌─────────┐ ┌─────────┐
              │ Partial │ │  Full   │
              │  Fill   │ │  Fill   │
              └─────────┘ └─────────┘
```

## Security Model

### Non-Custodial Design

- Users deposit to market vaults, not platform wallets
- Withdrawals only to original depositor
- No admin keys can access user funds

### On-chain Validation

- All order parameters validated on-chain
- Matching logic executed on-chain
- Events provide audit trail

### Access Control

| Operation | Authorization |
|-----------|---------------|
| Create Market | Market authority |
| Place Order | Order owner signature |
| Cancel Order | Order owner signature |
| Match Orders | Anyone (crank) |
| Withdraw | Account owner signature |

## Scalability

### Horizontal Scaling

- **API Server**: Stateless, scale with load balancer
- **Indexer**: Single writer, multiple readers
- **Crank**: Multiple instances per market

### Performance Optimizations

- **Order Book**: Red-black tree for O(log n) operations
- **Event Queue**: Circular buffer for efficient processing
- **Database**: TimescaleDB for time-series data
- **Caching**: Redis for hot data

## Deployment Architecture

### Production Setup

```
┌─────────────────────────────────────────────────────────────────┐
│                        Kubernetes Cluster                        │
│                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │
│  │   Indexer    │  │  API Server  │  │    Crank     │           │
│  │   (1 pod)    │  │  (2-10 pods) │  │   (1 pod)    │           │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘           │
│         │                 │                 │                    │
│         └─────────────────┼─────────────────┘                    │
│                           │                                      │
│  ┌────────────────────────┼────────────────────────┐            │
│  │                        │                        │            │
│  │  ┌──────────────┐  ┌───┴────────┐  ┌──────────┐│            │
│  │  │  PostgreSQL  │  │   Redis    │  │Prometheus││            │
│  │  │ (TimescaleDB)│  │  (Cache)   │  │ Grafana  ││            │
│  │  └──────────────┘  └────────────┘  └──────────┘│            │
│  │                    Data Layer                   │            │
│  └─────────────────────────────────────────────────┘            │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ Geyser gRPC
                              ▼
                    ┌──────────────────┐
                    │  Solana Cluster  │
                    │    (Mainnet)     │
                    └──────────────────┘
```

## Related Documentation

- [Getting Started](./getting-started.md) - Integration guide
- [API Reference](./api-reference.md) - REST API documentation
- [WebSocket Reference](./websocket-reference.md) - WebSocket API documentation
- [Deployment](./docker.md) - Docker and Kubernetes deployment
