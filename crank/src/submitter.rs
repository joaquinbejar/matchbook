//! Transaction submission for the crank service.
//!
//! Handles sending transactions with retries and confirmation tracking.

use std::time::Duration;

use super::builder::BuiltTransaction;
use super::metrics::CrankMetrics;

/// Configuration for the transaction submitter.
#[derive(Debug, Clone)]
pub struct SubmitterConfig {
    /// Maximum retries for failed transactions.
    pub max_retries: u32,

    /// Initial backoff in milliseconds.
    pub initial_backoff_ms: u64,

    /// Backoff multiplier.
    pub backoff_multiplier: f64,

    /// Maximum backoff in milliseconds.
    pub max_backoff_ms: u64,

    /// Confirmation timeout in milliseconds.
    pub confirmation_timeout_ms: u64,
}

impl Default for SubmitterConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            max_backoff_ms: 10_000,
            confirmation_timeout_ms: 30_000,
        }
    }
}

/// Result of a transaction submission.
#[derive(Debug, Clone)]
pub enum SubmitResult {
    /// Transaction confirmed successfully.
    Confirmed {
        /// Transaction signature.
        signature: String,
        /// Slot confirmed.
        slot: u64,
    },

    /// Transaction failed.
    Failed {
        /// Error message.
        error: String,
        /// Number of retries attempted.
        retries: u32,
    },

    /// Transaction dropped (not confirmed in time).
    Dropped {
        /// Transaction signature.
        signature: String,
    },
}

impl SubmitResult {
    /// Returns true if the transaction was confirmed.
    #[must_use]
    pub const fn is_confirmed(&self) -> bool {
        matches!(self, Self::Confirmed { .. })
    }

    /// Returns true if the transaction failed.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Returns the signature if available.
    #[must_use]
    pub fn signature(&self) -> Option<&str> {
        match self {
            Self::Confirmed { signature, .. } | Self::Dropped { signature } => Some(signature),
            Self::Failed { .. } => None,
        }
    }
}

/// Transaction submitter for the crank service.
pub struct TransactionSubmitter {
    /// Configuration.
    config: SubmitterConfig,

    /// Metrics.
    metrics: Option<std::sync::Arc<CrankMetrics>>,
}

impl TransactionSubmitter {
    /// Creates a new transaction submitter.
    #[must_use]
    pub fn new(config: SubmitterConfig) -> Self {
        Self {
            config,
            metrics: None,
        }
    }

    /// Creates a submitter with metrics.
    #[must_use]
    pub fn with_metrics(config: SubmitterConfig, metrics: std::sync::Arc<CrankMetrics>) -> Self {
        Self {
            config,
            metrics: Some(metrics),
        }
    }

    /// Returns the configuration.
    #[must_use]
    pub const fn config(&self) -> &SubmitterConfig {
        &self.config
    }

    /// Submits a transaction with retries.
    ///
    /// This is a placeholder implementation that simulates submission.
    pub async fn submit(&self, tx: &BuiltTransaction) -> SubmitResult {
        let mut retries = 0;
        let mut backoff = self.config.initial_backoff_ms;

        while retries < self.config.max_retries {
            match self.try_submit(tx).await {
                Ok(signature) => {
                    // Wait for confirmation
                    match self.wait_for_confirmation(&signature).await {
                        Ok(slot) => {
                            if let Some(metrics) = &self.metrics {
                                metrics.record_success(tx.match_count);
                            }
                            return SubmitResult::Confirmed { signature, slot };
                        }
                        Err(_) => {
                            return SubmitResult::Dropped { signature };
                        }
                    }
                }
                Err(e) => {
                    retries += 1;
                    if let Some(metrics) = &self.metrics {
                        metrics.record_failure();
                    }

                    if retries >= self.config.max_retries {
                        return SubmitResult::Failed { error: e, retries };
                    }

                    // Exponential backoff
                    tokio::time::sleep(Duration::from_millis(backoff)).await;
                    backoff = ((backoff as f64) * self.config.backoff_multiplier) as u64;
                    backoff = backoff.min(self.config.max_backoff_ms);
                }
            }
        }

        SubmitResult::Failed {
            error: "max retries exceeded".to_string(),
            retries,
        }
    }

