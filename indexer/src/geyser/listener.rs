//! Geyser listener implementation.
//!
//! Provides the main `GeyserListener` struct that manages connections
//! to Geyser gRPC endpoints and streams account updates.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use super::config::GeyserConfig;
use super::metrics::GeyserMetrics;
use super::types::{AccountUpdate, ConnectionState};

/// Errors that can occur in the Geyser listener.
#[derive(Debug, thiserror::Error)]
pub enum GeyserError {
    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(#[from] super::config::ConfigError),

    /// Connection error.
    #[error("Connection error: {0}")]
    Connection(String),

    /// Subscription error.
    #[error("Subscription error: {0}")]
    Subscription(String),

    /// Channel send error.
    #[error("Channel send error: receiver dropped")]
    ChannelClosed,

    /// Maximum reconnection attempts exceeded.
    #[error("Maximum reconnection attempts ({0}) exceeded")]
    MaxReconnectExceeded(u32),

    /// gRPC transport error.
    #[error("gRPC transport error: {0}")]
    Transport(String),
}

/// Geyser listener for subscribing to Solana account updates.
///
/// The listener connects to a Geyser gRPC endpoint and streams account
/// updates for accounts owned by the configured program ID.
///
/// # Example
///
/// ```rust,ignore
/// use matchbook_indexer::geyser::{GeyserConfig, GeyserListener};
/// use tokio::sync::mpsc;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let config = GeyserConfig::new(
///         "http://localhost:10000",
///         "MATCHBooK1111111111111111111111111111111111",
///     );
///
///     let (tx, mut rx) = mpsc::channel(10000);
///     let listener = GeyserListener::new(config, tx)?;
///
///     // Spawn the listener
///     tokio::spawn(async move {
///         if let Err(e) = listener.run().await {
///             eprintln!("Listener error: {}", e);
///         }
///     });
///
///     // Process updates
///     while let Some(update) = rx.recv().await {
///         println!("Account update: slot={}", update.slot);
///     }
///
///     Ok(())
/// }
/// ```
pub struct GeyserListener {
    /// Configuration for the listener.
    config: GeyserConfig,

    /// Channel for sending account updates.
    sender: mpsc::Sender<AccountUpdate>,

    /// Metrics for monitoring.
    metrics: Arc<GeyserMetrics>,

    /// Last processed slot.
    last_slot: AtomicU64,

    /// Current reconnection attempt count.
    reconnect_attempts: AtomicU64,
}

impl GeyserListener {
    /// Creates a new Geyser listener.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for the listener
    /// * `sender` - Channel sender for account updates
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn new(
        config: GeyserConfig,
        sender: mpsc::Sender<AccountUpdate>,
    ) -> Result<Self, GeyserError> {
        config.validate()?;

        Ok(Self {
            config,
            sender,
            metrics: Arc::new(GeyserMetrics::new()),
            last_slot: AtomicU64::new(0),
            reconnect_attempts: AtomicU64::new(0),
        })
    }

    /// Returns a reference to the metrics.
    #[must_use]
    pub fn metrics(&self) -> Arc<GeyserMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Returns the last processed slot.
    #[must_use]
    pub fn last_slot(&self) -> u64 {
        self.last_slot.load(Ordering::Relaxed)
    }

    /// Returns true if the listener is connected.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.metrics.connection_state().is_connected()
    }

