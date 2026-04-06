# M2 Issue #22: API: REST endpoints for market data and trading

## Overview

Implement the REST API server that provides market data, order book snapshots, trade history, and transaction building endpoints for clients.

## Acceptance Criteria

### Market Endpoints
- [ ] `GET /v1/markets` — List all markets
- [ ] `GET /v1/markets/{market}` — Market details
- [ ] `GET /v1/markets/{market}/orderbook` — Order book snapshot (configurable depth)
- [ ] `GET /v1/markets/{market}/trades` — Recent trades (pagination)
- [ ] `GET /v1/markets/{market}/candles` — OHLCV data (interval, time range)

### User Endpoints
- [ ] `GET /v1/accounts/{owner}/orders` — User's open orders
- [ ] `GET /v1/accounts/{owner}/trades` — User's trade history
- [ ] `GET /v1/accounts/{owner}/balances` — User's balances per market

### Transaction Building Endpoints
- [ ] `POST /v1/tx/place-order` — Build PlaceOrder transaction
- [ ] `POST /v1/tx/cancel-order` — Build CancelOrder transaction
- [ ] `POST /v1/tx/deposit` — Build Deposit transaction
- [ ] `POST /v1/tx/withdraw` — Build Withdraw transaction

### Other
- [ ] Request validation and error responses
- [ ] Rate limiting per endpoint
- [ ] OpenAPI/Swagger documentation
- [ ] Unit and integration tests

## Technical Approach

### 1. Project Structure

Create a new `api` crate:

```
api/
├── Cargo.toml
├── src/
│   ├── lib.rs           # Crate root
│   ├── server.rs        # Server setup and configuration
│   ├── routes/
│   │   ├── mod.rs       # Route registration
│   │   ├── markets.rs   # Market endpoints
│   │   ├── accounts.rs  # User account endpoints
│   │   └── tx.rs        # Transaction building endpoints
│   ├── handlers/
│   │   ├── mod.rs       # Handler exports
│   │   ├── markets.rs   # Market handlers
│   │   ├── accounts.rs  # Account handlers
│   │   └── tx.rs        # Transaction handlers
│   ├── models/
│   │   ├── mod.rs       # Model exports
│   │   ├── request.rs   # Request types
│   │   └── response.rs  # Response types
│   ├── error.rs         # Error types and responses
│   └── state.rs         # Application state
```

### 2. Dependencies

- `axum` — Web framework
- `tower` — Middleware (rate limiting)
- `tower-http` — HTTP middleware (CORS, tracing)
- `serde` — Serialization
- `utoipa` — OpenAPI documentation

### 3. Core Types

```rust
pub struct AppState {
    pub book_builder: Arc<RwLock<BookBuilder>>,
    pub event_processor: Arc<RwLock<EventProcessor>>,
    // Database connection pool
}

pub struct ApiError {
    pub code: String,
    pub message: String,
    pub status: StatusCode,
}
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `api/Cargo.toml` | Create | API crate dependencies |
| `api/src/lib.rs` | Create | Crate root |
| `api/src/server.rs` | Create | Server setup |
| `api/src/routes/*.rs` | Create | Route definitions |
| `api/src/handlers/*.rs` | Create | Request handlers |
| `api/src/models/*.rs` | Create | Request/response types |
| `api/src/error.rs` | Create | Error handling |
| `api/src/state.rs` | Create | Application state |
| `Cargo.toml` | Modify | Add api to workspace |

## Notes

- Use Axum for web framework
- JSON responses with consistent error format
- Pagination via cursor or offset
- Transaction building returns base64-encoded transaction
- Consider caching for frequently accessed data