    /// Attempts to submit a transaction once.
    async fn try_submit(&self, _tx: &BuiltTransaction) -> Result<String, String> {
        // Placeholder: In real implementation, this would send to RPC
        // For now, simulate success
        Ok(bs58::encode(&[0u8; 64]).into_string())
    }

    /// Waits for transaction confirmation.
    async fn wait_for_confirmation(&self, _signature: &str) -> Result<u64, String> {
        // Placeholder: In real implementation, this would poll for confirmation
        // For now, simulate success
        Ok(12345)
    }

    /// Calculates the backoff duration for a given retry count.
    #[must_use]
    pub fn calculate_backoff(&self, retry: u32) -> Duration {
        let backoff = self.config.initial_backoff_ms as f64
            * self.config.backoff_multiplier.powi(retry as i32);
        let backoff = (backoff as u64).min(self.config.max_backoff_ms);
        Duration::from_millis(backoff)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_submitter_config_default() {
        let config = SubmitterConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_backoff_ms, 100);
        assert_eq!(config.backoff_multiplier, 2.0);
    }

    #[test]
    fn test_submit_result_is_confirmed() {
        let confirmed = SubmitResult::Confirmed {
            signature: "sig".to_string(),
            slot: 100,
        };
        let failed = SubmitResult::Failed {
            error: "error".to_string(),
            retries: 3,
        };
        let dropped = SubmitResult::Dropped {
            signature: "sig".to_string(),
        };

        assert!(confirmed.is_confirmed());
        assert!(!failed.is_confirmed());
        assert!(!dropped.is_confirmed());
    }

    #[test]
    fn test_submit_result_is_failed() {
        let confirmed = SubmitResult::Confirmed {
            signature: "sig".to_string(),
            slot: 100,
        };
        let failed = SubmitResult::Failed {
            error: "error".to_string(),
            retries: 3,
        };

        assert!(!confirmed.is_failed());
        assert!(failed.is_failed());
    }

    #[test]
    fn test_submit_result_signature() {
        let confirmed = SubmitResult::Confirmed {
            signature: "sig1".to_string(),
            slot: 100,
        };
        let failed = SubmitResult::Failed {
            error: "error".to_string(),
            retries: 3,
        };
        let dropped = SubmitResult::Dropped {
            signature: "sig2".to_string(),
        };

        assert_eq!(confirmed.signature(), Some("sig1"));
        assert_eq!(failed.signature(), None);
        assert_eq!(dropped.signature(), Some("sig2"));
    }

    #[test]
    fn test_submitter_new() {
        let config = SubmitterConfig::default();
        let submitter = TransactionSubmitter::new(config.clone());
        assert_eq!(submitter.config().max_retries, config.max_retries);
    }

    #[test]
    fn test_submitter_calculate_backoff() {
        let config = SubmitterConfig {
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            max_backoff_ms: 10_000,
            ..Default::default()
        };
        let submitter = TransactionSubmitter::new(config);

        assert_eq!(submitter.calculate_backoff(0), Duration::from_millis(100));
        assert_eq!(submitter.calculate_backoff(1), Duration::from_millis(200));
        assert_eq!(submitter.calculate_backoff(2), Duration::from_millis(400));
        assert_eq!(submitter.calculate_backoff(3), Duration::from_millis(800));
    }

    #[test]
    fn test_submitter_calculate_backoff_max() {
        let config = SubmitterConfig {
            initial_backoff_ms: 1000,
            backoff_multiplier: 10.0,
            max_backoff_ms: 5000,
            ..Default::default()
        };
        let submitter = TransactionSubmitter::new(config);

        // Should be capped at max
        assert_eq!(submitter.calculate_backoff(2), Duration::from_millis(5000));
    }

    #[tokio::test]
    async fn test_submitter_submit() {
        let config = SubmitterConfig::default();
        let submitter = TransactionSubmitter::new(config);

        let tx = BuiltTransaction {
            market: [1u8; 32],
            data: vec![],
            match_count: 1,
            priority_fee: 1000,
            compute_units: 50_000,
        };

        let result = submitter.submit(&tx).await;
        assert!(result.is_confirmed());
    }
}