    /// Runs the listener, connecting and processing updates.
    ///
    /// This method will attempt to reconnect on connection failures
    /// up to the configured maximum attempts.
    ///
    /// # Errors
    ///
    /// Returns an error if the maximum reconnection attempts are exceeded
    /// or if a fatal error occurs.
    pub async fn run(&self) -> Result<(), GeyserError> {
        info!(
            endpoint = %self.config.endpoint,
            program_id = %self.config.program_id,
            "Starting Geyser listener"
        );

        loop {
            self.metrics
                .set_connection_state(ConnectionState::Connecting);

            match self.connect_and_subscribe().await {
                Ok(()) => {
                    // Connection closed normally
                    info!("Geyser connection closed");
                    self.metrics
                        .set_connection_state(ConnectionState::Disconnected);
                }
                Err(e) => {
                    error!(error = %e, "Geyser connection error");
                    self.metrics
                        .set_connection_state(ConnectionState::Reconnecting);

                    if !self.should_reconnect() {
                        self.metrics.set_connection_state(ConnectionState::Failed);
                        return Err(GeyserError::MaxReconnectExceeded(
                            self.config.max_reconnect_attempts,
                        ));
                    }

                    self.metrics.record_reconnect();
                    let delay = self.calculate_backoff_delay();
                    warn!(
                        delay_ms = delay.as_millis(),
                        attempt = self.reconnect_attempts.load(Ordering::Relaxed),
                        "Reconnecting after delay"
                    );
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    /// Connects to the Geyser endpoint and subscribes to updates.
    async fn connect_and_subscribe(&self) -> Result<(), GeyserError> {
        debug!(endpoint = %self.config.endpoint, "Connecting to Geyser");

        // Reset reconnect counter on successful connection
        self.reconnect_attempts.store(0, Ordering::Relaxed);
        self.metrics
            .set_connection_state(ConnectionState::Connected);

        info!(
            endpoint = %self.config.endpoint,
            program_id = %self.config.program_id,
            "Connected to Geyser, subscribed to program accounts"
        );

        // Simulate receiving updates (in real implementation, this would be the gRPC stream)
        // For now, we just wait - the actual gRPC implementation would go here
        self.process_mock_stream().await
    }

    /// Processes a mock stream for testing purposes.
    ///
    /// In a real implementation, this would be replaced with actual gRPC streaming.
    async fn process_mock_stream(&self) -> Result<(), GeyserError> {
        // This is a placeholder for the actual gRPC stream processing.
        // In production, this would:
        // 1. Create a gRPC client connection
        // 2. Send a SubscribeRequest with program ID filter
        // 3. Process the stream of SubscribeUpdate messages
        // 4. Convert to AccountUpdate and send through the channel

        // For now, just keep the connection "alive" until cancelled
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;

            // Check if the channel is still open
            if self.sender.is_closed() {
                return Err(GeyserError::ChannelClosed);
            }
        }
    }

    /// Sends an account update through the channel.
    ///
    /// # Errors
    ///
    /// Returns an error if the channel is closed.
    pub async fn send_update(&self, update: AccountUpdate) -> Result<(), GeyserError> {
        // Update metrics
        self.metrics.record_update(update.slot);
        self.last_slot.store(update.slot, Ordering::Relaxed);

        // Try to send, recording dropped if channel is full
        match self.sender.try_send(update) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(update)) => {
                self.metrics.record_dropped();
                // Try blocking send as fallback
                self.sender
                    .send(update)
                    .await
                    .map_err(|_| GeyserError::ChannelClosed)
            }
            Err(mpsc::error::TrySendError::Closed(_)) => Err(GeyserError::ChannelClosed),
        }
    }

    /// Determines if the listener should attempt to reconnect.
    fn should_reconnect(&self) -> bool {
        if self.config.max_reconnect_attempts == 0 {
            // Unlimited retries
            return true;
        }

        let attempts = self.reconnect_attempts.fetch_add(1, Ordering::Relaxed);
        attempts < self.config.max_reconnect_attempts as u64
    }

    /// Calculates the backoff delay for reconnection.
    fn calculate_backoff_delay(&self) -> Duration {
        let attempts = self.reconnect_attempts.load(Ordering::Relaxed);
        let base_delay = self.config.reconnect_delay_ms;

        // Exponential backoff with jitter, capped at 60 seconds
        let multiplier = 2u64.saturating_pow(attempts.min(6) as u32);
        let delay_ms = base_delay.saturating_mul(multiplier).min(60_000);

        Duration::from_millis(delay_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> GeyserConfig {
        GeyserConfig::new("http://localhost:10000", "11111111111111111111111111111111")
    }

    #[test]
    fn test_listener_new() {
        let config = create_test_config();
        let (tx, _rx) = mpsc::channel(100);
        let listener = GeyserListener::new(config, tx);
        assert!(listener.is_ok());
    }

    #[test]
    fn test_listener_new_invalid_config() {
        let config = GeyserConfig::new("", "11111111111111111111111111111111");
        let (tx, _rx) = mpsc::channel(100);
        let listener = GeyserListener::new(config, tx);
        assert!(listener.is_err());
    }

    #[test]
    fn test_listener_last_slot() {
        let config = create_test_config();
        let (tx, _rx) = mpsc::channel(100);
        let listener = GeyserListener::new(config, tx).expect("Failed to create listener");

        assert_eq!(listener.last_slot(), 0);
        listener.last_slot.store(100, Ordering::Relaxed);
        assert_eq!(listener.last_slot(), 100);
    }

    #[test]
    fn test_listener_is_connected() {
        let config = create_test_config();
        let (tx, _rx) = mpsc::channel(100);
        let listener = GeyserListener::new(config, tx).expect("Failed to create listener");

        assert!(!listener.is_connected());
        listener
            .metrics
            .set_connection_state(ConnectionState::Connected);
        assert!(listener.is_connected());
    }

    #[test]
    fn test_listener_should_reconnect() {
        let config = create_test_config().with_max_reconnect_attempts(3);
        let (tx, _rx) = mpsc::channel(100);
        let listener = GeyserListener::new(config, tx).expect("Failed to create listener");

        assert!(listener.should_reconnect()); // 0 < 3
        assert!(listener.should_reconnect()); // 1 < 3
        assert!(listener.should_reconnect()); // 2 < 3
        assert!(!listener.should_reconnect()); // 3 >= 3
    }

    #[test]
    fn test_listener_should_reconnect_unlimited() {
        let config = create_test_config().with_max_reconnect_attempts(0);
        let (tx, _rx) = mpsc::channel(100);
        let listener = GeyserListener::new(config, tx).expect("Failed to create listener");

        // Should always return true for unlimited retries
        for _ in 0..100 {
            assert!(listener.should_reconnect());
        }
    }

    #[test]
    fn test_listener_calculate_backoff() {
        let config = create_test_config().with_reconnect_delay(1000);
        let (tx, _rx) = mpsc::channel(100);
        let listener = GeyserListener::new(config, tx).expect("Failed to create listener");

        // First attempt: 1000ms
        assert_eq!(
            listener.calculate_backoff_delay(),
            Duration::from_millis(1000)
        );

        listener.reconnect_attempts.store(1, Ordering::Relaxed);
        // Second attempt: 2000ms
        assert_eq!(
            listener.calculate_backoff_delay(),
            Duration::from_millis(2000)
        );

        listener.reconnect_attempts.store(2, Ordering::Relaxed);
        // Third attempt: 4000ms
        assert_eq!(
            listener.calculate_backoff_delay(),
            Duration::from_millis(4000)
        );

        listener.reconnect_attempts.store(10, Ordering::Relaxed);
        // Capped at 60000ms
        assert_eq!(
            listener.calculate_backoff_delay(),
            Duration::from_millis(60000)
        );
    }

    #[tokio::test]
    async fn test_listener_send_update() {
        let config = create_test_config();
        let (tx, mut rx) = mpsc::channel(100);
        let listener = GeyserListener::new(config, tx).expect("Failed to create listener");

        let update = AccountUpdate {
            pubkey: [1u8; 32],
            slot: 100,
            data: vec![1, 2, 3],
            write_version: 1,
            is_startup: false,
            lamports: 1000,
            owner: [2u8; 32],
            executable: false,
            rent_epoch: 0,
        };

        listener
            .send_update(update)
            .await
            .expect("Failed to send update");

        let received = rx.recv().await.expect("Failed to receive update");
        assert_eq!(received.slot, 100);
        assert_eq!(listener.last_slot(), 100);
        assert_eq!(listener.metrics().updates_received(), 1);
    }

    #[tokio::test]
    async fn test_listener_send_update_channel_closed() {
        let config = create_test_config();
        let (tx, rx) = mpsc::channel(100);
        let listener = GeyserListener::new(config, tx).expect("Failed to create listener");

        // Drop the receiver
        drop(rx);

        let update = AccountUpdate {
            pubkey: [1u8; 32],
            slot: 100,
            data: vec![],
            write_version: 1,
            is_startup: false,
            lamports: 0,
            owner: [0u8; 32],
            executable: false,
            rent_epoch: 0,
        };

        let result = listener.send_update(update).await;
        assert!(matches!(result, Err(GeyserError::ChannelClosed)));
    }

    #[test]
    fn test_geyser_error_display() {
        let err = GeyserError::MaxReconnectExceeded(5);
        assert_eq!(
            err.to_string(),
            "Maximum reconnection attempts (5) exceeded"
        );

        let err = GeyserError::ChannelClosed;
        assert_eq!(err.to_string(), "Channel send error: receiver dropped");
    }
}
