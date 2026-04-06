//! Order book account structures using a critbit tree.
//!
//! The order book is implemented as a critbit tree (a form of radix tree) optimized
//! for Solana's compute constraints. This provides O(log n) operations for insert,
//! remove, and lookup.
//!
//! # Structure
//!
//! - [`OrderBookSideHeader`] - Header for the order book (bids or asks)
//! - [`AnyNode`] - Tagged enum for tree nodes
//! - [`InnerNode`] - Internal nodes for tree navigation
//! - [`LeafNode`] - Leaf nodes containing order data
//! - [`FreeNode`] - Nodes in the free list for reuse
//!
//! # PDA Derivation
//!
//! - Bids: `["bids", market]`
//! - Asks: `["asks", market]`
//!
//! # Key Encoding
//!
//! Orders are keyed by a u128 that encodes price-time priority:
//! - **Bids**: `(!price << 64) | seq_num` - Higher prices sort first
//! - **Asks**: `(price << 64) | seq_num` - Lower prices sort first

use anchor_lang::prelude::*;

/// Seed prefix for Bids PDA derivation.
pub const BIDS_SEED: &[u8] = b"bids";

/// Seed prefix for Asks PDA derivation.
pub const ASKS_SEED: &[u8] = b"asks";

/// Sentinel value indicating no node (null pointer).
pub const SENTINEL: u32 = u32::MAX;

/// Size of each node in bytes (for space calculation).
pub const NODE_SIZE: usize = 88;

/// Order book side header size in bytes.
pub const ORDERBOOK_HEADER_SIZE: usize = 128;

// ============================================================================
// Node Tag
// ============================================================================

/// Tag discriminator for node types.
#[derive(
    AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy, PartialEq, Eq, Default, InitSpace,
)]
#[repr(u8)]
pub enum NodeTag {
    /// Uninitialized node slot.
    #[default]
    Uninitialized,
    /// Inner node for tree navigation.
    InnerNode,
    /// Leaf node containing order data.
    LeafNode,
    /// Free node in the free list.
    FreeNode,
    /// Last free node in the free list (next = SENTINEL).
    LastFreeNode,
}

impl From<u8> for NodeTag {
    fn from(value: u8) -> Self {
        match value {
            1 => NodeTag::InnerNode,
            2 => NodeTag::LeafNode,
            3 => NodeTag::FreeNode,
            4 => NodeTag::LastFreeNode,
            _ => NodeTag::Uninitialized,
        }
    }
}

// ============================================================================
// Time In Force
// ============================================================================

/// Time-in-force options for orders.
#[derive(
    AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy, PartialEq, Eq, Default, InitSpace,
)]
#[repr(u8)]
pub enum TimeInForce {
    /// Good-til-cancelled: remains on book until filled or cancelled.
    #[default]
    GoodTilCancelled,
    /// Immediate-or-cancel: fills what it can, cancels the rest.
    ImmediateOrCancel,
    /// Fill-or-kill: must fill entirely or cancel entirely.
    FillOrKill,
    /// Post-only: must not match immediately (maker only).
    PostOnly,
}

impl From<u8> for TimeInForce {
    fn from(value: u8) -> Self {
        match value {
            1 => TimeInForce::ImmediateOrCancel,
            2 => TimeInForce::FillOrKill,
            3 => TimeInForce::PostOnly,
            _ => TimeInForce::GoodTilCancelled,
        }
    }
}

// ============================================================================
// Order ID
// ============================================================================

/// Unique identifier for an order encoding price-time priority.
///
/// The key is a u128 where:
/// - Upper 64 bits: price (inverted for bids so higher prices sort first)
/// - Lower 64 bits: sequence number (for time priority within same price)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct OrderId(pub u128);

