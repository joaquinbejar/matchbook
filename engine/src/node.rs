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

pub(crate) const FLAG_RED: u32 = 1;
pub(crate) const FLAG_ASK: u32 = 1 << 1;

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
    /// Unused.
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
    /// Reserved for order kinds (reverse, pegged).
    pub kind: u32,
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

    /// True if the order is dead at `now`.
    #[inline]
    pub fn is_expired(&self, now: u64) -> bool {
        self.expires_at != 0 && now >= self.expires_at
    }
}

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

const _: () = assert!(core::mem::size_of::<Node>() == NODE_SIZE);
const _: () = assert!(core::mem::size_of::<OrderLeaf>() == NODE_SIZE);
const _: () = assert!(core::mem::size_of::<TraderLeaf>() == NODE_SIZE);
const _: () = assert!(core::mem::align_of::<Node>() == 8);
