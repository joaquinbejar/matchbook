//! Metrics tracking for the book builder.
//!
//! Provides atomic counters for monitoring book operations.

use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Metrics for the book builder.
#[derive(Debug)]
pub struct BookMetrics {
    /// Total number of updates applied.
    update_count: AtomicU64,

    /// Total update time in nanoseconds.
    total_update_time_ns: AtomicU64,

    /// Number of snapshots generated.
    snapshot_count: AtomicU64,

    /// Current total depth across all books.
    total_depth: AtomicU64,

    /// Current spread (in price units, -1 if unavailable).
    current_spread: AtomicI64,

    /// Start time for rate calculation.
    start_time: Instant,
}

impl Default for BookMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl BookMetrics {
    /// Creates a new metrics instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            update_count: AtomicU64::new(0),
            total_update_time_ns: AtomicU64::new(0),
            snapshot_count: AtomicU64::new(0),
            total_depth: AtomicU64::new(0),
            current_spread: AtomicI64::new(-1),
            start_time: Instant::now(),
        }
    }

    /// Records an update operation.
    pub fn record_update(&self, duration: Duration) {
        self.update_count.fetch_add(1, Ordering::Relaxed);
        self.total_update_time_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    /// Records a snapshot generation.
    pub fn record_snapshot(&self) {
        self.snapshot_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Updates the total depth metric.
    pub fn set_total_depth(&self, depth: u64) {
        self.total_depth.store(depth, Ordering::Relaxed);
    }

    /// Updates the current spread metric.
    pub fn set_spread(&self, spread: Option<u64>) {
        let value = spread.map(|s| s as i64).unwrap_or(-1);
        self.current_spread.store(value, Ordering::Relaxed);
    }

    /// Returns the total number of updates.
    #[must_use]
    pub fn update_count(&self) -> u64 {
        self.update_count.load(Ordering::Relaxed)
    }

    /// Returns the total update time.
    #[must_use]
    pub fn total_update_time(&self) -> Duration {
        Duration::from_nanos(self.total_update_time_ns.load(Ordering::Relaxed))
    }

    /// Returns the average update time.
    #[must_use]
    pub fn average_update_time(&self) -> Duration {
        let count = self.update_count();
        if count == 0 {
            return Duration::ZERO;
        }
        let total_ns = self.total_update_time_ns.load(Ordering::Relaxed);
        Duration::from_nanos(total_ns / count)
    }

    /// Returns the number of snapshots generated.
    #[must_use]
    pub fn snapshot_count(&self) -> u64 {
        self.snapshot_count.load(Ordering::Relaxed)
    }

    /// Returns the total depth.
    #[must_use]
    pub fn total_depth(&self) -> u64 {
        self.total_depth.load(Ordering::Relaxed)
    }

    /// Returns the current spread.
    #[must_use]
    pub fn current_spread(&self) -> Option<u64> {
        let value = self.current_spread.load(Ordering::Relaxed);
        if value < 0 {
            None
        } else {
            Some(value as u64)
        }
    }

    /// Returns the updates per second since start.
    #[must_use]
    pub fn updates_per_second(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.update_count() as f64 / elapsed
        } else {
            0.0
        }
    }

    /// Returns a snapshot of all metrics.
    #[must_use]
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            update_count: self.update_count(),
            average_update_time: self.average_update_time(),
            snapshot_count: self.snapshot_count(),
            total_depth: self.total_depth(),
            current_spread: self.current_spread(),
            updates_per_second: self.updates_per_second(),
        }
    }

    /// Resets all counters.
    pub fn reset(&self) {
        self.update_count.store(0, Ordering::Relaxed);
        self.total_update_time_ns.store(0, Ordering::Relaxed);
        self.snapshot_count.store(0, Ordering::Relaxed);
    }
}

/// A point-in-time snapshot of book metrics.
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    /// Total updates applied.
    pub update_count: u64,
    /// Average update time.
    pub average_update_time: Duration,
    /// Snapshots generated.
    pub snapshot_count: u64,
    /// Total depth across all books.
    pub total_depth: u64,
    /// Current spread.
    pub current_spread: Option<u64>,
    /// Updates per second.
    pub updates_per_second: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_new() {
        let metrics = BookMetrics::new();
        assert_eq!(metrics.update_count(), 0);
        assert_eq!(metrics.snapshot_count(), 0);
        assert_eq!(metrics.total_depth(), 0);
        assert!(metrics.current_spread().is_none());
    }

    #[test]
    fn test_metrics_default() {
        let metrics = BookMetrics::default();
        assert_eq!(metrics.update_count(), 0);
    }

    #[test]
    fn test_metrics_record_update() {
        let metrics = BookMetrics::new();
        metrics.record_update(Duration::from_micros(100));

        assert_eq!(metrics.update_count(), 1);
        assert!(metrics.total_update_time() >= Duration::from_micros(100));
    }

    #[test]
    fn test_metrics_record_snapshot() {
        let metrics = BookMetrics::new();
        metrics.record_snapshot();
        metrics.record_snapshot();

        assert_eq!(metrics.snapshot_count(), 2);
    }

    #[test]
    fn test_metrics_set_depth() {
        let metrics = BookMetrics::new();
        metrics.set_total_depth(100);

        assert_eq!(metrics.total_depth(), 100);
    }

    #[test]
    fn test_metrics_set_spread() {
        let metrics = BookMetrics::new();

        // No spread
        assert!(metrics.current_spread().is_none());

        // Set spread
        metrics.set_spread(Some(5));
        assert_eq!(metrics.current_spread(), Some(5));

        // Clear spread
        metrics.set_spread(None);
        assert!(metrics.current_spread().is_none());
    }

    #[test]
    fn test_metrics_average_update_time() {
        let metrics = BookMetrics::new();

        // No updates = zero average
        assert_eq!(metrics.average_update_time(), Duration::ZERO);

        // Record some updates
        metrics.record_update(Duration::from_micros(100));
        metrics.record_update(Duration::from_micros(200));

        let avg = metrics.average_update_time();
        assert!(avg >= Duration::from_micros(100));
        assert!(avg <= Duration::from_micros(200));
    }

    #[test]
    fn test_metrics_reset() {
        let metrics = BookMetrics::new();
        metrics.record_update(Duration::from_micros(100));
        metrics.record_snapshot();

        metrics.reset();

        assert_eq!(metrics.update_count(), 0);
        assert_eq!(metrics.snapshot_count(), 0);
    }

    #[test]
    fn test_metrics_snapshot() {
        let metrics = BookMetrics::new();
        metrics.record_update(Duration::from_micros(100));
        metrics.set_total_depth(50);
        metrics.set_spread(Some(10));

        let snapshot = metrics.snapshot();

        assert_eq!(snapshot.update_count, 1);
        assert_eq!(snapshot.total_depth, 50);
        assert_eq!(snapshot.current_spread, Some(10));
    }
}
