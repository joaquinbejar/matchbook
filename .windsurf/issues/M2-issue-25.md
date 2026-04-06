# M2 Issue #25: Redis integration: Caching and pub/sub

## Overview

Implement Redis integration for caching frequently accessed data and pub/sub messaging between services. This improves API response times and enables real-time communication.

## Acceptance Criteria

### Redis Connection
- [ ] Redis connection pool management
- [ ] Configuration for Redis connection and TTLs
- [ ] Graceful degradation if Redis unavailable

### Caching Layer
- [ ] Market metadata (TTL: 1 minute)
- [ ] Order book snapshots (TTL: 1 second)
- [ ] Recent trades (TTL: 10 seconds)
- [ ] User balances (TTL: 5 seconds)
- [ ] Cache invalidation on updates

### Pub/Sub Channels
- [ ] `book:{market}` — Book update notifications
- [ ] `trades:{market}` — Trade notifications
- [ ] `orders:{owner}` — User order updates

### Metrics
- [ ] Hit rate
- [ ] Latency
- [ ] Memory usage

### Testing
- [ ] Unit tests for caching logic

## Technical Approach

### 1. Project Structure

Add a new `cache` module to the `indexer` crate (since it's shared infrastructure):

```
indexer/src/
├── cache/
│   ├── mod.rs           # Module exports
│   ├── config.rs        # Redis configuration
│   ├── pool.rs          # Connection pool management
│   ├── cache.rs         # Cache operations
│   ├── pubsub.rs        # Pub/sub operations
│   └── metrics.rs       # Cache metrics
```

### 2. Core Types

```rust
pub struct RedisConfig {
    pub url: String,
    pub pool_size: u32,
    pub market_ttl_secs: u64,
    pub orderbook_ttl_secs: u64,
    pub trades_ttl_secs: u64,
    pub balances_ttl_secs: u64,
}

pub struct RedisCache {
    pool: Pool<RedisConnectionManager>,
    config: RedisConfig,
    metrics: Arc<CacheMetrics>,
}

pub struct CacheMetrics {
    hits: AtomicU64,
    misses: AtomicU64,
    errors: AtomicU64,
}
```

### 3. Cache Operations

```rust
impl RedisCache {
    pub async fn get_market(&self, market: &[u8; 32]) -> Option<MarketInfo>;
    pub async fn set_market(&self, market: &[u8; 32], info: &MarketInfo);
    pub async fn get_orderbook(&self, market: &[u8; 32]) -> Option<BookSnapshot>;
    pub async fn set_orderbook(&self, market: &[u8; 32], snapshot: &BookSnapshot);
    pub async fn invalidate_market(&self, market: &[u8; 32]);
}
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `indexer/src/cache/mod.rs` | Create | Module exports |
| `indexer/src/cache/config.rs` | Create | Redis configuration |
| `indexer/src/cache/pool.rs` | Create | Connection pool |
| `indexer/src/cache/cache.rs` | Create | Cache operations |
| `indexer/src/cache/pubsub.rs` | Create | Pub/sub operations |
| `indexer/src/cache/metrics.rs` | Create | Cache metrics |
| `indexer/src/lib.rs` | Modify | Add cache module |
| `indexer/Cargo.toml` | Modify | Add redis dependency |
| `Cargo.toml` | Modify | Add redis to workspace deps |

## Notes

- Use redis-rs with connection pooling (deadpool-redis)
- Pub/sub for WebSocket server to receive updates
- Serialize cached data as JSON
- Cache-aside pattern for reads
- Graceful degradation if Redis unavailable
