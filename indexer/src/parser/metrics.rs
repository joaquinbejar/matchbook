//! Metrics tracking for the account parser.
//!
//! Provides atomic counters for monitoring parse operations.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Metrics for the account parser.
#[derive(Debug)]
pub struct ParserMetrics {
    /// Total number of parse operations.
    parse_count: AtomicU64,

    /// Number of successful parses.
    success_count: AtomicU64,

    /// Number of failed parses.
    error_count: AtomicU64,

    /// Total parse time in nanoseconds.
    total_parse_time_ns: AtomicU64,

    /// Start time for rate calculation.
    start_time: Instant,
}

impl Default for ParserMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl ParserMetrics {
    /// Creates a new metrics instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            parse_count: AtomicU64::new(0),
            success_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            total_parse_time_ns: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    /// Records a parse operation.
    pub fn record_parse(&self, duration: Duration, success: bool) {
        self.parse_count.fetch_add(1, Ordering::Relaxed);
        self.total_parse_time_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);

        if success {
            self.success_count.fetch_add(1, Ordering::Relaxed);
        } else {
            self.error_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Returns the total number of parse operations.
    #[must_use]
    pub fn parse_count(&self) -> u64 {
        self.parse_count.load(Ordering::Relaxed)
    }

    /// Returns the number of successful parses.
    #[must_use]
    pub fn success_count(&self) -> u64 {
        self.success_count.load(Ordering::Relaxed)
    }

    /// Returns the number of failed parses.
    #[must_use]
    pub fn error_count(&self) -> u64 {
        self.error_count.load(Ordering::Relaxed)
    }

    /// Returns the total parse time.
    #[must_use]
    pub fn total_parse_time(&self) -> Duration {
        Duration::from_nanos(self.total_parse_time_ns.load(Ordering::Relaxed))
    }

    /// Returns the average parse time.
    #[must_use]
    pub fn average_parse_time(&self) -> Duration {
        let count = self.parse_count();
        if count == 0 {
            return Duration::ZERO;
        }
        let total_ns = self.total_parse_time_ns.load(Ordering::Relaxed);
        Duration::from_nanos(total_ns / count)
    }

    /// Returns the error rate (0.0 to 1.0).
    #[must_use]
    pub fn error_rate(&self) -> f64 {
        let total = self.parse_count();
        if total == 0 {
            return 0.0;
        }
        self.error_count() as f64 / total as f64
    }

    /// Returns the parses per second since start.
    #[must_use]
    pub fn parses_per_second(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.parse_count() as f64 / elapsed
        } else {
            0.0
        }
    }

    /// Returns a snapshot of all metrics.
    #[must_use]
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            parse_count: self.parse_count(),
            success_count: self.success_count(),
            error_count: self.error_count(),
            total_parse_time: self.total_parse_time(),
            average_parse_time: self.average_parse_time(),
            error_rate: self.error_rate(),
            parses_per_second: self.parses_per_second(),
        }
    }

    /// Resets all counters.
    pub fn reset(&self) {
        self.parse_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
        self.error_count.store(0, Ordering::Relaxed);
        self.total_parse_time_ns.store(0, Ordering::Relaxed);
    }
}

/// A point-in-time snapshot of parser metrics.
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    /// Total parse operations.
    pub parse_count: u64,
    /// Successful parses.
    pub success_count: u64,
    /// Failed parses.
    pub error_count: u64,
    /// Total parse time.
    pub total_parse_time: Duration,
    /// Average parse time.
    pub average_parse_time: Duration,
    /// Error rate (0.0 to 1.0).
    pub error_rate: f64,
    /// Parses per second.
    pub parses_per_second: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_new() {
        let metrics = ParserMetrics::new();
        assert_eq!(metrics.parse_count(), 0);
        assert_eq!(metrics.success_count(), 0);
        assert_eq!(metrics.error_count(), 0);
    }

    #[test]
    fn test_metrics_default() {
        let metrics = ParserMetrics::default();
        assert_eq!(metrics.parse_count(), 0);
    }

    #[test]
    fn test_metrics_record_success() {
        let metrics = ParserMetrics::new();
        metrics.record_parse(Duration::from_micros(100), true);

        assert_eq!(metrics.parse_count(), 1);
        assert_eq!(metrics.success_count(), 1);
        assert_eq!(metrics.error_count(), 0);
    }

    #[test]
    fn test_metrics_record_error() {
        let metrics = ParserMetrics::new();
        metrics.record_parse(Duration::from_micros(100), false);

        assert_eq!(metrics.parse_count(), 1);
        assert_eq!(metrics.success_count(), 0);
        assert_eq!(metrics.error_count(), 1);
    }

    #[test]
    fn test_metrics_error_rate() {
        let metrics = ParserMetrics::new();

        // No parses = 0% error rate
        assert_eq!(metrics.error_rate(), 0.0);

        // 1 success, 1 error = 50% error rate
        metrics.record_parse(Duration::from_micros(100), true);
        metrics.record_parse(Duration::from_micros(100), false);
        assert!((metrics.error_rate() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_metrics_average_parse_time() {
        let metrics = ParserMetrics::new();

        // No parses = zero average
        assert_eq!(metrics.average_parse_time(), Duration::ZERO);

        // Record some parses
        metrics.record_parse(Duration::from_micros(100), true);
        metrics.record_parse(Duration::from_micros(200), true);

        // Average should be 150 microseconds
        let avg = metrics.average_parse_time();
        assert!(avg >= Duration::from_micros(100));
        assert!(avg <= Duration::from_micros(200));
    }

    #[test]
    fn test_metrics_reset() {
        let metrics = ParserMetrics::new();
        metrics.record_parse(Duration::from_micros(100), true);
        metrics.record_parse(Duration::from_micros(100), false);

        metrics.reset();

        assert_eq!(metrics.parse_count(), 0);
        assert_eq!(metrics.success_count(), 0);
        assert_eq!(metrics.error_count(), 0);
    }

    #[test]
    fn test_metrics_snapshot() {
        let metrics = ParserMetrics::new();
        metrics.record_parse(Duration::from_micros(100), true);
        metrics.record_parse(Duration::from_micros(100), false);

        let snapshot = metrics.snapshot();

        assert_eq!(snapshot.parse_count, 2);
        assert_eq!(snapshot.success_count, 1);
        assert_eq!(snapshot.error_count, 1);
        assert!((snapshot.error_rate - 0.5).abs() < 0.001);
    }
}
