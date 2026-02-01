//! Event cursor for tracking processing progress.
//!
//! Provides sequence number tracking for idempotent event processing.

use serde::{Deserialize, Serialize};

/// Cursor for tracking event processing progress.
///
/// Maintains the last processed sequence number to enable
/// idempotent processing and replay handling.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventCursor {
    /// Head sequence number from the event queue.
    pub head_seq_num: u64,

    /// Last successfully processed sequence number.
    pub last_processed: u64,

    /// Number of events processed.
    pub events_processed: u64,
}

impl EventCursor {
    /// Creates a new cursor starting at the given head sequence.
    #[must_use]
    pub const fn new(head_seq_num: u64) -> Self {
        Self {
            head_seq_num,
            last_processed: 0,
            events_processed: 0,
        }
    }

    /// Creates a cursor with a specific last processed value.
    #[must_use]
    pub const fn with_last_processed(head_seq_num: u64, last_processed: u64) -> Self {
        Self {
            head_seq_num,
            last_processed,
            events_processed: 0,
        }
    }

    /// Returns true if the given sequence number has been processed.
    #[must_use]
    pub const fn is_processed(&self, seq_num: u64) -> bool {
        seq_num <= self.last_processed
    }

    /// Returns true if the given sequence number should be processed.
    #[must_use]
    pub const fn should_process(&self, seq_num: u64) -> bool {
        seq_num > self.last_processed
    }

    /// Marks a sequence number as processed.
    ///
    /// Only updates if the sequence number is greater than the current.
    pub fn mark_processed(&mut self, seq_num: u64) {
        if seq_num > self.last_processed {
            self.last_processed = seq_num;
            self.events_processed = self.events_processed.saturating_add(1);
        }
    }

    /// Updates the head sequence number.
    pub fn update_head(&mut self, head_seq_num: u64) {
        self.head_seq_num = head_seq_num;
    }

    /// Returns the number of pending events.
    #[must_use]
    pub fn pending_count(&self) -> u64 {
        self.head_seq_num.saturating_sub(self.last_processed)
    }

    /// Returns true if there are pending events.
    #[must_use]
    pub const fn has_pending(&self) -> bool {
        self.head_seq_num > self.last_processed
    }

    /// Resets the cursor to initial state.
    pub fn reset(&mut self) {
        self.head_seq_num = 0;
        self.last_processed = 0;
        self.events_processed = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_new() {
        let cursor = EventCursor::new(100);
        assert_eq!(cursor.head_seq_num, 100);
        assert_eq!(cursor.last_processed, 0);
        assert_eq!(cursor.events_processed, 0);
    }

    #[test]
    fn test_cursor_default() {
        let cursor = EventCursor::default();
        assert_eq!(cursor.head_seq_num, 0);
        assert_eq!(cursor.last_processed, 0);
    }

    #[test]
    fn test_cursor_with_last_processed() {
        let cursor = EventCursor::with_last_processed(100, 50);
        assert_eq!(cursor.head_seq_num, 100);
        assert_eq!(cursor.last_processed, 50);
    }

    #[test]
    fn test_cursor_is_processed() {
        let cursor = EventCursor::with_last_processed(100, 50);

        assert!(cursor.is_processed(50));
        assert!(cursor.is_processed(25));
        assert!(!cursor.is_processed(51));
        assert!(!cursor.is_processed(100));
    }

    #[test]
    fn test_cursor_should_process() {
        let cursor = EventCursor::with_last_processed(100, 50);

        assert!(!cursor.should_process(50));
        assert!(!cursor.should_process(25));
        assert!(cursor.should_process(51));
        assert!(cursor.should_process(100));
    }

    #[test]
    fn test_cursor_mark_processed() {
        let mut cursor = EventCursor::new(100);

        cursor.mark_processed(10);
        assert_eq!(cursor.last_processed, 10);
        assert_eq!(cursor.events_processed, 1);

        cursor.mark_processed(20);
        assert_eq!(cursor.last_processed, 20);
        assert_eq!(cursor.events_processed, 2);

        // Should not go backwards
        cursor.mark_processed(15);
        assert_eq!(cursor.last_processed, 20);
        assert_eq!(cursor.events_processed, 2);
    }

    #[test]
    fn test_cursor_pending_count() {
        let cursor = EventCursor::with_last_processed(100, 50);
        assert_eq!(cursor.pending_count(), 50);

        let cursor2 = EventCursor::with_last_processed(100, 100);
        assert_eq!(cursor2.pending_count(), 0);
    }

    #[test]
    fn test_cursor_has_pending() {
        let cursor = EventCursor::with_last_processed(100, 50);
        assert!(cursor.has_pending());

        let cursor2 = EventCursor::with_last_processed(100, 100);
        assert!(!cursor2.has_pending());
    }

    #[test]
    fn test_cursor_reset() {
        let mut cursor = EventCursor::with_last_processed(100, 50);
        cursor.events_processed = 25;

        cursor.reset();

        assert_eq!(cursor.head_seq_num, 0);
        assert_eq!(cursor.last_processed, 0);
        assert_eq!(cursor.events_processed, 0);
    }
}
