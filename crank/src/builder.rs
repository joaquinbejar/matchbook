//! Transaction building for the crank service.
//!
//! Builds MatchOrders and ConsumeEvents transactions.

use super::detector::CrossInfo;

/// Transaction builder for crank operations.
#[derive(Debug, Clone)]
pub struct TransactionBuilder {
    /// Program ID for the Matchbook program.
    program_id: [u8; 32],
}

/// A built transaction ready for submission.
#[derive(Debug, Clone)]
pub struct BuiltTransaction {
    /// Market address.
    pub market: [u8; 32],

    /// Serialized transaction data (placeholder).
    pub data: Vec<u8>,

    /// Number of matches this transaction will execute.
    pub match_count: u32,

    /// Priority fee in lamports.
    pub priority_fee: u64,

    /// Estimated compute units.
    pub compute_units: u32,
}

impl TransactionBuilder {
    /// Creates a new transaction builder.
    #[must_use]
    pub fn new(program_id: [u8; 32]) -> Self {
        Self { program_id }
    }

    /// Returns the program ID.
    #[must_use]
    pub const fn program_id(&self) -> &[u8; 32] {
        &self.program_id
    }

    /// Builds a MatchOrders transaction for the given cross.
    ///
    /// # Arguments
    ///
    /// * `cross` - Cross information
    /// * `max_matches` - Maximum matches to execute
    /// * `priority_fee` - Priority fee in lamports
    #[must_use]
    pub fn build_match_orders(
        &self,
        cross: &CrossInfo,
        max_matches: u32,
        priority_fee: u64,
    ) -> BuiltTransaction {
        let match_count = cross.estimated_matches.min(max_matches);

        // Estimate compute units (placeholder)
        let compute_units = 50_000_u32.saturating_add(match_count.saturating_mul(10_000));

        BuiltTransaction {
            market: cross.market,
            data: self.serialize_match_orders(&cross.market, match_count),
            match_count,
            priority_fee,
            compute_units,
        }
    }

    /// Builds a ConsumeEvents transaction.
    ///
    /// # Arguments
    ///
    /// * `market` - Market address
    /// * `limit` - Maximum events to consume
    /// * `priority_fee` - Priority fee in lamports
    #[must_use]
    pub fn build_consume_events(
        &self,
        market: &[u8; 32],
        limit: u32,
        priority_fee: u64,
    ) -> BuiltTransaction {
        // Estimate compute units (placeholder)
        let compute_units = 30_000_u32.saturating_add(limit.saturating_mul(5_000));

        BuiltTransaction {
            market: *market,
            data: self.serialize_consume_events(market, limit),
            match_count: 0,
            priority_fee,
            compute_units,
        }
    }

    /// Builds a bundled transaction with MatchOrders and ConsumeEvents.
    ///
    /// # Arguments
    ///
    /// * `cross` - Cross information
    /// * `max_matches` - Maximum matches to execute
    /// * `consume_limit` - Maximum events to consume
    /// * `priority_fee` - Priority fee in lamports
    #[must_use]
    pub fn build_bundled(
        &self,
        cross: &CrossInfo,
        max_matches: u32,
        consume_limit: u32,
        priority_fee: u64,
    ) -> BuiltTransaction {
        let match_count = cross.estimated_matches.min(max_matches);

        // Estimate compute units for both instructions
        let match_cu = 50_000_u32.saturating_add(match_count.saturating_mul(10_000));
        let consume_cu = 30_000_u32.saturating_add(consume_limit.saturating_mul(5_000));
        let compute_units = match_cu.saturating_add(consume_cu);

        let mut data = self.serialize_match_orders(&cross.market, match_count);
        data.extend(self.serialize_consume_events(&cross.market, consume_limit));

        BuiltTransaction {
            market: cross.market,
            data,
            match_count,
            priority_fee,
            compute_units,
        }
    }

    /// Serializes a MatchOrders instruction (placeholder).
    fn serialize_match_orders(&self, market: &[u8; 32], limit: u32) -> Vec<u8> {
        let mut data = Vec::with_capacity(40);
        data.push(0x01); // Instruction discriminator
        data.extend_from_slice(market);
        data.extend_from_slice(&limit.to_le_bytes());
        data
    }

    /// Serializes a ConsumeEvents instruction (placeholder).
    fn serialize_consume_events(&self, market: &[u8; 32], limit: u32) -> Vec<u8> {
        let mut data = Vec::with_capacity(40);
        data.push(0x02); // Instruction discriminator
        data.extend_from_slice(market);
        data.extend_from_slice(&limit.to_le_bytes());
        data
    }
}

impl BuiltTransaction {
    /// Returns the market address as base58.
    #[must_use]
    pub fn market_string(&self) -> String {
        bs58::encode(&self.market).into_string()
    }

    /// Returns the estimated transaction cost in lamports.
    #[must_use]
    pub fn estimated_cost(&self) -> u64 {
        // Base fee + priority fee
        5000_u64.saturating_add(self.priority_fee)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_cross_info() -> CrossInfo {
        CrossInfo {
            market: [1u8; 32],
            best_bid: 1100,
            best_ask: 1000,
            estimated_matches: 5,
            matchable_quantity: 500,
        }
    }

    #[test]
    fn test_builder_new() {
        let program_id = [0u8; 32];
        let builder = TransactionBuilder::new(program_id);
        assert_eq!(builder.program_id(), &program_id);
    }

    #[test]
    fn test_build_match_orders() {
        let builder = TransactionBuilder::new([0u8; 32]);
        let cross = create_cross_info();

        let tx = builder.build_match_orders(&cross, 10, 1000);

        assert_eq!(tx.market, cross.market);
        assert_eq!(tx.match_count, 5); // min(5, 10)
        assert_eq!(tx.priority_fee, 1000);
        assert!(tx.compute_units > 0);
        assert!(!tx.data.is_empty());
    }

    #[test]
    fn test_build_match_orders_limited() {
        let builder = TransactionBuilder::new([0u8; 32]);
        let cross = create_cross_info();

        let tx = builder.build_match_orders(&cross, 3, 1000);

        assert_eq!(tx.match_count, 3); // min(5, 3)
    }

    #[test]
    fn test_build_consume_events() {
        let builder = TransactionBuilder::new([0u8; 32]);
        let market = [1u8; 32];

        let tx = builder.build_consume_events(&market, 10, 500);

        assert_eq!(tx.market, market);
        assert_eq!(tx.match_count, 0);
        assert_eq!(tx.priority_fee, 500);
        assert!(tx.compute_units > 0);
    }

    #[test]
    fn test_build_bundled() {
        let builder = TransactionBuilder::new([0u8; 32]);
        let cross = create_cross_info();

        let tx = builder.build_bundled(&cross, 10, 20, 2000);

        assert_eq!(tx.market, cross.market);
        assert_eq!(tx.match_count, 5);
        assert_eq!(tx.priority_fee, 2000);
        // Bundled should have more compute units
        assert!(tx.compute_units > 50_000);
    }

    #[test]
    fn test_built_transaction_market_string() {
        let tx = BuiltTransaction {
            market: [1u8; 32],
            data: vec![],
            match_count: 1,
            priority_fee: 1000,
            compute_units: 50_000,
        };

        let s = tx.market_string();
        assert!(!s.is_empty());
    }

    #[test]
    fn test_built_transaction_estimated_cost() {
        let tx = BuiltTransaction {
            market: [1u8; 32],
            data: vec![],
            match_count: 1,
            priority_fee: 1000,
            compute_units: 50_000,
        };

        assert_eq!(tx.estimated_cost(), 6000); // 5000 + 1000
    }
}
