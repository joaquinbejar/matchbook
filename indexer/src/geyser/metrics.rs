//! Metrics tracking for Geyser listener.
//!
//! Provides atomic counters and gauges for monitoring the health
//! and performance of the Geyser connection.

use std::sync::atomic::{AtomicI64, AtomicU64, AtomicU8, Ordering};
use std::time::Instant;

use super::types::ConnectionState;

/// Metrics for the Geyser listener.
#[derive(Debug)]
pub struct GeyserMetrics {
    /// Total number of account updates received.
    updates_received: AtomicU64,

    /// Timestamp of the last update (Unix epoch seconds).
    last_update_time: AtomicU64,

    /// Slot number of the last update.
    last_update_slot: AtomicU64,

    /// Current connection state.
    connection_state: AtomicU8,

    /// Number of reconnection attempts.
    reconnect_count: AtomicU64,

    /// Estimated lag in slots (current slot - last update slot).
    lag_slots: AtomicI64,

    /// Number of updates dropped due to backpressure.
    updates_dropped: AtomicU64,

    /// Start time for rate calculation.
    start_time: Instant,
}

impl Default for GeyserMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl GeyserMetrics {
    /// Creates a new metrics instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            updates_received: AtomicU64::new(0),
            last_update_time: AtomicU64::new(0),
            last_update_slot: AtomicU64::new(0),
            connection_state: AtomicU8::new(ConnectionState::Disconnected as u8),
            reconnect_count: AtomicU64::new(0),
            lag_slots: AtomicI64::new(0),
            updates_dropped: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    /// Records an account update.
    pub fn record_update(&self, slot: u64) {
        self.updates_received.fetch_add(1, Ordering::Relaxed);
        self.last_update_slot.store(slot, Ordering::Relaxed);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.last_update_time.store(now, Ordering::Relaxed);
    }

    /// Records a dropped update due to backpressure.
    pub fn record_dropped(&self) {
        self.updates_dropped.fetch_add(1, Ordering::Relaxed);
    }

    /// Updates the connection state.
    pub fn set_connection_state(&self, state: ConnectionState) {
        self.connection_state.store(state as u8, Ordering::Relaxed);
    }

    /// Increments the reconnection counter.
    pub fn record_reconnect(&self) {
        self.reconnect_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Updates the lag estimate.
    pub fn set_lag(&self, current_slot: u64) {
        let last_slot = self.last_update_slot.load(Ordering::Relaxed);
        let lag = current_slot.saturating_sub(last_slot) as i64;
        self.lag_slots.store(lag, Ordering::Relaxed);
    }

    /// Returns the total number of updates received.
    #[must_use]
    pub fn updates_received(&self) -> u64 {
        self.updates_received.load(Ordering::Relaxed)
    }

    /// Returns the slot of the last update.
    #[must_use]
    pub fn last_update_slot(&self) -> u64 {
        self.last_update_slot.load(Ordering::Relaxed)
    }

    /// Returns the timestamp of the last update.
    #[must_use]
    pub fn last_update_time(&self) -> u64 {
        self.last_update_time.load(Ordering::Relaxed)
    }

    /// Returns the current connection state.
    #[must_use]
    pub fn connection_state(&self) -> ConnectionState {
        match self.connection_state.load(Ordering::Relaxed) {
            0 => ConnectionState::Disconnected,
            1 => ConnectionState::Connecting,
            2 => ConnectionState::Connected,
            3 => ConnectionState::Reconnecting,
            4 => ConnectionState::Failed,
            _ => ConnectionState::Disconnected,
        }
    }

    /// Returns the number of reconnection attempts.
    #[must_use]
    pub fn reconnect_count(&self) -> u64 {
        self.reconnect_count.load(Ordering::Relaxed)
    }

    /// Returns the estimated lag in slots.
    #[must_use]
    pub fn lag_slots(&self) -> i64 {
        self.lag_slots.load(Ordering::Relaxed)
    }

    /// Returns the number of dropped updates.
    #[must_use]
    pub fn updates_dropped(&self) -> u64 {
        self.updates_dropped.load(Ordering::Relaxed)
    }

    /// Returns the average updates per second since start.
    #[must_use]
    pub fn updates_per_second(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.updates_received() as f64 / elapsed
        } else {
            0.0
        }
    }

