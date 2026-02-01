//! Cross detection logic for the crank service.
//!
//! Monitors order books for crossing conditions (bid >= ask).

use std::sync::Arc;

use matchbook_indexer::BookBuilder;
use tokio::sync::RwLock;

/// Information about a detected cross.
#[derive(Debug, Clone)]
pub struct CrossInfo {
    /// Market address.
    pub market: [u8; 32],

    /// Best bid price.
    pub best_bid: u64,

    /// Best ask price.
    pub best_ask: u64,

    /// Estimated number of matches possible.
    pub estimated_matches: u32,

    /// Total quantity that can be matched.
    pub matchable_quantity: u64,
}

impl CrossInfo {
    /// Returns the spread (negative when crossed).
    #[must_use]
    pub fn spread(&self) -> i64 {
        (self.best_ask as i64).saturating_sub(self.best_bid as i64)
    }

    /// Returns true if there is a cross (bid >= ask).
    #[must_use]
    pub const fn is_crossed(&self) -> bool {
        self.best_bid >= self.best_ask
    }

    /// Returns the market address as base58.
    #[must_use]
    pub fn market_string(&self) -> String {
        bs58::encode(&self.market).into_string()
    }
}

/// Detector for crossing orders in order books.
pub struct CrossDetector {
    /// Book builder for order book data.
    book_builder: Arc<RwLock<BookBuilder>>,
}

impl CrossDetector {
    /// Creates a new cross detector.
    #[must_use]
    pub fn new(book_builder: Arc<RwLock<BookBuilder>>) -> Self {
        Self { book_builder }
    }

    /// Detects if there is a cross in the given market.
    ///
    /// Returns `Some(CrossInfo)` if bid >= ask, `None` otherwise.
    pub async fn detect_cross(&self, market: &[u8; 32]) -> Option<CrossInfo> {
        let book_builder = self.book_builder.read().await;

        // Get order book snapshot
        let snapshot = book_builder.get_snapshot(market, 1)?;

        // Extract best bid and ask from snapshot
        match snapshot {
            matchbook_indexer::book::BookUpdate::Snapshot { bids, asks, .. } => {
                let best_bid = bids.first().map(|l| l.price)?;
                let best_ask = asks.first().map(|l| l.price)?;

                // Check for cross
                if best_bid >= best_ask {
                    // Estimate matches by looking at overlapping levels
                    let (estimated_matches, matchable_quantity) =
                        self.estimate_matches(&bids, &asks);

                    Some(CrossInfo {
                        market: *market,
                        best_bid,
                        best_ask,
                        estimated_matches,
                        matchable_quantity,
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Estimates the number of matches and matchable quantity.
    fn estimate_matches(
        &self,
        bids: &[matchbook_indexer::book::PriceLevel],
        asks: &[matchbook_indexer::book::PriceLevel],
    ) -> (u32, u64) {
        let mut matches = 0u32;
        let mut quantity = 0u64;

        for bid in bids {
            for ask in asks {
                if bid.price >= ask.price {
                    matches = matches.saturating_add(1);
                    quantity = quantity.saturating_add(bid.quantity.min(ask.quantity));
                }
            }
        }

        (matches, quantity)
    }

    /// Checks multiple markets for crosses.
    pub async fn detect_crosses(&self, markets: &[[u8; 32]]) -> Vec<CrossInfo> {
        let mut crosses = Vec::new();

        for market in markets {
            if let Some(cross) = self.detect_cross(market).await {
                crosses.push(cross);
            }
        }

        crosses
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_info_spread() {
        let info = CrossInfo {
            market: [0u8; 32],
            best_bid: 1000,
            best_ask: 1100,
            estimated_matches: 0,
            matchable_quantity: 0,
        };

        assert_eq!(info.spread(), 100);
    }

    #[test]
    fn test_cross_info_spread_crossed() {
        let info = CrossInfo {
            market: [0u8; 32],
            best_bid: 1100,
            best_ask: 1000,
            estimated_matches: 1,
            matchable_quantity: 100,
        };

        assert_eq!(info.spread(), -100);
    }

    #[test]
    fn test_cross_info_is_crossed() {
        let crossed = CrossInfo {
            market: [0u8; 32],
            best_bid: 1100,
            best_ask: 1000,
            estimated_matches: 1,
            matchable_quantity: 100,
        };

        let not_crossed = CrossInfo {
            market: [0u8; 32],
            best_bid: 1000,
            best_ask: 1100,
            estimated_matches: 0,
            matchable_quantity: 0,
        };

        assert!(crossed.is_crossed());
        assert!(!not_crossed.is_crossed());
    }

    #[test]
    fn test_cross_info_market_string() {
        let info = CrossInfo {
            market: [1u8; 32],
            best_bid: 1000,
            best_ask: 1100,
            estimated_matches: 0,
            matchable_quantity: 0,
        };

        let s = info.market_string();
        assert!(!s.is_empty());
    }

    #[tokio::test]
    async fn test_detector_new() {
        let book_builder = Arc::new(RwLock::new(BookBuilder::new()));
        let detector = CrossDetector::new(book_builder);
        assert!(detector.detect_cross(&[0u8; 32]).await.is_none());
    }

    #[tokio::test]
    async fn test_detector_detect_crosses_empty() {
        let book_builder = Arc::new(RwLock::new(BookBuilder::new()));
        let detector = CrossDetector::new(book_builder);
        let crosses = detector.detect_crosses(&[]).await;
        assert!(crosses.is_empty());
    }
}
