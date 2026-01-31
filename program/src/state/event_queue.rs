//! Event queue account structures using a ring buffer.
//!
//! The event queue is implemented as a ring buffer for storing fill and cancel
//! events. This enables asynchronous event processing and decouples matching
//! from settlement.
//!
//! # Structure
//!
//! - [`EventQueueHeader`] - Header for the event queue
//! - [`Event`] - Tagged enum for event types
//! - [`FillEvent`] - Fill event data
//! - [`OutEvent`] - Order removal event data
//!
//! # PDA Derivation
//!
//! Event queue: `["event_queue", market]`
//!
//! # Ring Buffer
//!
//! The queue uses a ring buffer with head pointer and count for O(1) push/pop.
//! Events are consumed by the ConsumeEvents instruction.

use anchor_lang::prelude::*;

/// Seed prefix for EventQueue PDA derivation.
pub const EVENT_QUEUE_SEED: &[u8] = b"event_queue";

/// Event queue header size in bytes.
pub const EVENT_QUEUE_HEADER_SIZE: usize = 120;

// ============================================================================
// Side (for events)
// ============================================================================

/// Side of an order (bid or ask).
#[derive(
    AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy, PartialEq, Eq, Default, InitSpace,
)]
#[repr(u8)]
pub enum Side {
    /// Buy order.
    #[default]
    Bid = 0,
    /// Sell order.
    Ask = 1,
}

impl Side {
    /// Returns the opposite side.
    #[must_use]
    #[inline]
    pub const fn opposite(&self) -> Self {
        match self {
            Self::Bid => Self::Ask,
            Self::Ask => Self::Bid,
        }
    }

    /// Returns true if this is a bid.
    #[must_use]
    #[inline]
    pub const fn is_bid(&self) -> bool {
        matches!(self, Self::Bid)
    }

    /// Returns true if this is an ask.
    #[must_use]
    #[inline]
    pub const fn is_ask(&self) -> bool {
        matches!(self, Self::Ask)
    }
}

impl From<u8> for Side {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Ask,
            _ => Self::Bid,
        }
    }
}

// ============================================================================
// Out Reason
// ============================================================================

/// Reason for order removal from the book.
#[derive(
    AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy, PartialEq, Eq, Default, InitSpace,
)]
#[repr(u8)]
pub enum OutReason {
    /// Order was cancelled by user.
    #[default]
    Cancelled = 0,
    /// Order was fully filled.
    Filled = 1,
    /// Order expired (time-in-force).
    Expired = 2,
}

impl From<u8> for OutReason {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Filled,
            2 => Self::Expired,
            _ => Self::Cancelled,
        }
    }
}

// ============================================================================
// Fill Event
// ============================================================================

/// Fill event emitted when orders match.
///
/// Contains all information needed to settle the trade off-chain.
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy, PartialEq, Eq, InitSpace)]
pub struct FillEvent {
    /// Side of the taker order.
    pub taker_side: Side,
    /// Maker's OpenOrders account.
    pub maker: Pubkey,
    /// Maker's order ID.
    pub maker_order_id: u128,
    /// Maker's client order ID.
    pub maker_client_order_id: u64,
    /// Taker's OpenOrders account.
    pub taker: Pubkey,
    /// Taker's order ID.
    pub taker_order_id: u128,
    /// Taker's client order ID.
    pub taker_client_order_id: u64,
    /// Execution price in ticks.
    pub price: u64,
    /// Filled quantity in lots.
    pub quantity: u64,
    /// Fee paid by taker (in quote tokens).
    pub taker_fee: u64,
    /// Rebate received by maker (in quote tokens).
    pub maker_rebate: u64,
}

impl FillEvent {
    /// Creates a new fill event.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        taker_side: Side,
        maker: Pubkey,
        maker_order_id: u128,
        maker_client_order_id: u64,
        taker: Pubkey,
        taker_order_id: u128,
        taker_client_order_id: u64,
        price: u64,
        quantity: u64,
        taker_fee: u64,
        maker_rebate: u64,
    ) -> Self {
        Self {
            taker_side,
            maker,
            maker_order_id,
            maker_client_order_id,
            taker,
            taker_order_id,
            taker_client_order_id,
            price,
            quantity,
            taker_fee,
            maker_rebate,
        }
    }
}

// ============================================================================
// Out Event
// ============================================================================