    /// Returns a snapshot of all metrics.
    #[must_use]
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            updates_received: self.updates_received(),
            updates_per_second: self.updates_per_second(),
            last_update_slot: self.last_update_slot(),
            last_update_time: self.last_update_time(),
            connection_state: self.connection_state(),
            reconnect_count: self.reconnect_count(),
            lag_slots: self.lag_slots(),
            updates_dropped: self.updates_dropped(),
        }
    }

    /// Resets all counters (useful for testing).
    pub fn reset(&self) {
        self.updates_received.store(0, Ordering::Relaxed);
        self.last_update_time.store(0, Ordering::Relaxed);
        self.last_update_slot.store(0, Ordering::Relaxed);
        self.reconnect_count.store(0, Ordering::Relaxed);
        self.lag_slots.store(0, Ordering::Relaxed);
        self.updates_dropped.store(0, Ordering::Relaxed);
    }
}

/// A point-in-time snapshot of metrics.
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    /// Total updates received.
    pub updates_received: u64,
    /// Updates per second.
    pub updates_per_second: f64,
    /// Last update slot.
    pub last_update_slot: u64,
    /// Last update time (Unix epoch).
    pub last_update_time: u64,
    /// Connection state.
    pub connection_state: ConnectionState,
    /// Reconnection count.
    pub reconnect_count: u64,
    /// Lag in slots.
    pub lag_slots: i64,
    /// Dropped updates.
    pub updates_dropped: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_new() {
        let metrics = GeyserMetrics::new();
        assert_eq!(metrics.updates_received(), 0);
        assert_eq!(metrics.last_update_slot(), 0);
        assert_eq!(metrics.connection_state(), ConnectionState::Disconnected);
    }

    #[test]
    fn test_metrics_record_update() {
        let metrics = GeyserMetrics::new();
        metrics.record_update(100);
        assert_eq!(metrics.updates_received(), 1);
        assert_eq!(metrics.last_update_slot(), 100);

        metrics.record_update(101);
        assert_eq!(metrics.updates_received(), 2);
        assert_eq!(metrics.last_update_slot(), 101);
    }

    #[test]
    fn test_metrics_connection_state() {
        let metrics = GeyserMetrics::new();

        metrics.set_connection_state(ConnectionState::Connecting);
        assert_eq!(metrics.connection_state(), ConnectionState::Connecting);

        metrics.set_connection_state(ConnectionState::Connected);
        assert_eq!(metrics.connection_state(), ConnectionState::Connected);
    }

    #[test]
    fn test_metrics_reconnect() {
        let metrics = GeyserMetrics::new();
        assert_eq!(metrics.reconnect_count(), 0);

        metrics.record_reconnect();
        assert_eq!(metrics.reconnect_count(), 1);

        metrics.record_reconnect();
        assert_eq!(metrics.reconnect_count(), 2);
    }

    #[test]
    fn test_metrics_lag() {
        let metrics = GeyserMetrics::new();
        metrics.record_update(100);
        metrics.set_lag(105);
        assert_eq!(metrics.lag_slots(), 5);
    }

    #[test]
    fn test_metrics_dropped() {
        let metrics = GeyserMetrics::new();
        metrics.record_dropped();
        metrics.record_dropped();
        assert_eq!(metrics.updates_dropped(), 2);
    }

    #[test]
    fn test_metrics_snapshot() {
        let metrics = GeyserMetrics::new();
        metrics.record_update(100);
        metrics.set_connection_state(ConnectionState::Connected);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.updates_received, 1);
        assert_eq!(snapshot.last_update_slot, 100);
        assert_eq!(snapshot.connection_state, ConnectionState::Connected);
    }

    #[test]
    fn test_metrics_reset() {
        let metrics = GeyserMetrics::new();
        metrics.record_update(100);
        metrics.record_reconnect();
        metrics.record_dropped();

        metrics.reset();

        assert_eq!(metrics.updates_received(), 0);
        assert_eq!(metrics.last_update_slot(), 0);
        assert_eq!(metrics.reconnect_count(), 0);
        assert_eq!(metrics.updates_dropped(), 0);
    }
}
