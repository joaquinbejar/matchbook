# M3 Issue #28: Rust SDK: HTTP client for REST API

## Overview

Implement the HTTP client that wraps the REST API endpoints, providing a type-safe interface for fetching market data, order book snapshots, and user data.

## Acceptance Criteria

### MatchbookClient Configuration
- [ ] Base URL
- [ ] Request timeout
- [ ] Retry policy
- [ ] Optional API key authentication

### Market Data Methods
- [ ] `get_markets()` → `Vec<Market>`
- [ ] `get_market(address)` → `Market`
- [ ] `get_orderbook(market, depth)` → `OrderBook`
- [ ] `get_trades(market, limit, before)` → `Vec<Trade>`

### User Data Methods
- [ ] `get_orders(owner, market)` → `Vec<Order>`
- [ ] `get_user_trades(owner, market)` → `Vec<Trade>`
- [ ] `get_balances(owner)` → `Vec<Balance>`

### Requirements
- [ ] Error handling with typed errors
- [ ] Async/await support (tokio)
- [ ] Unit tests
- [ ] Documentation with examples

## Technical Approach

### 1. Project Structure

Add to existing `sdk` crate:

```
sdk/src/
├── client/
│   ├── mod.rs           # Module exports
│   ├── config.rs        # Client configuration
│   ├── http.rs          # HTTP client implementation
│   └── error.rs         # Client-specific errors
```

### 2. Client Configuration

```rust
pub struct ClientConfig {
    pub base_url: String,
    pub timeout: Duration,
    pub max_retries: u32,
    pub api_key: Option<String>,
}
```

### 3. HTTP Client

```rust
pub struct MatchbookClient {
    config: ClientConfig,
    http: reqwest::Client,
}

impl MatchbookClient {
    // Market data
    pub async fn get_markets(&self) -> Result<Vec<Market>, ClientError>;
    pub async fn get_market(&self, address: &str) -> Result<Market, ClientError>;
    pub async fn get_orderbook(&self, market: &str, depth: Option<u32>) -> Result<OrderBook, ClientError>;
    pub async fn get_trades(&self, market: &str, limit: Option<u32>, before: Option<&str>) -> Result<Vec<Trade>, ClientError>;
    
    // User data
    pub async fn get_orders(&self, owner: &str, market: Option<&str>) -> Result<Vec<Order>, ClientError>;
    pub async fn get_user_trades(&self, owner: &str, market: Option<&str>) -> Result<Vec<Trade>, ClientError>;
    pub async fn get_balances(&self, owner: &str) -> Result<Vec<Balance>, ClientError>;
}
```

## Files to Create/Modify

| File | Description |
|------|-------------|
| `sdk/src/client/mod.rs` | Module exports |
| `sdk/src/client/config.rs` | Client configuration |
| `sdk/src/client/http.rs` | HTTP client implementation |
| `sdk/src/client/error.rs` | Client-specific errors |
| `sdk/src/lib.rs` | Add client module |
| `sdk/Cargo.toml` | Add reqwest dependency |

## Notes

- Use reqwest with tokio runtime
- Deserialize responses into existing SDK types
- Handle rate limiting with backoff
- Support custom HTTP client for testing
