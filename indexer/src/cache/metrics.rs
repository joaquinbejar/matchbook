//! Cache metrics tracking.
//!
//! Provides atomic counters for monitoring cache operations.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Metrics for the cache.
#[derive(Debug)]
pub struct CacheMetrics {
    /// Cache hits.
    hits: AtomicU64,

    /// Cache misses.
    misses: AtomicU64,

    /// Cache errors.
    errors: AtomicU64,

    /// Total get operations.
    gets: AtomicU64,

    /// Total set operations.
    sets: AtomicU64,

    /// Total delete operations.
    deletes: AtomicU64,

    /// Total publish operations.
    publishes: AtomicU64,

    /// Start time for rate calculation.
    start_time: Instant,
}

impl Default for CacheMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheMetrics {
    /// Creates a new metrics instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            gets: AtomicU64::new(0),
            sets: AtomicU64::new(0),
            deletes: AtomicU64::new(0),
            publishes: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    /// Records a cache hit.
    pub fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
        self.gets.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a cache miss.
    pub fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
        self.gets.fetch_add(1, Ordering::Relaxed);
    }

    /// Records an error.
    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a set operation.
    pub fn record_set(&self) {
        self.sets.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a delete operation.
    pub fn record_delete(&self) {
        self.deletes.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a publish operation.
    pub fn record_publish(&self) {
        self.publishes.fetch_add(1, Ordering::Relaxed);
    }

    /// Returns the number of hits.
    #[must_use]
    pub fn hits(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    /// Returns the number of misses.
    #[must_use]
    pub fn misses(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    /// Returns the number of errors.
    #[must_use]
    pub fn errors(&self) -> u64 {
        self.errors.load(Ordering::Relaxed)
    }

    /// Returns the number of get operations.
    #[must_use]
    pub fn gets(&self) -> u64 {
        self.gets.load(Ordering::Relaxed)
    }

    /// Returns the number of set operations.
    #[must_use]
    pub fn sets(&self) -> u64 {
        self.sets.load(Ordering::Relaxed)
    }

    /// Returns the number of delete operations.
    #[must_use]
    pub fn deletes(&self) -> u64 {
        self.deletes.load(Ordering::Relaxed)
    }

    /// Returns the number of publish operations.
    #[must_use]
    pub fn publishes(&self) -> u64 {
        self.publishes.load(Ordering::Relaxed)
    }

    /// Returns the uptime.
    #[must_use]
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Returns the hit rate (0.0 to 1.0).
    #[must_use]
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits() + self.misses();
        if total > 0 {
            self.hits() as f64 / total as f64
        } else {
            0.0
        }
    }

    /// Returns operations per second.
    #[must_use]
    pub fn ops_per_second(&self) -> f64 {
        let elapsed = self.uptime().as_secs_f64();
        if elapsed > 0.0 {
            let total = self.gets() + self.sets() + self.deletes();
            total as f64 / elapsed
        } else {
            0.0
        }
    }

    /// Returns a snapshot of all metrics.
    #[must_use]
    pub fn snapshot(&self) -> CacheMetricsSnapshot {
        CacheMetricsSnapshot {
            hits: self.hits(),
            misses: self.misses(),
            errors: self.errors(),
            gets: self.gets(),
            sets: self.sets(),
            deletes: self.deletes(),
            publishes: self.publishes(),
            uptime: self.uptime(),
            hit_rate: self.hit_rate(),
            ops_per_second: self.ops_per_second(),
        }
    }

    /// Resets all counters.
    pub fn reset(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.errors.store(0, Ordering::Relaxed);
        self.gets.store(0, Ordering::Relaxed);
        self.sets.store(0, Ordering::Relaxed);
        self.deletes.store(0, Ordering::Relaxed);
        self.publishes.store(0, Ordering::Relaxed);
    }
}

/// A point-in-time snapshot of cache metrics.
#[derive(Debug, Clone)]
pub struct CacheMetricsSnapshot {
    /// Cache hits.
    pub hits: u64,
    /// Cache misses.
    pub misses: u64,
    /// Cache errors.
    pub errors: u64,
    /// Get operations.
    pub gets: u64,
    /// Set operations.
    pub sets: u64,
    /// Delete operations.
    pub deletes: u64,
    /// Publish operations.
    pub publishes: u64,
    /// Uptime.
    pub uptime: Duration,
    /// Hit rate.
    pub hit_rate: f64,
    /// Operations per second.
    pub ops_per_second: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_new() {
        let metrics = CacheMetrics::new();
        assert_eq!(metrics.hits(), 0);
        assert_eq!(metrics.misses(), 0);
        assert_eq!(metrics.errors(), 0);
    }

    #[test]
    fn test_metrics_default() {
        let metrics = CacheMetrics::default();
        assert_eq!(metrics.hits(), 0);
    }

    #[test]
    fn test_metrics_record_hit() {
        let metrics = CacheMetrics::new();

        metrics.record_hit();
        metrics.record_hit();

        assert_eq!(metrics.hits(), 2);
        assert_eq!(metrics.gets(), 2);
    }

    #[test]
    fn test_metrics_record_miss() {
        let metrics = CacheMetrics::new();

        metrics.record_miss();

        assert_eq!(metrics.misses(), 1);
        assert_eq!(metrics.gets(), 1);
    }

    #[test]
    fn test_metrics_record_error() {
        let metrics = CacheMetrics::new();

        metrics.record_error();

        assert_eq!(metrics.errors(), 1);
    }

    #[test]
    fn test_metrics_record_set() {
        let metrics = CacheMetrics::new();

        metrics.record_set();
        metrics.record_set();

        assert_eq!(metrics.sets(), 2);
    }

    #[test]
    fn test_metrics_record_delete() {
        let metrics = CacheMetrics::new();

        metrics.record_delete();

        assert_eq!(metrics.deletes(), 1);
    }

    #[test]
    fn test_metrics_record_publish() {
        let metrics = CacheMetrics::new();

        metrics.record_publish();
        metrics.record_publish();

        assert_eq!(metrics.publishes(), 2);
    }

    #[test]
    fn test_metrics_hit_rate() {
        let metrics = CacheMetrics::new();

        // No operations
        assert_eq!(metrics.hit_rate(), 0.0);

        // 2 hits, 1 miss
        metrics.record_hit();
        metrics.record_hit();
        metrics.record_miss();

        let rate = metrics.hit_rate();
        assert!((rate - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_metrics_snapshot() {
        let metrics = CacheMetrics::new();

        metrics.record_hit();
        metrics.record_miss();
        metrics.record_set();

        let snapshot = metrics.snapshot();

        assert_eq!(snapshot.hits, 1);
        assert_eq!(snapshot.misses, 1);
        assert_eq!(snapshot.sets, 1);
    }

    #[test]
    fn test_metrics_reset() {
        let metrics = CacheMetrics::new();

        metrics.record_hit();
        metrics.record_miss();
        metrics.record_error();

        metrics.reset();

        assert_eq!(metrics.hits(), 0);
        assert_eq!(metrics.misses(), 0);
        assert_eq!(metrics.errors(), 0);
    }
}
