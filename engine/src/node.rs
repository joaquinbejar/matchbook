//! Fixed-size nodes stored in the market buffer.
//!
//! Every node is 88 bytes and starts with the same 40-byte red-black tree
//! prefix, so orders and trader seats live in the same pool and the tree code
//! never needs to know what it is ordering.

use bytemuck::{Pod, Zeroable};

/// Size of every node in bytes.
pub const NODE_SIZE: usize = 88;

/// Null node index.
pub const NIL: u32 = u32::MAX;

/// Low bits of an order id that hold the order's node index, so a cancel
/// finds its order in O(1) without scanning or a client-side hint.
pub const ORDER_INDEX_BITS: u32 = 24;

/// Largest sequence number an order id can carry (2^40 - 1).
pub const MAX_ORDER_SEQ: u64 = (1 << (64 - ORDER_INDEX_BITS)) - 1;

/// Builds the public order id from a sequence number and a node index.
#[inline]
pub const fn order_id(seq: u64, index: u32) -> u64 {
    (seq << ORDER_INDEX_BITS) | index as u64
}

/// Splits a public order id into `(sequence number, node index)`.
#[inline]
pub const fn split_order_id(id: u64) -> (u64, u32) {
    (
        id >> ORDER_INDEX_BITS,
        (id & ((1 << ORDER_INDEX_BITS) - 1)) as u32,
    )
}

pub(crate) const FLAG_RED: u32 = 1;
pub(crate) const FLAG_ASK: u32 = 1 << 1;
pub(crate) const FLAG_ABOVE: u32 = 1 << 2;

/// Kind of a node, stored in its first four bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NodeTag {
    /// Never used (zeroed tail of the buffer).
    Uninitialized = 0,
    /// On the free list.
    Free = 1,
    /// Resting order.
    Order = 2,
    /// Trader seat with balances.
    Trader = 3,
    /// Key allowed to act for a seat.
    Delegate = 4,
    /// Order waiting for a trigger price.
    Stop = 5,
}

/// Untyped node with the shared tree prefix. Cast to [`OrderLeaf`] or
/// [`TraderLeaf`] with [`bytemuck::cast_ref`].
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct Node {
    /// [`NodeTag`] as u32.
    pub tag: u32,
    /// Bit 0: red. Bit 1: ask (orders only).
    pub flags: u32,
    /// Left child.
    pub left: u32,
    /// Right child.
    pub right: u32,
    /// Parent.
    pub parent: u32,
    /// Type-specific (free list next, trader order head).
    pub extra: u32,
    /// Low half of the key.
    pub key_lo: u64,
    /// High half of the key.
    pub key_hi: u64,
    /// Type-specific payload.
    pub payload: [u64; 6],
}

impl Node {
    /// Tree key.
    #[inline]
    pub fn key(&self) -> u128 {
        (u128::from(self.key_hi) << 64) | u128::from(self.key_lo)
    }

    #[inline]
    pub(crate) fn set_key(&mut self, key: u128) {
        self.key_lo = key as u64;
        self.key_hi = (key >> 64) as u64;
    }

    /// True when the node holds this tag.
    #[inline]
    pub fn is(&self, tag: NodeTag) -> bool {
        self.tag == tag as u32
    }
}

/// A resting order.
///
/// Book key: high 64 bits are the price; low 64 bits are the sequence number
/// for asks and its bitwise complement for bids, so the best ask is the
/// minimum key, the best bid is the maximum key, and earlier orders win ties.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct OrderLeaf {
    /// Always [`NodeTag::Order`].
    pub tag: u32,
    /// Tree flags.
    pub flags: u32,
    /// Tree link.
    pub left: u32,
    /// Tree link.
    pub right: u32,
    /// Tree link.
    pub parent: u32,
    /// For a reverse order, the node index of the order it flips into (its
    /// partner on the other side) plus one, or 0 if it has none. 0 for other
    /// orders. Use [`OrderLeaf::partner`].
    pub extra: u32,
    /// Low half of the book key.
    pub key_lo: u64,
    /// High half of the book key (the price).
    pub key_hi: u64,
    /// Owning trader node.
    pub trader: u32,
    /// Previous order of the same trader.
    pub prev: u32,
    /// Next order of the same trader.
    pub next: u32,
    /// Reverse spread in parts per million, or 0 for an ordinary order.
    pub reverse_spread: u32,
    /// Remaining quantity in base lots.
    pub base_lots: u64,
    /// Client-supplied id.
    pub client_order_id: u64,
    /// Unix timestamp at or after which the order is dead (0 = never).
    pub expires_at: u64,
    /// Engine sequence number, which is the order id.
    pub seq: u64,
}

impl OrderLeaf {
    /// Limit price in quote lots per base lot.
    #[inline]
    pub fn price(&self) -> u64 {
        self.key_hi
    }

    /// True for asks.
    #[inline]
    pub fn is_ask(&self) -> bool {
        self.flags & FLAG_ASK != 0
    }

    /// The reverse order this one flips into, or [`NIL`].
    #[inline]
    pub fn partner(&self) -> u32 {
        self.extra.wrapping_sub(1)
    }

    /// Links the partner (or [`NIL`] to unlink).
    #[inline]
    pub fn set_partner(&mut self, i: u32) {
        self.extra = i.wrapping_add(1);
    }

    /// True if the order is dead at `now`.
    #[inline]
    pub fn is_expired(&self, now: u64) -> bool {
        self.expires_at != 0 && now >= self.expires_at
    }
}

