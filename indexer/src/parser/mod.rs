//! Account parser module for Matchbook indexer.
//!
//! This module provides components for deserializing raw Solana account data
//! into structured Rust types.
//!
//! # Components
//!
//! - [`types`]: Parsed account types and errors
//! - [`discriminators`]: Account discriminator constants
//! - [`metrics`]: Parser metrics tracking
//! - [`AccountParser`]: Main parser implementation

pub mod discriminators;
pub mod metrics;
pub mod types;

use std::sync::Arc;

pub use discriminators::Discriminator;
pub use metrics::ParserMetrics;
pub use types::{
    EventType, ParseError, ParsedAccount, ParsedEvent, ParsedMarket, ParsedOpenOrders, ParsedOrder,
    Side,
};

/// Account parser for deserializing on-chain state.
///
/// Transforms raw account data into structured domain types.
///
/// # Example
///
/// ```rust,ignore
/// use matchbook_indexer::parser::AccountParser;
///
/// let parser = AccountParser::new();
/// let account_data: &[u8] = &[/* raw account data */];
///
/// match parser.parse(account_data) {
///     Ok(parsed) => println!("Parsed: {:?}", parsed),
///     Err(e) => eprintln!("Parse error: {}", e),
/// }
/// ```
pub struct AccountParser {
    /// Metrics for tracking parse operations.
    metrics: Arc<ParserMetrics>,
}

impl Default for AccountParser {
    fn default() -> Self {
        Self::new()
    }
}

