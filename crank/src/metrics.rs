//! Crank service metrics.
//!
//! Provides atomic counters for monitoring crank operations.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Metrics for the crank service.
#[derive(Debug)]
pub struct CrankMetrics {
    /// Total matches executed.
    matches_executed: AtomicU64,

    /// Total transactions submitted.
    transactions_submitted: AtomicU64,

    /// Successful transactions.
    transactions_success: AtomicU64,

    /// Failed transactions.
    transactions_failed: AtomicU64,

    /// Dropped transactions.
    transactions_dropped: AtomicU64,

    /// Total priority fees paid in lamports.
    fees_paid: AtomicU64,

    /// Total crosses detected.
    crosses_detected: AtomicU64,

    /// Total poll cycles.
    poll_cycles: AtomicU64,

    /// Start time for rate calculation.
    start_time: Instant,
}

impl Default for CrankMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl CrankMetrics {
    /// Creates a new metrics instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            matches_executed: AtomicU64::new(0),
            transactions_submitted: AtomicU64::new(0),
            transactions_success: AtomicU64::new(0),
            transactions_failed: AtomicU64::new(0),
            transactions_dropped: AtomicU64::new(0),
            fees_paid: AtomicU64::new(0),
            crosses_detected: AtomicU64::new(0),
            poll_cycles: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    /// Records a successful transaction.
    pub fn record_success(&self, matches: u32) {
        self.transactions_success.fetch_add(1, Ordering::Relaxed);
        self.matches_executed
            .fetch_add(u64::from(matches), Ordering::Relaxed);
    }

