//! WebSocket metrics tracking.
//!
//! Provides atomic counters for monitoring WebSocket connections.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Metrics for the WebSocket server.
#[derive(Debug)]
pub struct WsMetrics {
    /// Total connections opened.
    connections_opened: AtomicU64,

    /// Total connections closed.
    connections_closed: AtomicU64,

    /// Total messages received.
    messages_received: AtomicU64,

    /// Total messages sent.
    messages_sent: AtomicU64,

    /// Total errors.
    errors: AtomicU64,

    /// Total subscriptions.
    subscriptions: AtomicU64,

    /// Total unsubscriptions.
    unsubscriptions: AtomicU64,

    /// Start time for rate calculation.
    start_time: Instant,
}

impl Default for WsMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl WsMetrics {
    /// Creates a new metrics instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            connections_opened: AtomicU64::new(0),
            connections_closed: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            messages_sent: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            subscriptions: AtomicU64::new(0),
            unsubscriptions: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    /// Records a connection opened.
    pub fn record_connection_opened(&self) {
        self.connections_opened.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a connection closed.
    pub fn record_connection_closed(&self) {
        self.connections_closed.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a message received.
    pub fn record_message_received(&self) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a message sent.
    pub fn record_message_sent(&self) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
    }

    /// Records an error.
    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a subscription.
    pub fn record_subscription(&self) {
        self.subscriptions.fetch_add(1, Ordering::Relaxed);
    }

    /// Records an unsubscription.
    pub fn record_unsubscription(&self) {
        self.unsubscriptions.fetch_add(1, Ordering::Relaxed);
    }

    /// Returns the total connections opened.
    #[must_use]
    pub fn connections_opened(&self) -> u64 {
        self.connections_opened.load(Ordering::Relaxed)
    }

    /// Returns the total connections closed.
    #[must_use]
    pub fn connections_closed(&self) -> u64 {
        self.connections_closed.load(Ordering::Relaxed)
    }

    /// Returns the current active connections.
    #[must_use]
    pub fn active_connections(&self) -> u64 {
        self.connections_opened()
            .saturating_sub(self.connections_closed())
    }

    /// Returns the total messages received.
    #[must_use]
    pub fn messages_received(&self) -> u64 {
        self.messages_received.load(Ordering::Relaxed)
    }

    /// Returns the total messages sent.
    #[must_use]
    pub fn messages_sent(&self) -> u64 {
        self.messages_sent.load(Ordering::Relaxed)
    }

    /// Returns the total errors.
    #[must_use]
    pub fn errors(&self) -> u64 {
        self.errors.load(Ordering::Relaxed)
    }

    /// Returns the total subscriptions.
    #[must_use]
    pub fn subscriptions(&self) -> u64 {
        self.subscriptions.load(Ordering::Relaxed)
    }

    /// Returns the total unsubscriptions.
    #[must_use]
    pub fn unsubscriptions(&self) -> u64 {
        self.unsubscriptions.load(Ordering::Relaxed)
    }

    /// Returns the uptime.
    #[must_use]
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Returns messages per second (received + sent).
    #[must_use]
    pub fn messages_per_second(&self) -> f64 {
        let elapsed = self.uptime().as_secs_f64();
        if elapsed > 0.0 {
            (self.messages_received() + self.messages_sent()) as f64 / elapsed
        } else {
            0.0
        }
    }

    /// Returns a snapshot of all metrics.
    #[must_use]
    pub fn snapshot(&self) -> WsMetricsSnapshot {
        WsMetricsSnapshot {
            connections_opened: self.connections_opened(),
            connections_closed: self.connections_closed(),
            active_connections: self.active_connections(),
            messages_received: self.messages_received(),
            messages_sent: self.messages_sent(),
            errors: self.errors(),
            subscriptions: self.subscriptions(),
            unsubscriptions: self.unsubscriptions(),
            uptime: self.uptime(),
            messages_per_second: self.messages_per_second(),
        }
    }

    /// Resets all counters.
    pub fn reset(&self) {
        self.connections_opened.store(0, Ordering::Relaxed);
        self.connections_closed.store(0, Ordering::Relaxed);
        self.messages_received.store(0, Ordering::Relaxed);
        self.messages_sent.store(0, Ordering::Relaxed);
        self.errors.store(0, Ordering::Relaxed);
        self.subscriptions.store(0, Ordering::Relaxed);
        self.unsubscriptions.store(0, Ordering::Relaxed);
    }
}

/// A point-in-time snapshot of WebSocket metrics.
#[derive(Debug, Clone)]
pub struct WsMetricsSnapshot {
    /// Total connections opened.
    pub connections_opened: u64,
    /// Total connections closed.
    pub connections_closed: u64,
    /// Active connections.
    pub active_connections: u64,
    /// Messages received.
    pub messages_received: u64,
    /// Messages sent.
    pub messages_sent: u64,
    /// Errors.
    pub errors: u64,
    /// Subscriptions.
    pub subscriptions: u64,
    /// Unsubscriptions.
    pub unsubscriptions: u64,
    /// Uptime.
    pub uptime: Duration,
    /// Messages per second.
    pub messages_per_second: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_new() {
        let metrics = WsMetrics::new();
        assert_eq!(metrics.connections_opened(), 0);
        assert_eq!(metrics.connections_closed(), 0);
        assert_eq!(metrics.active_connections(), 0);
    }

    #[test]
    fn test_metrics_default() {
        let metrics = WsMetrics::default();
        assert_eq!(metrics.connections_opened(), 0);
    }

    #[test]
    fn test_metrics_record_connection() {
        let metrics = WsMetrics::new();

        metrics.record_connection_opened();
        metrics.record_connection_opened();
        assert_eq!(metrics.connections_opened(), 2);
        assert_eq!(metrics.active_connections(), 2);

        metrics.record_connection_closed();
        assert_eq!(metrics.connections_closed(), 1);
        assert_eq!(metrics.active_connections(), 1);
    }

    #[test]
    fn test_metrics_record_messages() {
        let metrics = WsMetrics::new();

        metrics.record_message_received();
        metrics.record_message_received();
        metrics.record_message_sent();

        assert_eq!(metrics.messages_received(), 2);
        assert_eq!(metrics.messages_sent(), 1);
    }

    #[test]
    fn test_metrics_record_error() {
        let metrics = WsMetrics::new();

        metrics.record_error();
        assert_eq!(metrics.errors(), 1);
    }

    #[test]
    fn test_metrics_record_subscription() {
        let metrics = WsMetrics::new();

        metrics.record_subscription();
        metrics.record_subscription();
        metrics.record_unsubscription();

        assert_eq!(metrics.subscriptions(), 2);
        assert_eq!(metrics.unsubscriptions(), 1);
    }

    #[test]
    fn test_metrics_snapshot() {
        let metrics = WsMetrics::new();

        metrics.record_connection_opened();
        metrics.record_message_received();
        metrics.record_message_sent();

        let snapshot = metrics.snapshot();

        assert_eq!(snapshot.connections_opened, 1);
        assert_eq!(snapshot.active_connections, 1);
        assert_eq!(snapshot.messages_received, 1);
        assert_eq!(snapshot.messages_sent, 1);
    }

    #[test]
    fn test_metrics_reset() {
        let metrics = WsMetrics::new();

        metrics.record_connection_opened();
        metrics.record_message_received();
        metrics.record_error();

        metrics.reset();

        assert_eq!(metrics.connections_opened(), 0);
        assert_eq!(metrics.messages_received(), 0);
        assert_eq!(metrics.errors(), 0);
    }
}