impl OrderId {
    /// Creates a new order ID for a bid order.
    ///
    /// Bids use inverted price so higher prices sort first (lower key value).
    ///
    /// # Arguments
    ///
    /// * `price` - The order price in native units
    /// * `seq_num` - The sequence number for time priority
    #[must_use]
    #[inline]
    pub const fn new_bid(price: u64, seq_num: u64) -> Self {
        Self(((!price as u128) << 64) | (seq_num as u128))
    }

    /// Creates a new order ID for an ask order.
    ///
    /// Asks use normal price so lower prices sort first (lower key value).
    ///
    /// # Arguments
    ///
    /// * `price` - The order price in native units
    /// * `seq_num` - The sequence number for time priority
    #[must_use]
    #[inline]
    pub const fn new_ask(price: u64, seq_num: u64) -> Self {
        Self(((price as u128) << 64) | (seq_num as u128))
    }

    /// Extracts the price from the order ID.
    ///
    /// # Arguments
    ///
    /// * `is_bid` - Whether this is a bid order (price is inverted)
    #[must_use]
    #[inline]
    pub const fn price(&self, is_bid: bool) -> u64 {
        let raw = (self.0 >> 64) as u64;
        if is_bid {
            !raw
        } else {
            raw
        }
    }

    /// Extracts the sequence number from the order ID.
    #[must_use]
    #[inline]
    pub const fn seq_num(&self) -> u64 {
        self.0 as u64
    }

    /// Returns the raw u128 value.
    #[must_use]
    #[inline]
    pub const fn get(&self) -> u128 {
        self.0
    }
}

// ============================================================================
// Inner Node
// ============================================================================

/// Inner node for critbit tree navigation.
///
/// Inner nodes store the critical bit position and pointers to children.
/// The critical bit is the first bit position where the keys of the left
/// and right subtrees differ.
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy, PartialEq, Eq, InitSpace)]
pub struct InnerNode {
    /// The critical bit position (0-127).
    pub prefix_len: u32,
    /// The key prefix for this subtree.
    pub key: u128,
    /// Child node indices [left, right].
    pub children: [u32; 2],
}

impl InnerNode {
    /// Creates a new inner node.
    ///
    /// # Arguments
    ///
    /// * `prefix_len` - The critical bit position
    /// * `key` - The key prefix
    /// * `children` - Child node indices [left, right]
    #[must_use]
    pub const fn new(prefix_len: u32, key: u128, children: [u32; 2]) -> Self {
        Self {
            prefix_len,
            key,
            children,
        }
    }

    /// Returns the child index for the given direction.
    ///
    /// # Arguments
    ///
    /// * `side` - 0 for left, 1 for right
    #[must_use]
    #[inline]
    #[allow(clippy::indexing_slicing)]
    pub const fn child(&self, side: usize) -> u32 {
        // SAFETY: side & 1 is always 0 or 1, which is within bounds of [u32; 2]
        self.children[side & 1]
    }

    /// Sets the child index for the given direction.
    ///
    /// # Arguments
    ///
    /// * `side` - 0 for left, 1 for right
    /// * `index` - The child node index
    #[inline]
    #[allow(clippy::indexing_slicing)]
    pub fn set_child(&mut self, side: usize, index: u32) {
        // SAFETY: side & 1 is always 0 or 1, which is within bounds of [u32; 2]
        self.children[side & 1] = index;
    }
}

// ============================================================================
// Leaf Node
// ============================================================================

/// Leaf node containing order data.
///
/// Leaf nodes store the actual order information and are the terminal
/// nodes of the critbit tree.
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy, PartialEq, Eq, InitSpace)]
pub struct LeafNode {
    /// Slot index in the owner's OpenOrders account.
    pub owner_slot: u8,
    /// Time-in-force setting for this order.
    pub time_in_force: TimeInForce,
    /// Order key encoding price-time priority.
    pub key: u128,
    /// Owner's OpenOrders account pubkey.
    pub owner: Pubkey,
    /// Remaining quantity in lots.
    pub quantity: u64,
    /// Client-provided order ID for tracking.
    pub client_order_id: u64,
}

