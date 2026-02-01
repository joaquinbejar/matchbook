//! Redis caching and pub/sub module.
//!
//! Provides Redis integration for caching frequently accessed data
//! and pub/sub messaging between services.
//!
//! # Caching
//!
//! - Market metadata (TTL: 1 minute)
//! - Order book snapshots (TTL: 1 second)
//! - Recent trades (TTL: 10 seconds)
//! - User balances (TTL: 5 seconds)
//!
//! # Pub/Sub Channels
//!
//! - `book:{market}` — Book update notifications
//! - `trades:{market}` — Trade notifications
//! - `orders:{owner}` — User order updates

pub mod config;
pub mod metrics;
pub mod pubsub;
pub mod store;

pub use config::RedisConfig;
pub use metrics::CacheMetrics;
pub use pubsub::RedisPubSub;
pub use store::RedisCache;