impl AccountParser {
    /// Creates a new account parser.
    #[must_use]
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(ParserMetrics::new()),
        }
    }

    /// Returns a reference to the parser metrics.
    #[must_use]
    pub fn metrics(&self) -> Arc<ParserMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Parses raw account data into a structured type.
    ///
    /// Determines the account type from the discriminator and delegates
    /// to the appropriate parser.
    ///
    /// # Arguments
    ///
    /// * `data` - Raw account data bytes
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed or the account type
    /// is not recognized.
    pub fn parse(&self, data: &[u8]) -> Result<ParsedAccount, ParseError> {
        let start = std::time::Instant::now();

        let result = self.parse_inner(data);

        // Record metrics
        let elapsed = start.elapsed();
        self.metrics.record_parse(elapsed, result.is_ok());

        result
    }

    /// Internal parse implementation.
    fn parse_inner(&self, data: &[u8]) -> Result<ParsedAccount, ParseError> {
        if data.len() < 8 {
            return Err(ParseError::DataTooShort {
                expected: 8,
                actual: data.len(),
            });
        }

        let discriminator: [u8; 8] = data[0..8]
            .try_into()
            .map_err(|_| ParseError::InvalidDiscriminator)?;

        match Discriminator::from_bytes(&discriminator) {
            Some(Discriminator::Market) => {
                let market = self.parse_market(data)?;
                Ok(ParsedAccount::Market(market))
            }
            Some(Discriminator::OrderBookSide) => {
                let orders = self.parse_orderbook(data)?;
                Ok(ParsedAccount::OrderBookSide { orders })
            }
            Some(Discriminator::EventQueue) => {
                let events = self.parse_event_queue(data)?;
                Ok(ParsedAccount::EventQueue { events })
            }
            Some(Discriminator::OpenOrders) => {
                let open_orders = self.parse_open_orders(data)?;
                Ok(ParsedAccount::OpenOrders(open_orders))
            }
            None => Ok(ParsedAccount::Unknown { discriminator }),
        }
    }

    /// Parses a Market account.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed.
    pub fn parse_market(&self, data: &[u8]) -> Result<ParsedMarket, ParseError> {
        // Market account layout (after 8-byte discriminator):
        // - authority: [u8; 32]
        // - base_mint: [u8; 32]
        // - quote_mint: [u8; 32]
        // - bids: [u8; 32]
        // - asks: [u8; 32]
        // - event_queue: [u8; 32]
        // - base_vault: [u8; 32]
        // - quote_vault: [u8; 32]
        // - base_lot_size: u64
        // - quote_lot_size: u64
        // - tick_size: u64
        // - min_order_size: u64
        // - taker_fee_bps: u16
        // - maker_fee_bps: i16
        // - status: u8
        // - seq_num: u64
        // - bump: u8

        const MIN_SIZE: usize = 8 + 32 * 8 + 8 * 4 + 2 + 2 + 1 + 8 + 1; // 298 bytes

        if data.len() < MIN_SIZE {
            return Err(ParseError::DataTooShort {
                expected: MIN_SIZE,
                actual: data.len(),
            });
        }

        let mut offset = 8; // Skip discriminator

        // Skip authority
        offset += 32;

        let base_mint: [u8; 32] = data[offset..offset + 32]
            .try_into()
            .map_err(|_| ParseError::InvalidData("base_mint".to_string()))?;
        offset += 32;

        let quote_mint: [u8; 32] = data[offset..offset + 32]
            .try_into()
            .map_err(|_| ParseError::InvalidData("quote_mint".to_string()))?;
        offset += 32;

        // Skip bids, asks, event_queue, base_vault, quote_vault
        offset += 32 * 5;

        let base_lot_size = u64::from_le_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| ParseError::InvalidData("base_lot_size".to_string()))?,
        );
        offset += 8;

        let quote_lot_size = u64::from_le_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| ParseError::InvalidData("quote_lot_size".to_string()))?,
        );
        offset += 8;

        let tick_size = u64::from_le_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| ParseError::InvalidData("tick_size".to_string()))?,
        );
        offset += 8;

        // Skip min_order_size
        offset += 8;

        let taker_fee_bps = u16::from_le_bytes(
            data[offset..offset + 2]
                .try_into()
                .map_err(|_| ParseError::InvalidData("taker_fee_bps".to_string()))?,
        );
        offset += 2;

        let maker_fee_bps = i16::from_le_bytes(
            data[offset..offset + 2]
                .try_into()
                .map_err(|_| ParseError::InvalidData("maker_fee_bps".to_string()))?,
        );
        offset += 2;

        let status = data[offset];

        Ok(ParsedMarket {
            base_mint,
            quote_mint,
            base_lot_size,
            quote_lot_size,
            tick_size,
            taker_fee_bps,
            maker_fee_bps,
            status,
        })
    }

    /// Parses an OrderBook account.
    ///
    /// Traverses the B+ tree structure to extract all orders.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed.
    pub fn parse_orderbook(&self, data: &[u8]) -> Result<Vec<ParsedOrder>, ParseError> {
        // OrderBook header layout (after 8-byte discriminator):
        // - bump: u8
        // - padding: [u8; 7]
        // - leaf_count: u32
        // - free_list_head: u32
        // - root: u32
        // - padding2: [u8; 4]
        // - nodes: [Node; ...]

        const HEADER_SIZE: usize = 8 + 1 + 7 + 4 + 4 + 4 + 4; // 32 bytes
        const NODE_SIZE: usize = 88;

        if data.len() < HEADER_SIZE {
            return Err(ParseError::DataTooShort {
                expected: HEADER_SIZE,
                actual: data.len(),
            });
        }

        let leaf_count = u32::from_le_bytes(
            data[16..20]
                .try_into()
                .map_err(|_| ParseError::InvalidData("leaf_count".to_string()))?,
        );

        let mut orders = Vec::with_capacity(leaf_count as usize);

        // Parse nodes starting after header
        let nodes_data = &data[HEADER_SIZE..];
        let node_count = nodes_data.len() / NODE_SIZE;

        for i in 0..node_count {
            let node_start = i * NODE_SIZE;
            let node_end = node_start + NODE_SIZE;

            if node_end > nodes_data.len() {
                break;
            }

            let node_data = &nodes_data[node_start..node_end];

            // Check node tag (first byte): 2 = Leaf
            if node_data[0] == 2 {
                if let Some(order) = self.parse_leaf_node(node_data) {
                    orders.push(order);
                }
            }
        }

        Ok(orders)
    }

    /// Parses a leaf node into an order.
    fn parse_leaf_node(&self, data: &[u8]) -> Option<ParsedOrder> {
        // Leaf node layout:
        // - tag: u8 (= 2)
        // - padding: [u8; 3]
        // - slot: u32
        // - order_id: u128
        // - owner: [u8; 32]
        // - quantity: u64
        // - client_order_id: u64
        // - timestamp: u64
        // - padding2: [u8; 8]

        if data.len() < 88 || data[0] != 2 {
            return None;
        }

        let order_id = u128::from_le_bytes(data[8..24].try_into().ok()?);
        let owner: [u8; 32] = data[24..56].try_into().ok()?;
        let quantity = u64::from_le_bytes(data[56..64].try_into().ok()?);
        let client_order_id = u64::from_le_bytes(data[64..72].try_into().ok()?);

        // Extract price from order_id (upper 64 bits for asks, inverted for bids)
        let price = (order_id >> 64) as u64;

        Some(ParsedOrder {
            order_id,
            owner,
            price,
            quantity,
            client_order_id,
        })
    }

    /// Parses an EventQueue account.
    ///
    /// Iterates through the ring buffer to extract all events.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed.
    pub fn parse_event_queue(&self, data: &[u8]) -> Result<Vec<ParsedEvent>, ParseError> {
        // EventQueue header layout (after 8-byte discriminator):
        // - bump: u8
        // - padding: [u8; 7]
        // - head: u32
        // - count: u32
        // - seq_num: u64
        // - events: [Event; ...]

        const HEADER_SIZE: usize = 8 + 1 + 7 + 4 + 4 + 8; // 32 bytes
        const EVENT_SIZE: usize = 144;

        if data.len() < HEADER_SIZE {
            return Err(ParseError::DataTooShort {
                expected: HEADER_SIZE,
                actual: data.len(),
            });
        }

        let head = u32::from_le_bytes(
            data[16..20]
                .try_into()
                .map_err(|_| ParseError::InvalidData("head".to_string()))?,
        );

        let count = u32::from_le_bytes(
            data[20..24]
                .try_into()
                .map_err(|_| ParseError::InvalidData("count".to_string()))?,
        );

        let events_data = &data[HEADER_SIZE..];
        let capacity = events_data.len() / EVENT_SIZE;

        if capacity == 0 {
            return Ok(Vec::new());
        }

        let mut events = Vec::with_capacity(count as usize);

        for i in 0..count {
            let idx = ((head as usize) + (i as usize)) % capacity;
            let event_start = idx * EVENT_SIZE;
            let event_end = event_start + EVENT_SIZE;

            if event_end > events_data.len() {
                break;
            }

            let event_data = &events_data[event_start..event_end];
            if let Some(event) = self.parse_event(event_data) {
                events.push(event);
            }
        }

        Ok(events)
    }

    /// Parses a single event from the queue.
    fn parse_event(&self, data: &[u8]) -> Option<ParsedEvent> {
        // Event layout:
        // - event_type: u8 (0 = Fill, 1 = Out)
        // - padding: [u8; 7]
        // - ... event-specific data

        if data.is_empty() {
            return None;
        }

        let event_type = match data[0] {
            0 => EventType::Fill,
            1 => EventType::Out,
            _ => return None,
        };

        Some(ParsedEvent { event_type })
    }

    /// Parses an OpenOrders account.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed.
    pub fn parse_open_orders(&self, data: &[u8]) -> Result<ParsedOpenOrders, ParseError> {
        // OpenOrders layout (after 8-byte discriminator):
        // - owner: [u8; 32]
        // - market: [u8; 32]
        // - delegate: Option<[u8; 32]> (1 + 32 bytes)
        // - base_free: u64
        // - base_locked: u64
        // - quote_free: u64
        // - quote_locked: u64
        // - referrer_rebates: u64
        // - open_order_count: u8
        // - orders: [OrderSlot; 128]
        // - bump: u8

        const MIN_SIZE: usize = 8 + 32 + 32 + 1 + 32 + 8 * 5 + 1; // 146 bytes minimum

        if data.len() < MIN_SIZE {
            return Err(ParseError::DataTooShort {
                expected: MIN_SIZE,
                actual: data.len(),
            });
        }

        let mut offset = 8; // Skip discriminator

        let owner: [u8; 32] = data[offset..offset + 32]
            .try_into()
            .map_err(|_| ParseError::InvalidData("owner".to_string()))?;
        offset += 32;

        let market: [u8; 32] = data[offset..offset + 32]
            .try_into()
            .map_err(|_| ParseError::InvalidData("market".to_string()))?;
        offset += 32;

        // Skip delegate (Option<Pubkey>)
        let has_delegate = data[offset] == 1;
        offset += 1;
        if has_delegate {
            offset += 32;
        }

        let base_free = u64::from_le_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| ParseError::InvalidData("base_free".to_string()))?,
        );
        offset += 8;

        let base_locked = u64::from_le_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| ParseError::InvalidData("base_locked".to_string()))?,
        );
        offset += 8;

        let quote_free = u64::from_le_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| ParseError::InvalidData("quote_free".to_string()))?,
        );
        offset += 8;

        let quote_locked = u64::from_le_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| ParseError::InvalidData("quote_locked".to_string()))?,
        );

        Ok(ParsedOpenOrders {
            owner,
            market,
            base_free,
            base_locked,
            quote_free,
            quote_locked,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_new() {
        let parser = AccountParser::new();
        assert_eq!(parser.metrics().parse_count(), 0);
    }

    #[test]
    fn test_parser_default() {
        let parser = AccountParser::default();
        assert_eq!(parser.metrics().parse_count(), 0);
    }

    #[test]
    fn test_parse_too_short() {
        let parser = AccountParser::new();
        let data = vec![0u8; 4];
        let result = parser.parse(&data);
        assert!(matches!(result, Err(ParseError::DataTooShort { .. })));
    }

    #[test]
    fn test_parse_unknown_discriminator() {
        let parser = AccountParser::new();
        let data = vec![0xFFu8; 100];
        let result = parser.parse(&data);
        assert!(matches!(result, Ok(ParsedAccount::Unknown { .. })));
    }

    #[test]
    fn test_parse_metrics_recorded() {
        let parser = AccountParser::new();
        let data = vec![0u8; 100];

        let _ = parser.parse(&data);

        assert_eq!(parser.metrics().parse_count(), 1);
    }
}