impl LeafNode {
    /// Creates a new leaf node.
    ///
    /// # Arguments
    ///
    /// * `owner_slot` - Slot index in owner's OpenOrders
    /// * `time_in_force` - Order time-in-force setting
    /// * `key` - Order key (price-time priority)
    /// * `owner` - Owner's OpenOrders pubkey
    /// * `quantity` - Order quantity in lots
    /// * `client_order_id` - Client-provided order ID
    #[must_use]
    pub const fn new(
        owner_slot: u8,
        time_in_force: TimeInForce,
        key: u128,
        owner: Pubkey,
        quantity: u64,
        client_order_id: u64,
    ) -> Self {
        Self {
            owner_slot,
            time_in_force,
            key,
            owner,
            quantity,
            client_order_id,
        }
    }

    /// Returns the order ID from the key.
    #[must_use]
    #[inline]
    pub const fn order_id(&self) -> OrderId {
        OrderId(self.key)
    }

    /// Extracts the price from the key.
    ///
    /// # Arguments
    ///
    /// * `is_bid` - Whether this is a bid order
    #[must_use]
    #[inline]
    pub const fn price(&self, is_bid: bool) -> u64 {
        OrderId(self.key).price(is_bid)
    }
}

// ============================================================================
// Free Node
// ============================================================================

/// Free node in the free list for node reuse.
///
/// When nodes are removed from the tree, they are added to a free list
/// for reuse, avoiding the need for reallocation.
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy, PartialEq, Eq, InitSpace)]
pub struct FreeNode {
    /// Index of next free node (SENTINEL if last).
    pub next: u32,
}

impl FreeNode {
    /// Creates a new free node.
    ///
    /// # Arguments
    ///
    /// * `next` - Index of the next free node
    #[must_use]
    pub const fn new(next: u32) -> Self {
        Self { next }
    }

    /// Returns true if this is the last free node.
    #[must_use]
    #[inline]
    pub const fn is_last(&self) -> bool {
        self.next == SENTINEL
    }
}

// ============================================================================
// Any Node (Tagged Enum)
// ============================================================================

/// Tagged enum representing any node in the order book.
///
/// This allows treating all node types uniformly with type safety.
#[derive(
    AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy, PartialEq, Eq, InitSpace, Default,
)]
pub enum AnyNode {
    /// Uninitialized node slot.
    #[default]
    Uninitialized,
    /// Inner node for tree navigation.
    Inner(InnerNode),
    /// Leaf node containing order data.
    Leaf(LeafNode),
    /// Free node in the free list.
    Free(FreeNode),
}

impl AnyNode {
    /// Returns the node tag.
    #[must_use]
    #[inline]
    pub const fn tag(&self) -> NodeTag {
        match self {
            Self::Uninitialized => NodeTag::Uninitialized,
            Self::Inner(_) => NodeTag::InnerNode,
            Self::Leaf(_) => NodeTag::LeafNode,
            Self::Free(_) => NodeTag::FreeNode,
        }
    }

    /// Returns true if this is an inner node.
    #[must_use]
    #[inline]
    pub const fn is_inner(&self) -> bool {
        matches!(self, Self::Inner(_))
    }

    /// Returns true if this is a leaf node.
    #[must_use]
    #[inline]
    pub const fn is_leaf(&self) -> bool {
        matches!(self, Self::Leaf(_))
    }

    /// Returns true if this is a free node.
    #[must_use]
    #[inline]
    pub const fn is_free(&self) -> bool {
        matches!(self, Self::Free(_))
    }

    /// Returns true if this node is uninitialized.
    #[must_use]
    #[inline]
    pub const fn is_uninitialized(&self) -> bool {
        matches!(self, Self::Uninitialized)
    }

    /// Returns the node as an inner node reference, if it is one.
    #[must_use]
    #[inline]
    pub const fn as_inner(&self) -> Option<&InnerNode> {
        match self {
            Self::Inner(inner) => Some(inner),
            _ => None,
        }
    }