/// Out event emitted when an order is removed from the book.
///
/// This can happen due to cancellation, full fill, or expiration.
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy, PartialEq, Eq, InitSpace)]
pub struct OutEvent {
    /// Side of the order.
    pub side: Side,
    /// Owner's OpenOrders account.
    pub owner: Pubkey,
    /// Order ID.
    pub order_id: u128,
    /// Client order ID.
    pub client_order_id: u64,
    /// Base tokens released back to owner.
    pub base_released: u64,
    /// Quote tokens released back to owner.
    pub quote_released: u64,
    /// Reason for removal.
    pub reason: OutReason,
}

impl OutEvent {
    /// Creates a new out event.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        side: Side,
        owner: Pubkey,
        order_id: u128,
        client_order_id: u64,
        base_released: u64,
        quote_released: u64,
        reason: OutReason,
    ) -> Self {
        Self {
            side,
            owner,
            order_id,
            client_order_id,
            base_released,
            quote_released,
            reason,
        }
    }
}

// ============================================================================
// Event (Tagged Enum)
// ============================================================================

/// Event stored in the event queue.
///
/// Events are emitted during order matching and consumed by the
/// ConsumeEvents instruction for off-chain processing.
#[derive(
    AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy, PartialEq, Eq, InitSpace, Default,
)]
pub enum Event {
    /// Empty slot (no event).
    #[default]
    Empty,
    /// Fill event - orders matched.
    Fill(FillEvent),
    /// Out event - order removed from book.
    Out(OutEvent),
}

impl Event {
    /// Returns true if this is an empty slot.
    #[must_use]
    #[inline]
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Returns true if this is a fill event.
    #[must_use]
    #[inline]
    pub const fn is_fill(&self) -> bool {
        matches!(self, Self::Fill(_))
    }

    /// Returns true if this is an out event.
    #[must_use]
    #[inline]
    pub const fn is_out(&self) -> bool {
        matches!(self, Self::Out(_))
    }

    /// Returns the fill event if this is one.
    #[must_use]
    #[inline]
    pub const fn as_fill(&self) -> Option<&FillEvent> {
        match self {
            Self::Fill(fill) => Some(fill),
            _ => None,
        }
    }

    /// Returns the out event if this is one.
    #[must_use]
    #[inline]
    pub const fn as_out(&self) -> Option<&OutEvent> {
        match self {
            Self::Out(out) => Some(out),
            _ => None,
        }
    }

    /// Creates a fill event.
    #[must_use]
    #[inline]
    pub const fn new_fill(fill: FillEvent) -> Self {
        Self::Fill(fill)
    }

    /// Creates an out event.
    #[must_use]
    #[inline]
    pub const fn new_out(out: OutEvent) -> Self {
        Self::Out(out)
    }
}

// ============================================================================
// Event Queue Header
// ============================================================================

/// Event queue header (ring buffer metadata).
///
/// This structure is stored at the beginning of the event queue account
/// and contains metadata about the ring buffer. The actual events follow
/// immediately after in the account data.
///
/// # PDA Seeds
///
/// Event queue: `["event_queue", market.key()]`
#[account(zero_copy(unsafe))]
#[derive(Debug)]
#[repr(C)]
pub struct EventQueueHeader {
    /// Bump seed for PDA derivation.
    pub bump: u8,
    /// Padding for alignment.
    pub padding: [u8; 7],
    /// Market this event queue belongs to.
    pub market: Pubkey,
    /// Head pointer (index of next event to consume).
    pub head: u32,
    /// Number of pending events in the queue.
    pub count: u32,
    /// Monotonically increasing sequence number.
    pub seq_num: u64,
    /// Reserved for future use.
    pub reserved: [u8; 64],
}

impl EventQueueHeader {
    /// Header size in bytes.
    pub const SIZE: usize = EVENT_QUEUE_HEADER_SIZE;