/// An order that waits until the last trade price reaches a trigger, then
/// is placed for its seat. Funds for the order are locked while it waits.
///
/// Stops live in two trees, one for each trigger direction, keyed by the
/// trigger price and then the sequence number (complemented for stops below,
/// so that the first to trigger is the minimum above and the maximum below,
/// earliest first). They share the seat's order list, so `trader`, `prev`
/// and `next` sit at the same offsets as in [`OrderLeaf`].
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct StopLeaf {
    /// Always [`NodeTag::Stop`].
    pub tag: u32,
    /// Tree flags, plus the order side and the trigger direction.
    pub flags: u32,
    /// Tree link.
    pub left: u32,
    /// Tree link.
    pub right: u32,
    /// Tree link.
    pub parent: u32,
    /// Node index of the one-cancels-other partner plus one, or 0.
    pub extra: u32,
    /// Low half of the key: the sequence number, complemented below.
    pub key_lo: u64,
    /// High half of the key: the trigger price.
    pub key_hi: u64,
    /// Owning trader node.
    pub trader: u32,
    /// Previous order or stop of the same trader.
    pub prev: u32,
    /// Next order or stop of the same trader.
    pub next: u32,
    /// [`crate::OrderType`] of the order.
    pub order_type: u8,
    /// [`crate::SelfTradeBehavior`] of the order.
    pub self_trade: u8,
    /// Match limit of the order.
    pub match_limit: u16,
    /// Limit price of the order.
    pub price: u64,
    /// Size of the order in base lots.
    pub base_lots: u64,
    /// Client-supplied id.
    pub client_order_id: u64,
    /// Unix timestamp at which the stop, and the order it places, die
    /// (0 = never).
    pub expires_at: u64,
}

impl StopLeaf {
    /// Trigger price.
    #[inline]
    pub fn trigger_price(&self) -> u64 {
        self.key_hi
    }

    /// True if it triggers when the last price is at or above the trigger.
    #[inline]
    pub fn is_above(&self) -> bool {
        self.flags & FLAG_ABOVE != 0
    }

    /// True for a sell order.
    #[inline]
    pub fn is_ask(&self) -> bool {
        self.flags & FLAG_ASK != 0
    }

    /// Sequence number.
    #[inline]
    pub fn seq(&self) -> u64 {
        if self.is_above() {
            self.key_lo
        } else {
            !self.key_lo
        }
    }

    /// The one-cancels-other partner, or [`NIL`].
    #[inline]
    pub fn partner(&self) -> u32 {
        self.extra.wrapping_sub(1)
    }
}

const _: () = {
    assert!(core::mem::size_of::<StopLeaf>() == NODE_SIZE);
    assert!(core::mem::offset_of!(StopLeaf, trader) == core::mem::offset_of!(OrderLeaf, trader));
    assert!(core::mem::offset_of!(StopLeaf, prev) == core::mem::offset_of!(OrderLeaf, prev));
    assert!(core::mem::offset_of!(StopLeaf, next) == core::mem::offset_of!(OrderLeaf, next));
};

/// A trader seat, keyed by the first 16 bytes of the owner.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct TraderLeaf {
    /// Always [`NodeTag::Trader`].
    pub tag: u32,
    /// Tree flags.
    pub flags: u32,
    /// Tree link.
    pub left: u32,
    /// Tree link.
    pub right: u32,
    /// Tree link.
    pub parent: u32,
    /// First order of this trader, or [`NIL`].
    pub order_head: u32,
    /// Low half of the key.
    pub key_lo: u64,
    /// High half of the key.
    pub key_hi: u64,
    /// Owner public key.
    pub owner: [u8; 32],
    /// Base atoms available to trade or withdraw.
    pub base_free: u64,
    /// Quote atoms available to trade or withdraw.
    pub quote_free: u64,
}

impl TraderLeaf {
    pub(crate) fn key_for(owner: &[u8; 32]) -> u128 {
        let mut b = [0u8; 16];
        b.copy_from_slice(&owner[..16]);
        u128::from_be_bytes(b)
    }
}

/// Bits of [`TraderLeaf::flags`] that count the seat's delegates.
pub(crate) const DELEGATE_COUNT_SHIFT: u32 = 8;
pub(crate) const DELEGATE_COUNT_MASK: u32 = 0xff << DELEGATE_COUNT_SHIFT;

impl TraderLeaf {
    /// Number of delegates acting for this seat.
    #[inline]
    pub fn delegate_count(&self) -> u32 {
        (self.flags & DELEGATE_COUNT_MASK) >> DELEGATE_COUNT_SHIFT
    }
}

/// A key allowed to act for a seat (a session key or a trading bot), stored
/// in the traders tree next to the seats. It can trade or cancel, as its
/// permissions allow, but never withdraw or manage delegates.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct DelegateLeaf {
    /// [`NodeTag::Delegate`].
    pub tag: u32,
    /// Tree color.
    pub flags: u32,
    /// Left child.
    pub left: u32,
    /// Right child.
    pub right: u32,
    /// Parent.
    pub parent: u32,
    /// Seat this delegate acts for.
    pub trader: u32,
    /// Low half of the tree key.
    pub key_lo: u64,
    /// High half of the tree key.
    pub key_hi: u64,
    /// The delegate's public key.
    pub delegate: [u8; 32],
    /// Unix time after which the delegate can no longer act, or 0.
    pub expires_at: u64,
    /// [`crate::permission`] bits.
    pub permissions: u64,
}

const _: () = assert!(core::mem::size_of::<Node>() == NODE_SIZE);
const _: () = assert!(core::mem::size_of::<DelegateLeaf>() == NODE_SIZE);
const _: () = assert!(core::mem::size_of::<OrderLeaf>() == NODE_SIZE);
const _: () = assert!(core::mem::size_of::<TraderLeaf>() == NODE_SIZE);
const _: () = assert!(core::mem::align_of::<Node>() == 8);