    /// Returns the node as a mutable inner node reference, if it is one.
    #[must_use]
    #[inline]
    pub fn as_inner_mut(&mut self) -> Option<&mut InnerNode> {
        match self {
            Self::Inner(inner) => Some(inner),
            _ => None,
        }
    }

    /// Returns the node as a leaf node reference, if it is one.
    #[must_use]
    #[inline]
    pub const fn as_leaf(&self) -> Option<&LeafNode> {
        match self {
            Self::Leaf(leaf) => Some(leaf),
            _ => None,
        }
    }

    /// Returns the node as a mutable leaf node reference, if it is one.
    #[must_use]
    #[inline]
    pub fn as_leaf_mut(&mut self) -> Option<&mut LeafNode> {
        match self {
            Self::Leaf(leaf) => Some(leaf),
            _ => None,
        }
    }

    /// Returns the node as a free node reference, if it is one.
    #[must_use]
    #[inline]
    pub const fn as_free(&self) -> Option<&FreeNode> {
        match self {
            Self::Free(free) => Some(free),
            _ => None,
        }
    }

    /// Returns the node as a mutable free node reference, if it is one.
    #[must_use]
    #[inline]
    pub fn as_free_mut(&mut self) -> Option<&mut FreeNode> {
        match self {
            Self::Free(free) => Some(free),
            _ => None,
        }
    }

    /// Returns the key if this is a leaf or inner node.
    #[must_use]
    pub const fn key(&self) -> Option<u128> {
        match self {
            Self::Inner(inner) => Some(inner.key),
            Self::Leaf(leaf) => Some(leaf.key),
            _ => None,
        }
    }

    /// Creates an inner node variant.
    #[must_use]
    #[inline]
    pub const fn new_inner(inner: InnerNode) -> Self {
        Self::Inner(inner)
    }

    /// Creates a leaf node variant.
    #[must_use]
    #[inline]
    pub const fn new_leaf(leaf: LeafNode) -> Self {
        Self::Leaf(leaf)
    }

    /// Creates a free node variant.
    #[must_use]
    #[inline]
    pub const fn new_free(free: FreeNode) -> Self {
        Self::Free(free)
    }
}

// ============================================================================
// Order Book Side Header
// ============================================================================

/// Order book side header (bids or asks).
///
/// This structure is stored at the beginning of the order book account
/// and contains metadata about the tree. The actual nodes follow
/// immediately after in the account data.
///
/// # PDA Seeds
///
/// - Bids: `["bids", market.key()]`
/// - Asks: `["asks", market.key()]`
#[account(zero_copy(unsafe))]
#[derive(Debug)]
#[repr(C)]
pub struct OrderBookSideHeader {
    /// Bump seed for PDA derivation.
    pub bump: u8,
    /// Padding for alignment.
    pub padding: [u8; 7],
    /// Market this order book belongs to.
    pub market: Pubkey,
    /// Whether this is the bids side (1) or asks side (0).
    pub is_bids: u8,
    /// Padding for alignment.
    pub padding2: [u8; 7],
    /// Number of leaf nodes (orders) in the tree.
    pub leaf_count: u32,
    /// Head of the free list (SENTINEL if empty).
    pub free_list_head: u32,
    /// Root node index (SENTINEL if empty).
    pub root: u32,
    /// Padding for alignment.
    pub padding3: [u8; 4],
    /// Reserved for future use.
    pub reserved: [u8; 64],
}

impl OrderBookSideHeader {
    /// Header size in bytes.
    pub const SIZE: usize = ORDERBOOK_HEADER_SIZE;

