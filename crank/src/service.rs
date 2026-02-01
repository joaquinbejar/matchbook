//! Main crank service.
//!
//! Orchestrates cross detection, transaction building, and submission.

use std::sync::Arc;
use std::time::Duration;

use matchbook_indexer::BookBuilder;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::builder::TransactionBuilder;
use super::config::CrankConfig;
use super::detector::CrossDetector;
use super::metrics::CrankMetrics;
use super::submitter::{SubmitResult, SubmitterConfig, TransactionSubmitter};

/// The main crank service.
pub struct CrankService {
    /// Configuration.
    config: CrankConfig,

    /// Cross detector.
    detector: CrossDetector,

    /// Transaction builder.
    builder: TransactionBuilder,

    /// Transaction submitter.
    submitter: TransactionSubmitter,

    /// Metrics.
    metrics: Arc<CrankMetrics>,

    /// Parsed market addresses.
    markets: Vec<[u8; 32]>,

    /// Whether the service is running.
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl CrankService {
    /// Creates a new crank service.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn new(
        config: CrankConfig,
        book_builder: Arc<RwLock<BookBuilder>>,
        program_id: [u8; 32],
    ) -> Result<Self, super::config::ConfigError> {
        config.validate()?;
        let markets = config.parse_markets()?;

        let metrics = Arc::new(CrankMetrics::new());

        let submitter_config = SubmitterConfig {
            max_retries: config.max_retries,
            initial_backoff_ms: 100,
            backoff_multiplier: config.backoff_multiplier,
            max_backoff_ms: config.max_backoff_ms,
            confirmation_timeout_ms: 30_000,
        };

        Ok(Self {
            config,
            detector: CrossDetector::new(book_builder),
            builder: TransactionBuilder::new(program_id),
            submitter: TransactionSubmitter::with_metrics(submitter_config, Arc::clone(&metrics)),
            metrics,
            markets,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    /// Returns the metrics.
    #[must_use]
    pub fn metrics(&self) -> Arc<CrankMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Returns the configuration.
    #[must_use]
    pub const fn config(&self) -> &CrankConfig {
        &self.config
    }

    /// Returns true if the service is running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Stops the service.
    pub fn stop(&self) {
        self.running
            .store(false, std::sync::atomic::Ordering::Relaxed);
        info!("Crank service stop requested");
    }

    /// Runs the crank service.
    ///
    /// This will poll for crosses and submit transactions until stopped.
    pub async fn run(&self) {
        self.running
            .store(true, std::sync::atomic::Ordering::Relaxed);

        info!("Crank service started with {} markets", self.markets.len());

        let poll_interval = Duration::from_millis(self.config.poll_interval_ms);

        while self.is_running() {
            self.metrics.record_poll();

            // Detect crosses in all markets
            let crosses = self.detector.detect_crosses(&self.markets).await;

            for cross in crosses {
                self.metrics.record_cross();
                debug!(
                    "Cross detected in market {}: bid={} ask={}",
                    cross.market_string(),
                    cross.best_bid,
                    cross.best_ask
                );

                // Calculate priority fee
                let priority_fee = self.calculate_priority_fee(&cross);

                // Check profitability
                if self.config.enable_profitability_check
                    && !self.is_profitable(&cross, priority_fee)
                {
                    debug!("Skipping unprofitable cross");
                    continue;
                }

                // Build and submit transaction
                let tx = self.builder.build_match_orders(
                    &cross,
                    self.config.max_matches_per_tx,
                    priority_fee,
                );

                self.metrics.record_submission(priority_fee);

                match self.submitter.submit(&tx).await {
                    SubmitResult::Confirmed { signature, slot } => {
                        info!(
                            "Transaction confirmed: {} (slot {}), {} matches",
                            signature, slot, tx.match_count
                        );
                    }
                    SubmitResult::Failed { error, retries } => {
                        warn!("Transaction failed after {} retries: {}", retries, error);
                    }
                    SubmitResult::Dropped { signature } => {
                        warn!("Transaction dropped: {}", signature);
                        self.metrics.record_dropped();
                    }
                }
            }

            tokio::time::sleep(poll_interval).await;
        }

        info!("Crank service stopped");
    }

    /// Runs a single poll cycle.
    ///
    /// Returns the number of transactions submitted.
    pub async fn poll_once(&self) -> u32 {
        self.metrics.record_poll();

        let crosses = self.detector.detect_crosses(&self.markets).await;
        let mut submitted = 0;

        for cross in crosses {
            self.metrics.record_cross();

            let priority_fee = self.calculate_priority_fee(&cross);

            if self.config.enable_profitability_check && !self.is_profitable(&cross, priority_fee) {
                continue;
            }

            let tx = self.builder.build_match_orders(
                &cross,
                self.config.max_matches_per_tx,
                priority_fee,
            );

            self.metrics.record_submission(priority_fee);
            let _ = self.submitter.submit(&tx).await;
            submitted += 1;
        }

        submitted
    }

    /// Calculates the priority fee for a cross.
    fn calculate_priority_fee(&self, cross: &super::detector::CrossInfo) -> u64 {
        // Simple linear scaling based on estimated matches
        let base = self.config.min_priority_fee;
        let per_match = (self.config.max_priority_fee - self.config.min_priority_fee)
            / self.config.max_matches_per_tx.max(1) as u64;

        let fee = base.saturating_add(per_match.saturating_mul(cross.estimated_matches as u64));
        fee.min(self.config.max_priority_fee)
    }

    /// Checks if a cross is profitable to execute.
    fn is_profitable(&self, cross: &super::detector::CrossInfo, priority_fee: u64) -> bool {
        // Estimate fees earned from matching
        // This is a placeholder - real implementation would calculate actual fees
        let estimated_fees = cross.matchable_quantity / 10_000; // 0.01% fee

        // Transaction cost
        let tx_cost = 5000_u64.saturating_add(priority_fee);

        estimated_fees > tx_cost.saturating_add(self.config.min_profit_threshold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_config() -> CrankConfig {
        CrankConfig::with_markets(vec!["11111111111111111111111111111111".to_string()])
    }

    #[test]
    fn test_service_new() {
        let config = create_config();
        let book_builder = Arc::new(RwLock::new(BookBuilder::new()));
        let program_id = [0u8; 32];

        let service = CrankService::new(config, book_builder, program_id);
        assert!(service.is_ok());
    }

    #[test]
    fn test_service_new_invalid_config() {
        let mut config = create_config();
        config.min_priority_fee = 100_000;
        config.max_priority_fee = 1000;

        let book_builder = Arc::new(RwLock::new(BookBuilder::new()));
        let program_id = [0u8; 32];

        let service = CrankService::new(config, book_builder, program_id);
        assert!(service.is_err());
    }

    #[test]
    fn test_service_metrics() {
        let config = create_config();
        let book_builder = Arc::new(RwLock::new(BookBuilder::new()));
        let program_id = [0u8; 32];

        let service = CrankService::new(config, book_builder, program_id).expect("service");
        let metrics = service.metrics();

        assert_eq!(metrics.matches_executed(), 0);
    }

    #[test]
    fn test_service_is_running() {
        let config = create_config();
        let book_builder = Arc::new(RwLock::new(BookBuilder::new()));
        let program_id = [0u8; 32];

        let service = CrankService::new(config, book_builder, program_id).expect("service");

        assert!(!service.is_running());
    }

    #[test]
    fn test_service_stop() {
        let config = create_config();
        let book_builder = Arc::new(RwLock::new(BookBuilder::new()));
        let program_id = [0u8; 32];

        let service = CrankService::new(config, book_builder, program_id).expect("service");
        service.stop();

        assert!(!service.is_running());
    }

    #[tokio::test]
    async fn test_service_poll_once() {
        let config = create_config();
        let book_builder = Arc::new(RwLock::new(BookBuilder::new()));
        let program_id = [0u8; 32];

        let service = CrankService::new(config, book_builder, program_id).expect("service");
        let submitted = service.poll_once().await;

        // No crosses in empty book
        assert_eq!(submitted, 0);
        assert_eq!(service.metrics().poll_cycles(), 1);
    }

    #[test]
    fn test_service_calculate_priority_fee() {
        let config = CrankConfig {
            min_priority_fee: 1000,
            max_priority_fee: 10_000,
            max_matches_per_tx: 10,
            ..create_config()
        };
        let book_builder = Arc::new(RwLock::new(BookBuilder::new()));
        let program_id = [0u8; 32];

        let service = CrankService::new(config, book_builder, program_id).expect("service");

        let cross = super::super::detector::CrossInfo {
            market: [0u8; 32],
            best_bid: 1100,
            best_ask: 1000,
            estimated_matches: 5,
            matchable_quantity: 500,
        };

        let fee = service.calculate_priority_fee(&cross);
        assert!(fee >= 1000);
        assert!(fee <= 10_000);
    }
}
