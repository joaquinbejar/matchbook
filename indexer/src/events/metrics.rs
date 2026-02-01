//! Metrics tracking for the event processor.
//!
//! Provides atomic counters for monitoring event processing.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Metrics for the event processor.
#[derive(Debug)]
pub struct EventMetrics {
    /// Total number of events processed.
    events_processed: AtomicU64,

    /// Number of fill events.
    fills_processed: AtomicU64,

    /// Number of out events.
    outs_processed: AtomicU64,

    /// Number of events skipped (already processed).
    events_skipped: AtomicU64,

    /// Number of processing errors.
    errors: AtomicU64,

    /// Total processing time in nanoseconds.
    total_processing_time_ns: AtomicU64,

    /// Start time for rate calculation.
    start_time: Instant,
}

impl Default for EventMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl EventMetrics {
    /// Creates a new metrics instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            events_processed: AtomicU64::new(0),
            fills_processed: AtomicU64::new(0),
            outs_processed: AtomicU64::new(0),
            events_skipped: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            total_processing_time_ns: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    /// Records a batch of processed events.
    pub fn record_batch(&self, fills: u64, outs: u64, skipped: u64, duration: Duration) {
        self.events_processed
            .fetch_add(fills + outs, Ordering::Relaxed);
        self.fills_processed.fetch_add(fills, Ordering::Relaxed);
        self.outs_processed.fetch_add(outs, Ordering::Relaxed);
        self.events_skipped.fetch_add(skipped, Ordering::Relaxed);
        self.total_processing_time_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    /// Records a fill event.
    pub fn record_fill(&self) {
        self.events_processed.fetch_add(1, Ordering::Relaxed);
        self.fills_processed.fetch_add(1, Ordering::Relaxed);
    }

    /// Records an out event.
    pub fn record_out(&self) {
        self.events_processed.fetch_add(1, Ordering::Relaxed);
        self.outs_processed.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a skipped event.
    pub fn record_skipped(&self) {
        self.events_skipped.fetch_add(1, Ordering::Relaxed);
    }

    /// Records an error.
    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Returns the total events processed.
    #[must_use]
    pub fn events_processed(&self) -> u64 {
        self.events_processed.load(Ordering::Relaxed)
    }

    /// Returns the number of fills processed.
    #[must_use]
    pub fn fills_processed(&self) -> u64 {
        self.fills_processed.load(Ordering::Relaxed)
    }

    /// Returns the number of outs processed.
    #[must_use]
    pub fn outs_processed(&self) -> u64 {
        self.outs_processed.load(Ordering::Relaxed)
    }

    /// Returns the number of events skipped.
    #[must_use]
    pub fn events_skipped(&self) -> u64 {
        self.events_skipped.load(Ordering::Relaxed)
    }

    /// Returns the number of errors.
    #[must_use]
    pub fn errors(&self) -> u64 {
        self.errors.load(Ordering::Relaxed)
    }

    /// Returns the total processing time.
    #[must_use]
    pub fn total_processing_time(&self) -> Duration {
        Duration::from_nanos(self.total_processing_time_ns.load(Ordering::Relaxed))
    }

    /// Returns the average processing time per event.
    #[must_use]
    pub fn average_processing_time(&self) -> Duration {
        let count = self.events_processed();
        if count == 0 {
            return Duration::ZERO;
        }
        let total_ns = self.total_processing_time_ns.load(Ordering::Relaxed);
        Duration::from_nanos(total_ns / count)
    }

    /// Returns the events per second since start.
    #[must_use]
    pub fn events_per_second(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.events_processed() as f64 / elapsed
        } else {
            0.0
        }
    }

    /// Returns the error rate (0.0 to 1.0).
    #[must_use]
    pub fn error_rate(&self) -> f64 {
        let total = self.events_processed() + self.errors();
        if total == 0 {
            return 0.0;
        }
        self.errors() as f64 / total as f64
    }

    /// Returns a snapshot of all metrics.
    #[must_use]
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            events_processed: self.events_processed(),
            fills_processed: self.fills_processed(),
            outs_processed: self.outs_processed(),
            events_skipped: self.events_skipped(),
            errors: self.errors(),
            average_processing_time: self.average_processing_time(),
            events_per_second: self.events_per_second(),
            error_rate: self.error_rate(),
        }
    }

    /// Resets all counters.
    pub fn reset(&self) {
        self.events_processed.store(0, Ordering::Relaxed);
        self.fills_processed.store(0, Ordering::Relaxed);
        self.outs_processed.store(0, Ordering::Relaxed);
        self.events_skipped.store(0, Ordering::Relaxed);
        self.errors.store(0, Ordering::Relaxed);
        self.total_processing_time_ns.store(0, Ordering::Relaxed);
    }
}