    /// Derives the PDA for a bids order book.
    ///
    /// # Arguments
    ///
    /// * `market` - The market pubkey
    /// * `program_id` - The program ID
    #[must_use]
    pub fn derive_bids_pda(market: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[BIDS_SEED, market.as_ref()], program_id)
    }

    /// Derives the PDA for an asks order book.
    ///
    /// # Arguments
    ///
    /// * `market` - The market pubkey
    /// * `program_id` - The program ID
    #[must_use]
    pub fn derive_asks_pda(market: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[ASKS_SEED, market.as_ref()], program_id)
    }

    /// Returns true if this is the bids side.
    #[must_use]
    #[inline]
    pub const fn is_bids(&self) -> bool {
        self.is_bids != 0
    }

    /// Returns true if the tree is empty.
    #[must_use]
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.leaf_count == 0
    }

    /// Returns the number of orders in the book.
    #[must_use]
    #[inline]
    pub const fn len(&self) -> u32 {
        self.leaf_count
    }
}

// ============================================================================
// Critbit Tree Operations
// ============================================================================

/// Finds the critical bit position where two keys differ.
///
/// Returns the position of the most significant differing bit (0-127).
#[must_use]
#[inline]
pub const fn critbit(a: u128, b: u128) -> u32 {
    let diff = a ^ b;
    if diff == 0 {
        128 // Keys are equal
    } else {
        127 - diff.leading_zeros()
    }
}