    /// Records a failed transaction.
    pub fn record_failure(&self) {
        self.transactions_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a dropped transaction.
    pub fn record_dropped(&self) {
        self.transactions_dropped.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a transaction submission.
    pub fn record_submission(&self, priority_fee: u64) {
        self.transactions_submitted.fetch_add(1, Ordering::Relaxed);
        self.fees_paid.fetch_add(priority_fee, Ordering::Relaxed);
    }

    /// Records a detected cross.
    pub fn record_cross(&self) {
        self.crosses_detected.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a poll cycle.
    pub fn record_poll(&self) {
        self.poll_cycles.fetch_add(1, Ordering::Relaxed);
    }

    /// Returns total matches executed.
    #[must_use]
    pub fn matches_executed(&self) -> u64 {
        self.matches_executed.load(Ordering::Relaxed)
    }

    /// Returns total transactions submitted.
    #[must_use]
    pub fn transactions_submitted(&self) -> u64 {
        self.transactions_submitted.load(Ordering::Relaxed)
    }

    /// Returns successful transactions.
    #[must_use]
    pub fn transactions_success(&self) -> u64 {
        self.transactions_success.load(Ordering::Relaxed)
    }

    /// Returns failed transactions.
    #[must_use]
    pub fn transactions_failed(&self) -> u64 {
        self.transactions_failed.load(Ordering::Relaxed)
    }

    /// Returns dropped transactions.
    #[must_use]
    pub fn transactions_dropped(&self) -> u64 {
        self.transactions_dropped.load(Ordering::Relaxed)
    }

    /// Returns total fees paid.
    #[must_use]
    pub fn fees_paid(&self) -> u64 {
        self.fees_paid.load(Ordering::Relaxed)
    }

    /// Returns crosses detected.
    #[must_use]
    pub fn crosses_detected(&self) -> u64 {
        self.crosses_detected.load(Ordering::Relaxed)
    }

    /// Returns poll cycles.
    #[must_use]
    pub fn poll_cycles(&self) -> u64 {
        self.poll_cycles.load(Ordering::Relaxed)
    }

    /// Returns the uptime.
    #[must_use]
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Returns the success rate (0.0 to 1.0).
    #[must_use]
    pub fn success_rate(&self) -> f64 {
        let submitted = self.transactions_submitted();
        if submitted > 0 {
            self.transactions_success() as f64 / submitted as f64
        } else {
            0.0
        }
    }

    /// Returns matches per second.
    #[must_use]
    pub fn matches_per_second(&self) -> f64 {
        let elapsed = self.uptime().as_secs_f64();
        if elapsed > 0.0 {
            self.matches_executed() as f64 / elapsed
        } else {
            0.0
        }
    }

    /// Returns a snapshot of all metrics.
    #[must_use]
    pub fn snapshot(&self) -> CrankMetricsSnapshot {
        CrankMetricsSnapshot {
            matches_executed: self.matches_executed(),
            transactions_submitted: self.transactions_submitted(),
            transactions_success: self.transactions_success(),
            transactions_failed: self.transactions_failed(),
            transactions_dropped: self.transactions_dropped(),
            fees_paid: self.fees_paid(),
            crosses_detected: self.crosses_detected(),
            poll_cycles: self.poll_cycles(),
            uptime: self.uptime(),
            success_rate: self.success_rate(),
            matches_per_second: self.matches_per_second(),
        }
    }

    /// Resets all counters.
    pub fn reset(&self) {
        self.matches_executed.store(0, Ordering::Relaxed);
        self.transactions_submitted.store(0, Ordering::Relaxed);
        self.transactions_success.store(0, Ordering::Relaxed);
        self.transactions_failed.store(0, Ordering::Relaxed);
        self.transactions_dropped.store(0, Ordering::Relaxed);
        self.fees_paid.store(0, Ordering::Relaxed);
        self.crosses_detected.store(0, Ordering::Relaxed);
        self.poll_cycles.store(0, Ordering::Relaxed);
    }
}

/// A point-in-time snapshot of crank metrics.
#[derive(Debug, Clone)]
pub struct CrankMetricsSnapshot {
    /// Total matches executed.
    pub matches_executed: u64,
    /// Total transactions submitted.
    pub transactions_submitted: u64,
    /// Successful transactions.
    pub transactions_success: u64,
    /// Failed transactions.
    pub transactions_failed: u64,
    /// Dropped transactions.
    pub transactions_dropped: u64,
    /// Total fees paid.
    pub fees_paid: u64,
    /// Crosses detected.
    pub crosses_detected: u64,
    /// Poll cycles.
    pub poll_cycles: u64,
    /// Uptime.
    pub uptime: Duration,
    /// Success rate.
    pub success_rate: f64,
    /// Matches per second.
    pub matches_per_second: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_new() {
        let metrics = CrankMetrics::new();
        assert_eq!(metrics.matches_executed(), 0);
        assert_eq!(metrics.transactions_submitted(), 0);
    }

    #[test]
    fn test_metrics_default() {
        let metrics = CrankMetrics::default();
        assert_eq!(metrics.matches_executed(), 0);
    }

    #[test]
    fn test_metrics_record_success() {
        let metrics = CrankMetrics::new();

        metrics.record_success(5);
        metrics.record_success(3);

        assert_eq!(metrics.matches_executed(), 8);
        assert_eq!(metrics.transactions_success(), 2);
    }

    #[test]
    fn test_metrics_record_failure() {
        let metrics = CrankMetrics::new();

        metrics.record_failure();
        metrics.record_failure();

        assert_eq!(metrics.transactions_failed(), 2);
    }

    #[test]
    fn test_metrics_record_dropped() {
        let metrics = CrankMetrics::new();

        metrics.record_dropped();

        assert_eq!(metrics.transactions_dropped(), 1);
    }

    #[test]
    fn test_metrics_record_submission() {
        let metrics = CrankMetrics::new();

        metrics.record_submission(1000);
        metrics.record_submission(2000);

        assert_eq!(metrics.transactions_submitted(), 2);
        assert_eq!(metrics.fees_paid(), 3000);
    }

    #[test]
    fn test_metrics_record_cross() {
        let metrics = CrankMetrics::new();

        metrics.record_cross();
        metrics.record_cross();

        assert_eq!(metrics.crosses_detected(), 2);
    }

    #[test]
    fn test_metrics_record_poll() {
        let metrics = CrankMetrics::new();

        metrics.record_poll();

        assert_eq!(metrics.poll_cycles(), 1);
    }

    #[test]
    fn test_metrics_success_rate() {
        let metrics = CrankMetrics::new();

        // No submissions
        assert_eq!(metrics.success_rate(), 0.0);

        // 2 success, 1 failure
        metrics.record_submission(1000);
        metrics.record_success(1);
        metrics.record_submission(1000);
        metrics.record_success(1);
        metrics.record_submission(1000);
        metrics.record_failure();

        let rate = metrics.success_rate();
        assert!((rate - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_metrics_snapshot() {
        let metrics = CrankMetrics::new();

        metrics.record_success(5);
        metrics.record_submission(1000);
        metrics.record_cross();

        let snapshot = metrics.snapshot();

        assert_eq!(snapshot.matches_executed, 5);
        assert_eq!(snapshot.transactions_submitted, 1);
        assert_eq!(snapshot.crosses_detected, 1);
    }

    #[test]
    fn test_metrics_reset() {
        let metrics = CrankMetrics::new();

        metrics.record_success(5);
        metrics.record_submission(1000);
        metrics.record_cross();

        metrics.reset();

        assert_eq!(metrics.matches_executed(), 0);
        assert_eq!(metrics.transactions_submitted(), 0);
        assert_eq!(metrics.crosses_detected(), 0);
    }
}