/// A point-in-time snapshot of event metrics.
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    /// Total events processed.
    pub events_processed: u64,
    /// Fills processed.
    pub fills_processed: u64,
    /// Outs processed.
    pub outs_processed: u64,
    /// Events skipped.
    pub events_skipped: u64,
    /// Errors.
    pub errors: u64,
    /// Average processing time.
    pub average_processing_time: Duration,
    /// Events per second.
    pub events_per_second: f64,
    /// Error rate.
    pub error_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_new() {
        let metrics = EventMetrics::new();
        assert_eq!(metrics.events_processed(), 0);
        assert_eq!(metrics.fills_processed(), 0);
        assert_eq!(metrics.outs_processed(), 0);
        assert_eq!(metrics.events_skipped(), 0);
        assert_eq!(metrics.errors(), 0);
    }

    #[test]
    fn test_metrics_default() {
        let metrics = EventMetrics::default();
        assert_eq!(metrics.events_processed(), 0);
    }

    #[test]
    fn test_metrics_record_fill() {
        let metrics = EventMetrics::new();
        metrics.record_fill();
        metrics.record_fill();

        assert_eq!(metrics.events_processed(), 2);
        assert_eq!(metrics.fills_processed(), 2);
        assert_eq!(metrics.outs_processed(), 0);
    }

    #[test]
    fn test_metrics_record_out() {
        let metrics = EventMetrics::new();
        metrics.record_out();

        assert_eq!(metrics.events_processed(), 1);
        assert_eq!(metrics.fills_processed(), 0);
        assert_eq!(metrics.outs_processed(), 1);
    }

    #[test]
    fn test_metrics_record_batch() {
        let metrics = EventMetrics::new();
        metrics.record_batch(5, 3, 2, Duration::from_micros(100));

        assert_eq!(metrics.events_processed(), 8);
        assert_eq!(metrics.fills_processed(), 5);
        assert_eq!(metrics.outs_processed(), 3);
        assert_eq!(metrics.events_skipped(), 2);
    }

    #[test]
    fn test_metrics_record_skipped() {
        let metrics = EventMetrics::new();
        metrics.record_skipped();
        metrics.record_skipped();

        assert_eq!(metrics.events_skipped(), 2);
    }

    #[test]
    fn test_metrics_record_error() {
        let metrics = EventMetrics::new();
        metrics.record_error();

        assert_eq!(metrics.errors(), 1);
    }

    #[test]
    fn test_metrics_error_rate() {
        let metrics = EventMetrics::new();

        // No events = 0% error rate
        assert_eq!(metrics.error_rate(), 0.0);

        // 1 success, 1 error = 50% error rate
        metrics.record_fill();
        metrics.record_error();
        assert!((metrics.error_rate() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_metrics_reset() {
        let metrics = EventMetrics::new();
        metrics.record_fill();
        metrics.record_out();
        metrics.record_error();

        metrics.reset();

        assert_eq!(metrics.events_processed(), 0);
        assert_eq!(metrics.fills_processed(), 0);
        assert_eq!(metrics.outs_processed(), 0);
        assert_eq!(metrics.errors(), 0);
    }

    #[test]
    fn test_metrics_snapshot() {
        let metrics = EventMetrics::new();
        metrics.record_fill();
        metrics.record_out();
        metrics.record_skipped();

        let snapshot = metrics.snapshot();

        assert_eq!(snapshot.events_processed, 2);
        assert_eq!(snapshot.fills_processed, 1);
        assert_eq!(snapshot.outs_processed, 1);
        assert_eq!(snapshot.events_skipped, 1);
    }
}