/// Gets the bit at the given position in a key.
///
/// # Arguments
///
/// * `key` - The key to examine
/// * `bit` - The bit position (0-127, where 127 is MSB)
#[must_use]
#[inline]
pub const fn get_bit(key: u128, bit: u32) -> usize {
    ((key >> bit) & 1) as usize
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_id_bid_encoding() {
        let price = 1000u64;
        let seq_num = 42u64;
        let order_id = OrderId::new_bid(price, seq_num);

        assert_eq!(order_id.price(true), price);
        assert_eq!(order_id.seq_num(), seq_num);
    }

    #[test]
    fn test_order_id_ask_encoding() {
        let price = 1000u64;
        let seq_num = 42u64;
        let order_id = OrderId::new_ask(price, seq_num);

        assert_eq!(order_id.price(false), price);
        assert_eq!(order_id.seq_num(), seq_num);
    }

    #[test]
    fn test_order_id_bid_ordering() {
        // Higher prices should have lower keys (sort first)
        let high_price = OrderId::new_bid(2000, 1);
        let low_price = OrderId::new_bid(1000, 1);

        assert!(high_price.get() < low_price.get());
    }

    #[test]
    fn test_order_id_ask_ordering() {
        // Lower prices should have lower keys (sort first)
        let high_price = OrderId::new_ask(2000, 1);
        let low_price = OrderId::new_ask(1000, 1);

        assert!(low_price.get() < high_price.get());
    }

    #[test]
    fn test_order_id_time_priority() {
        // Same price, earlier time should sort first
        let earlier = OrderId::new_bid(1000, 1);
        let later = OrderId::new_bid(1000, 2);

        assert!(earlier.get() < later.get());
    }

    #[test]
    fn test_inner_node_creation() {
        let node = InnerNode::new(64, 0x1234, [1, 2]);

        assert_eq!(node.prefix_len, 64);
        assert_eq!(node.key, 0x1234);
        assert_eq!(node.child(0), 1);
        assert_eq!(node.child(1), 2);
    }

    #[test]
    fn test_leaf_node_creation() {
        let owner = Pubkey::new_unique();
        let node = LeafNode::new(5, TimeInForce::PostOnly, 0xABCD, owner, 100, 999);

        assert_eq!(node.owner_slot, 5);
        assert_eq!(node.time_in_force, TimeInForce::PostOnly);
        assert_eq!(node.key, 0xABCD);
        assert_eq!(node.owner, owner);
        assert_eq!(node.quantity, 100);
        assert_eq!(node.client_order_id, 999);
    }

    #[test]
    fn test_free_node_creation() {
        let node = FreeNode::new(42);
        assert_eq!(node.next, 42);
        assert!(!node.is_last());

        let last_node = FreeNode::new(SENTINEL);
        assert_eq!(last_node.next, SENTINEL);
        assert!(last_node.is_last());
    }

    #[test]
    fn test_any_node_tag_detection() {
        let node = AnyNode::default();
        assert_eq!(node.tag(), NodeTag::Uninitialized);
        assert!(node.is_uninitialized());

        let inner_node = AnyNode::new_inner(InnerNode::new(0, 0, [0, 0]));
        assert!(inner_node.is_inner());
        assert!(!inner_node.is_leaf());
        assert!(!inner_node.is_free());

        let leaf_node = AnyNode::new_leaf(LeafNode::new(
            0,
            TimeInForce::GoodTilCancelled,
            0,
            Pubkey::default(),
            0,
            0,
        ));
        assert!(!leaf_node.is_inner());
        assert!(leaf_node.is_leaf());
        assert!(!leaf_node.is_free());

        let free_node = AnyNode::new_free(FreeNode::new(0));
        assert!(!free_node.is_inner());
        assert!(!free_node.is_leaf());
        assert!(free_node.is_free());
    }

    #[test]
    fn test_critbit_function() {
        // Same keys
        assert_eq!(critbit(0, 0), 128);
        assert_eq!(critbit(0xFF, 0xFF), 128);

        // Differ at MSB
        assert_eq!(critbit(0, 1 << 127), 127);

        // Differ at LSB
        assert_eq!(critbit(0, 1), 0);

        // Differ at bit 64
        assert_eq!(critbit(0, 1 << 64), 64);
    }

    #[test]
    fn test_get_bit_function() {
        let key: u128 = 0b1010;

        assert_eq!(get_bit(key, 0), 0);
        assert_eq!(get_bit(key, 1), 1);
        assert_eq!(get_bit(key, 2), 0);
        assert_eq!(get_bit(key, 3), 1);
    }

    #[test]
    fn test_orderbook_pda_derivation() {
        let market = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();

        let (bids_pda1, bump1) = OrderBookSideHeader::derive_bids_pda(&market, &program_id);
        let (bids_pda2, bump2) = OrderBookSideHeader::derive_bids_pda(&market, &program_id);

        // Same inputs produce same outputs
        assert_eq!(bids_pda1, bids_pda2);
        assert_eq!(bump1, bump2);

        // Bids and asks have different PDAs
        let (asks_pda, _) = OrderBookSideHeader::derive_asks_pda(&market, &program_id);
        assert_ne!(bids_pda1, asks_pda);
    }

    #[test]
    fn test_time_in_force_conversion() {
        assert_eq!(TimeInForce::from(0), TimeInForce::GoodTilCancelled);
        assert_eq!(TimeInForce::from(1), TimeInForce::ImmediateOrCancel);
        assert_eq!(TimeInForce::from(2), TimeInForce::FillOrKill);
        assert_eq!(TimeInForce::from(3), TimeInForce::PostOnly);
        assert_eq!(TimeInForce::from(255), TimeInForce::GoodTilCancelled);
    }

    #[test]
    fn test_node_tag_conversion() {
        assert_eq!(NodeTag::from(0), NodeTag::Uninitialized);
        assert_eq!(NodeTag::from(1), NodeTag::InnerNode);
        assert_eq!(NodeTag::from(2), NodeTag::LeafNode);
        assert_eq!(NodeTag::from(3), NodeTag::FreeNode);
        assert_eq!(NodeTag::from(4), NodeTag::LastFreeNode);
        assert_eq!(NodeTag::from(255), NodeTag::Uninitialized);
    }

    #[test]
    fn test_header_is_empty() {
        let header = OrderBookSideHeader {
            bump: 255,
            padding: [0; 7],
            market: Pubkey::default(),
            is_bids: 1,
            padding2: [0; 7],
            leaf_count: 0,
            free_list_head: SENTINEL,
            root: SENTINEL,
            padding3: [0; 4],
            reserved: [0; 64],
        };

        assert!(header.is_empty());
        assert_eq!(header.len(), 0);
        assert!(header.is_bids());
    }
}