    /// Derives the PDA for an event queue.
    ///
    /// # Arguments
    ///
    /// * `market` - The market pubkey
    /// * `program_id` - The program ID
    #[must_use]
    pub fn derive_pda(market: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[EVENT_QUEUE_SEED, market.as_ref()], program_id)
    }

    /// Returns true if the queue is empty.
    #[must_use]
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Returns the number of events in the queue.
    #[must_use]
    #[inline]
    pub const fn len(&self) -> u32 {
        self.count
    }

    /// Returns true if the queue is full.
    ///
    /// # Arguments
    ///
    /// * `capacity` - The total capacity of the event array
    #[must_use]
    #[inline]
    pub const fn is_full(&self, capacity: usize) -> bool {
        self.count as usize >= capacity
    }

    /// Returns the remaining capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity` - The total capacity of the event array
    #[must_use]
    #[inline]
    pub const fn remaining_capacity(&self, capacity: usize) -> usize {
        capacity.saturating_sub(self.count as usize)
    }

    /// Calculates the tail index (next write position).
    ///
    /// # Arguments
    ///
    /// * `capacity` - The total capacity of the event array
    #[must_use]
    #[inline]
    pub const fn tail(&self, capacity: usize) -> usize {
        ((self.head as usize) + (self.count as usize)) % capacity
    }

    /// Pushes an event to the queue.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to push
    /// * `events` - The event array (mutable)
    ///
    /// # Returns
    ///
    /// The sequence number assigned to the event, or None if queue is full.
    #[allow(clippy::indexing_slicing)]
    pub fn push(&mut self, event: Event, events: &mut [Event]) -> Option<u64> {
        let capacity = events.len();
        if self.is_full(capacity) {
            return None;
        }

        let tail = self.tail(capacity);
        // SAFETY: tail is always < capacity due to modulo operation
        events[tail] = event;

        self.count = self.count.saturating_add(1);
        self.seq_num = self.seq_num.saturating_add(1);

        Some(self.seq_num)
    }

    /// Pops an event from the queue.
    ///
    /// # Arguments
    ///
    /// * `events` - The event array
    ///
    /// # Returns
    ///
    /// The event at the head, or None if queue is empty.
    #[allow(clippy::indexing_slicing)]
    pub fn pop(&mut self, events: &mut [Event]) -> Option<Event> {
        if self.is_empty() {
            return None;
        }

        let capacity = events.len();
        let head_idx = self.head as usize;

        // SAFETY: head is always < capacity due to modulo in advance_head
        let event = events[head_idx];
        events[head_idx] = Event::Empty;

        self.head = ((self.head as usize + 1) % capacity) as u32;
        self.count = self.count.saturating_sub(1);

        Some(event)
    }

    /// Peeks at the event at the head without removing it.
    ///
    /// # Arguments
    ///
    /// * `events` - The event array
    ///
    /// # Returns
    ///
    /// A reference to the event at the head, or None if queue is empty.
    #[must_use]
    #[allow(clippy::indexing_slicing)]
    pub fn peek<'a>(&self, events: &'a [Event]) -> Option<&'a Event> {
        if self.is_empty() {
            return None;
        }

        // SAFETY: head is always < capacity
        Some(&events[self.head as usize])
    }

    /// Peeks at an event at a specific offset from head.
    ///
    /// # Arguments
    ///
    /// * `events` - The event array
    /// * `offset` - Offset from head (0 = head)
    ///
    /// # Returns
    ///
    /// A reference to the event, or None if offset is out of bounds.
    #[must_use]
    #[allow(clippy::indexing_slicing)]
    pub fn peek_at<'a>(&self, events: &'a [Event], offset: u32) -> Option<&'a Event> {
        if offset >= self.count {
            return None;
        }

        let capacity = events.len();
        let idx = ((self.head as usize) + (offset as usize)) % capacity;

        // SAFETY: idx is always < capacity due to modulo
        Some(&events[idx])
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_header() -> EventQueueHeader {
        EventQueueHeader {
            bump: 255,
            padding: [0; 7],
            market: Pubkey::default(),
            head: 0,
            count: 0,
            seq_num: 0,
            reserved: [0; 64],
        }
    }

    fn create_test_fill_event() -> FillEvent {
        FillEvent::new(
            Side::Bid,
            Pubkey::new_unique(),
            1000,
            100,
            Pubkey::new_unique(),
            2000,
            200,
            50,
            10,
            5,
            2,
        )
    }

    fn create_test_out_event() -> OutEvent {
        OutEvent::new(
            Side::Ask,
            Pubkey::new_unique(),
            3000,
            300,
            100,
            50,
            OutReason::Cancelled,
        )
    }

    #[test]
    fn test_event_queue_pda_derivation() {
        let market = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();

        let (pda1, bump1) = EventQueueHeader::derive_pda(&market, &program_id);
        let (pda2, bump2) = EventQueueHeader::derive_pda(&market, &program_id);

        // Same inputs produce same outputs
        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);

        // Different market produces different PDA
        let other_market = Pubkey::new_unique();
        let (pda3, _) = EventQueueHeader::derive_pda(&other_market, &program_id);
        assert_ne!(pda1, pda3);
    }

    #[test]
    fn test_push_single_event() {
        let mut header = create_test_header();
        let mut events = vec![Event::Empty; 10];

        let fill = create_test_fill_event();
        let seq = header.push(Event::new_fill(fill), &mut events);

        assert!(seq.is_some());
        assert_eq!(seq, Some(1));
        assert_eq!(header.count, 1);
        assert_eq!(header.seq_num, 1);
        assert!(!header.is_empty());
    }

    #[test]
    fn test_push_pop_fifo() {
        let mut header = create_test_header();
        let mut events = vec![Event::Empty; 10];

        // Push three events
        let fill1 = create_test_fill_event();
        let fill2 = create_test_fill_event();
        let out1 = create_test_out_event();

        header.push(Event::new_fill(fill1), &mut events);
        header.push(Event::new_fill(fill2), &mut events);
        header.push(Event::new_out(out1), &mut events);

        assert_eq!(header.count, 3);

        // Pop should return in FIFO order
        let popped1 = header.pop(&mut events);
        assert!(popped1.is_some());
        assert!(popped1.as_ref().is_some_and(|e| e.is_fill()));

        let popped2 = header.pop(&mut events);
        assert!(popped2.is_some());
        assert!(popped2.as_ref().is_some_and(|e| e.is_fill()));

        let popped3 = header.pop(&mut events);
        assert!(popped3.is_some());
        assert!(popped3.as_ref().is_some_and(|e| e.is_out()));

        assert!(header.is_empty());
    }

    #[test]
    fn test_wrap_around() {
        let mut header = create_test_header();
        let capacity = 4;
        let mut events = vec![Event::Empty; capacity];

        // Fill the queue
        for _ in 0..capacity {
            let fill = create_test_fill_event();
            header.push(Event::new_fill(fill), &mut events);
        }
        assert!(header.is_full(capacity));

        // Pop two events
        header.pop(&mut events);
        header.pop(&mut events);
        assert_eq!(header.count, 2);
        assert_eq!(header.head, 2);

        // Push two more (should wrap around)
        let fill = create_test_fill_event();
        let seq1 = header.push(Event::new_fill(fill), &mut events);
        assert!(seq1.is_some());

        let fill = create_test_fill_event();
        let seq2 = header.push(Event::new_fill(fill), &mut events);
        assert!(seq2.is_some());

        assert!(header.is_full(capacity));

        // Verify tail wrapped around
        assert_eq!(header.tail(capacity), 2); // (2 + 4) % 4 = 2
    }

    #[test]
    fn test_queue_full() {
        let mut header = create_test_header();
        let capacity = 3;
        let mut events = vec![Event::Empty; capacity];

        // Fill the queue
        for _ in 0..capacity {
            let fill = create_test_fill_event();
            let result = header.push(Event::new_fill(fill), &mut events);
            assert!(result.is_some());
        }

        // Try to push when full
        let fill = create_test_fill_event();
        let result = header.push(Event::new_fill(fill), &mut events);
        assert!(result.is_none());
    }

    #[test]
    fn test_queue_empty_pop() {
        let mut header = create_test_header();
        let mut events = vec![Event::Empty; 10];

        // Pop from empty queue
        let result = header.pop(&mut events);
        assert!(result.is_none());
    }

    #[test]
    fn test_peek() {
        let mut header = create_test_header();
        let mut events = vec![Event::Empty; 10];

        // Peek empty queue
        assert!(header.peek(&events).is_none());

        // Push and peek
        let fill = create_test_fill_event();
        header.push(Event::new_fill(fill), &mut events);

        let peeked = header.peek(&events);
        assert!(peeked.is_some());
        assert!(peeked.is_some_and(|e| e.is_fill()));

        // Peek doesn't remove
        assert_eq!(header.count, 1);
    }

    #[test]
    fn test_peek_at() {
        let mut header = create_test_header();
        let mut events = vec![Event::Empty; 10];

        // Push multiple events
        let fill = create_test_fill_event();
        header.push(Event::new_fill(fill), &mut events);

        let out = create_test_out_event();
        header.push(Event::new_out(out), &mut events);

        // Peek at offset 0 (head)
        let peeked0 = header.peek_at(&events, 0);
        assert!(peeked0.is_some_and(|e| e.is_fill()));

        // Peek at offset 1
        let peeked1 = header.peek_at(&events, 1);
        assert!(peeked1.is_some_and(|e| e.is_out()));

        // Peek at invalid offset
        let peeked2 = header.peek_at(&events, 2);
        assert!(peeked2.is_none());
    }

    #[test]
    fn test_fill_event_creation() {
        let maker = Pubkey::new_unique();
        let taker = Pubkey::new_unique();

        let fill = FillEvent::new(Side::Bid, maker, 1000, 100, taker, 2000, 200, 50, 10, 5, 2);

        assert_eq!(fill.taker_side, Side::Bid);
        assert_eq!(fill.maker, maker);
        assert_eq!(fill.maker_order_id, 1000);
        assert_eq!(fill.maker_client_order_id, 100);
        assert_eq!(fill.taker, taker);
        assert_eq!(fill.taker_order_id, 2000);
        assert_eq!(fill.taker_client_order_id, 200);
        assert_eq!(fill.price, 50);
        assert_eq!(fill.quantity, 10);
        assert_eq!(fill.taker_fee, 5);
        assert_eq!(fill.maker_rebate, 2);
    }

    #[test]
    fn test_out_event_creation() {
        let owner = Pubkey::new_unique();

        let out = OutEvent::new(Side::Ask, owner, 3000, 300, 100, 50, OutReason::Filled);

        assert_eq!(out.side, Side::Ask);
        assert_eq!(out.owner, owner);
        assert_eq!(out.order_id, 3000);
        assert_eq!(out.client_order_id, 300);
        assert_eq!(out.base_released, 100);
        assert_eq!(out.quote_released, 50);
        assert_eq!(out.reason, OutReason::Filled);
    }

    #[test]
    fn test_side_opposite() {
        assert_eq!(Side::Bid.opposite(), Side::Ask);
        assert_eq!(Side::Ask.opposite(), Side::Bid);
    }

    #[test]
    fn test_side_is_bid_ask() {
        assert!(Side::Bid.is_bid());
        assert!(!Side::Bid.is_ask());
        assert!(!Side::Ask.is_bid());
        assert!(Side::Ask.is_ask());
    }

    #[test]
    fn test_side_from_u8() {
        assert_eq!(Side::from(0), Side::Bid);
        assert_eq!(Side::from(1), Side::Ask);
        assert_eq!(Side::from(255), Side::Bid); // Default
    }

    #[test]
    fn test_out_reason_from_u8() {
        assert_eq!(OutReason::from(0), OutReason::Cancelled);
        assert_eq!(OutReason::from(1), OutReason::Filled);
        assert_eq!(OutReason::from(2), OutReason::Expired);
        assert_eq!(OutReason::from(255), OutReason::Cancelled); // Default
    }

    #[test]
    fn test_event_type_checks() {
        let empty = Event::Empty;
        assert!(empty.is_empty());
        assert!(!empty.is_fill());
        assert!(!empty.is_out());

        let fill = Event::new_fill(create_test_fill_event());
        assert!(!fill.is_empty());
        assert!(fill.is_fill());
        assert!(!fill.is_out());

        let out = Event::new_out(create_test_out_event());
        assert!(!out.is_empty());
        assert!(!out.is_fill());
        assert!(out.is_out());
    }

    #[test]
    fn test_event_as_accessors() {
        let fill_event = create_test_fill_event();
        let fill = Event::new_fill(fill_event);
        assert!(fill.as_fill().is_some());
        assert!(fill.as_out().is_none());

        let out_event = create_test_out_event();
        let out = Event::new_out(out_event);
        assert!(out.as_fill().is_none());
        assert!(out.as_out().is_some());
    }

    #[test]
    fn test_sequence_number_increments() {
        let mut header = create_test_header();
        let mut events = vec![Event::Empty; 10];

        let fill = create_test_fill_event();
        let seq1 = header.push(Event::new_fill(fill), &mut events);
        assert_eq!(seq1, Some(1));

        let fill = create_test_fill_event();
        let seq2 = header.push(Event::new_fill(fill), &mut events);
        assert_eq!(seq2, Some(2));

        let fill = create_test_fill_event();
        let seq3 = header.push(Event::new_fill(fill), &mut events);
        assert_eq!(seq3, Some(3));

        // Pop doesn't affect seq_num
        header.pop(&mut events);
        assert_eq!(header.seq_num, 3);
    }

    #[test]
    fn test_remaining_capacity() {
        let mut header = create_test_header();
        let capacity = 5;
        let mut events = vec![Event::Empty; capacity];

        assert_eq!(header.remaining_capacity(capacity), 5);

        let fill = create_test_fill_event();
        header.push(Event::new_fill(fill), &mut events);
        assert_eq!(header.remaining_capacity(capacity), 4);

        let fill = create_test_fill_event();
        header.push(Event::new_fill(fill), &mut events);
        assert_eq!(header.remaining_capacity(capacity), 3);

        header.pop(&mut events);
        assert_eq!(header.remaining_capacity(capacity), 4);
    }
}
