//! Event processor implementation.
//!
//! Processes on-chain events (fills and cancellations) and produces
//! structured data for persistence and publishing.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;

use super::cursor::EventCursor;
use super::metrics::EventMetrics;
use super::types::{OrderUpdate, OutReason, ProcessedFill, ProcessedOut, ProcessingResult};
use crate::book::Side;
use crate::parser::{EventType, ParsedEvent};

/// Event processor for handling fills and cancellations.
///
/// Processes events from the on-chain event queue, tracks progress
/// via cursors, and produces structured data for persistence.
///
/// # Example
///
/// ```rust,ignore
/// use matchbook_indexer::events::EventProcessor;
/// use matchbook_indexer::parser::ParsedEvent;
///
/// let mut processor = EventProcessor::new();
/// let market = [1u8; 32];
/// let events = vec![/* parsed events */];
///
/// let result = processor.process_events(market, &events, 100, 0);
/// println!("Processed {} fills, {} outs", result.fills, result.outs);
/// ```
pub struct EventProcessor {
    /// Cursors per market for tracking progress.
    cursors: HashMap<[u8; 32], EventCursor>,

    /// Metrics for monitoring.
    metrics: Arc<EventMetrics>,
}

impl Default for EventProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl EventProcessor {
    /// Creates a new event processor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cursors: HashMap::new(),
            metrics: Arc::new(EventMetrics::new()),
        }
    }

    /// Returns a reference to the metrics.
    #[must_use]
    pub fn metrics(&self) -> Arc<EventMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Returns the number of markets being tracked.
    #[must_use]
    pub fn market_count(&self) -> usize {
        self.cursors.len()
    }

    /// Returns the cursor for a market.
    #[must_use]
    pub fn get_cursor(&self, market: &[u8; 32]) -> Option<&EventCursor> {
        self.cursors.get(market)
    }

    /// Returns true if a sequence number has been processed for a market.
    #[must_use]
    pub fn is_processed(&self, market: &[u8; 32], seq_num: u64) -> bool {
        self.cursors
            .get(market)
            .map(|c| c.is_processed(seq_num))
            .unwrap_or(false)
    }

    /// Processes a batch of events for a market.
    ///
    /// # Arguments
    ///
    /// * `market` - Market address
    /// * `events` - Parsed events from the event queue
    /// * `head_seq_num` - Current head sequence number of the queue
    /// * `slot` - Solana slot of the update
    ///
    /// # Returns
    ///
    /// Processing result with fills, outs, and order updates.
    pub fn process_events(
        &mut self,
        market: [u8; 32],
        events: &[ParsedEvent],
        head_seq_num: u64,
        slot: u64,
    ) -> ProcessingResult {
        let start = Instant::now();

        // Get or create cursor and get last_processed value
        let cursor = self
            .cursors
            .entry(market)
            .or_insert_with(|| EventCursor::new(head_seq_num));
        cursor.update_head(head_seq_num);
        let mut last_processed = cursor.last_processed;

        let mut result = ProcessingResult::empty();

        for (i, event) in events.iter().enumerate() {
            let seq_num = head_seq_num.saturating_add(i as u64);

            // Skip already processed events (idempotency)
            // Note: last_processed == 0 means nothing processed yet, so seq_num 0 should be processed
            if last_processed > 0 && seq_num <= last_processed {
                result.skipped += 1;
                self.metrics.record_skipped();
                continue;
            }

            match event.event_type {
                EventType::Fill => {
                    if let Some((fill, update)) =
                        Self::process_fill_event(market, event, seq_num, slot)
                    {
                        result.processed_fills.push(fill);
                        result.order_updates.push(update);
                        result.fills += 1;
                    }
                }
                EventType::Out => {
                    if let Some((out, update)) =
                        Self::process_out_event(market, event, seq_num, slot)
                    {
                        result.processed_outs.push(out);
                        result.order_updates.push(update);
                        result.outs += 1;
                    }
                }
            }

            last_processed = seq_num;
            result.events_processed += 1;
        }

        // Update cursor with final processed value
        if let Some(cursor) = self.cursors.get_mut(&market) {
            cursor.last_processed = last_processed;
            cursor.events_processed = cursor
                .events_processed
                .saturating_add(result.events_processed as u64);
        }

        // Record batch metrics
        let elapsed = start.elapsed();
        self.metrics.record_batch(
            result.fills as u64,
            result.outs as u64,
            result.skipped as u64,
            elapsed,
        );

        result
    }

    /// Processes a fill event.
    fn process_fill_event(
        market: [u8; 32],
        _event: &ParsedEvent,
        seq_num: u64,
        slot: u64,
    ) -> Option<(ProcessedFill, OrderUpdate)> {
        // In a real implementation, we would extract data from the event
        // For now, create placeholder data
        let fill = ProcessedFill {
            market,
            maker_order_id: 0,
            taker_order_id: 0,
            maker_address: [0u8; 32],
            taker_address: [0u8; 32],
            price: 0,
            quantity: 0,
            taker_side: Side::Bid,
            taker_fee: 0,
            maker_rebate: 0,
            slot,
            seq_num,
            timestamp: Utc::now(),
        };

        let update = OrderUpdate::Filled {
            market,
            order_id: fill.maker_order_id,
            filled_quantity: fill.quantity,
            remaining_quantity: 0,
            fill_price: fill.price,
            seq_num,
        };

        Some((fill, update))
    }

    /// Processes an out event.
    fn process_out_event(
        market: [u8; 32],
        _event: &ParsedEvent,
        seq_num: u64,
        slot: u64,
    ) -> Option<(ProcessedOut, OrderUpdate)> {
        // In a real implementation, we would extract data from the event
        // For now, create placeholder data
        let out = ProcessedOut {
            market,
            order_id: 0,
            owner: [0u8; 32],
            reason: OutReason::Cancelled,
            remaining_quantity: 0,
            slot,
            seq_num,
            timestamp: Utc::now(),
        };

        let update = OrderUpdate::Cancelled {
            market,
            order_id: out.order_id,
            reason: out.reason,
            seq_num,
        };

        Some((out, update))
    }

    /// Resets the cursor for a market.
    pub fn reset_cursor(&mut self, market: &[u8; 32]) {
        if let Some(cursor) = self.cursors.get_mut(market) {
            cursor.reset();
        }
    }

    /// Removes a market from tracking.
    pub fn remove_market(&mut self, market: &[u8; 32]) -> Option<EventCursor> {
        self.cursors.remove(market)
    }

    /// Clears all markets.
    pub fn clear(&mut self) {
        self.cursors.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_fill_event() -> ParsedEvent {
        ParsedEvent {
            event_type: EventType::Fill,
        }
    }

    fn create_out_event() -> ParsedEvent {
        ParsedEvent {
            event_type: EventType::Out,
        }
    }

    #[test]
    fn test_processor_new() {
        let processor = EventProcessor::new();
        assert_eq!(processor.market_count(), 0);
    }

    #[test]
    fn test_processor_default() {
        let processor = EventProcessor::default();
        assert_eq!(processor.market_count(), 0);
    }

    #[test]
    fn test_processor_process_fills() {
        let mut processor = EventProcessor::new();
        let market = [1u8; 32];

        let events = vec![create_fill_event(), create_fill_event()];

        let result = processor.process_events(market, &events, 0, 100);

        assert_eq!(result.events_processed, 2);
        assert_eq!(result.fills, 2);
        assert_eq!(result.outs, 0);
        assert_eq!(result.processed_fills.len(), 2);
    }

    #[test]
    fn test_processor_process_outs() {
        let mut processor = EventProcessor::new();
        let market = [1u8; 32];

        let events = vec![create_out_event(), create_out_event()];

        let result = processor.process_events(market, &events, 0, 100);

        assert_eq!(result.events_processed, 2);
        assert_eq!(result.fills, 0);
        assert_eq!(result.outs, 2);
        assert_eq!(result.processed_outs.len(), 2);
    }

    #[test]
    fn test_processor_mixed_events() {
        let mut processor = EventProcessor::new();
        let market = [1u8; 32];

        let events = vec![create_fill_event(), create_out_event(), create_fill_event()];

        let result = processor.process_events(market, &events, 0, 100);

        assert_eq!(result.events_processed, 3);
        assert_eq!(result.fills, 2);
        assert_eq!(result.outs, 1);
    }

    #[test]
    fn test_processor_idempotency() {
        let mut processor = EventProcessor::new();
        let market = [1u8; 32];

        let events = vec![create_fill_event(), create_fill_event()];

        // First processing
        let result1 = processor.process_events(market, &events, 0, 100);
        assert_eq!(result1.events_processed, 2);
        assert_eq!(result1.skipped, 0);

        // Second processing (same events)
        let result2 = processor.process_events(market, &events, 0, 101);
        assert_eq!(result2.events_processed, 0);
        assert_eq!(result2.skipped, 2);
    }

    #[test]
    fn test_processor_cursor_tracking() {
        let mut processor = EventProcessor::new();
        let market = [1u8; 32];

        let events = vec![create_fill_event()];
        processor.process_events(market, &events, 10, 100);

        let cursor = processor.get_cursor(&market);
        assert!(cursor.is_some());

        let cursor = cursor.expect("cursor");
        assert_eq!(cursor.head_seq_num, 10);
        assert_eq!(cursor.last_processed, 10);
    }

    #[test]
    fn test_processor_is_processed() {
        let mut processor = EventProcessor::new();
        let market = [1u8; 32];

        // Not tracked yet
        assert!(!processor.is_processed(&market, 0));

        let events = vec![create_fill_event()];
        processor.process_events(market, &events, 5, 100);

        assert!(processor.is_processed(&market, 5));
        assert!(!processor.is_processed(&market, 6));
    }

    #[test]
    fn test_processor_order_updates() {
        let mut processor = EventProcessor::new();
        let market = [1u8; 32];

        let events = vec![create_fill_event(), create_out_event()];
        let result = processor.process_events(market, &events, 0, 100);

        assert_eq!(result.order_updates.len(), 2);
        assert!(result.order_updates[0].is_fill());
        assert!(result.order_updates[1].is_cancellation());
    }

    #[test]
    fn test_processor_reset_cursor() {
        let mut processor = EventProcessor::new();
        let market = [1u8; 32];

        let events = vec![create_fill_event()];
        processor.process_events(market, &events, 10, 100);

        processor.reset_cursor(&market);

        let cursor = processor.get_cursor(&market).expect("cursor");
        assert_eq!(cursor.last_processed, 0);
    }

    #[test]
    fn test_processor_remove_market() {
        let mut processor = EventProcessor::new();
        let market = [1u8; 32];

        let events = vec![create_fill_event()];
        processor.process_events(market, &events, 0, 100);

        assert_eq!(processor.market_count(), 1);

        let removed = processor.remove_market(&market);
        assert!(removed.is_some());
        assert_eq!(processor.market_count(), 0);
    }

    #[test]
    fn test_processor_clear() {
        let mut processor = EventProcessor::new();

        processor.process_events([1u8; 32], &[create_fill_event()], 0, 100);
        processor.process_events([2u8; 32], &[create_fill_event()], 0, 100);

        assert_eq!(processor.market_count(), 2);

        processor.clear();

        assert_eq!(processor.market_count(), 0);
    }

    #[test]
    fn test_processor_metrics() {
        let mut processor = EventProcessor::new();
        let market = [1u8; 32];

        let events = vec![create_fill_event(), create_out_event()];
        processor.process_events(market, &events, 0, 100);

        let metrics = processor.metrics();
        assert_eq!(metrics.fills_processed(), 1);
        assert_eq!(metrics.outs_processed(), 1);
    }

    #[test]
    fn test_processor_multiple_markets() {
        let mut processor = EventProcessor::new();
        let market1 = [1u8; 32];
        let market2 = [2u8; 32];

        processor.process_events(market1, &[create_fill_event()], 0, 100);
        processor.process_events(market2, &[create_out_event()], 0, 100);

        assert_eq!(processor.market_count(), 2);
        assert!(processor.get_cursor(&market1).is_some());
        assert!(processor.get_cursor(&market2).is_some());
    }
}
