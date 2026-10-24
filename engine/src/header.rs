//! Market header.

use bytemuck::{Pod, Zeroable};

/// Size of [`MarketHeader`] in bytes.
pub const HEADER_SIZE: usize = 400;

/// Magic value identifying an initialized market ("MTCHBOOK").
pub const MARKET_MAGIC: u64 = u64::from_le_bytes(*b"MTCHBOOK");

/// Operational status of a market.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MarketStatus {
    /// Everything allowed.
    Active = 0,
    /// Only cancels, withdrawals and fee collection.
    CancelOnly = 1,
    /// Only withdrawals.
    Closed = 2,
}

impl MarketStatus {
    /// Decodes a stored status.
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(Self::Active),
            1 => Some(Self::CancelOnly),
            2 => Some(Self::Closed),
            _ => None,
        }
    }
}

/// Fixed header at the start of every market buffer.
///
/// Prices are integers in quote lots per base lot, so a fill of `n` lots at
/// price `p` moves `n * base_lot_size` base atoms and
/// `n * p * quote_lot_size` quote atoms, with no rounding.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct MarketHeader {
    /// [`MARKET_MAGIC`] once initialized.
    pub magic: u64,
    /// Base mint (opaque to the engine).
    pub base_mint: [u8; 32],
    /// Quote mint (opaque to the engine).
    pub quote_mint: [u8; 32],
    /// Base vault (opaque to the engine).
    pub base_vault: [u8; 32],
    /// Quote vault (opaque to the engine).
    pub quote_vault: [u8; 32],
    /// Market authority (opaque to the engine).
    pub authority: [u8; 32],
    /// Base atoms per base lot.
    pub base_lot_size: u64,
    /// Quote atoms per quote lot.
    pub quote_lot_size: u64,
    /// Minimum order size in base lots.
    pub min_base_lots: u64,
    /// Last assigned order sequence number.
    pub seq_num: u64,
    /// Quote atoms of fees owed to the market authority.
    pub fees_accrued: u64,
    /// Lifetime traded base lots.
    pub total_base_lots_traded: u64,
    /// Lifetime traded quote atoms.
    pub total_quote_atoms_traded: u64,
    /// Taker fee in basis points.
    pub taker_fee_bps: u32,
    /// Maker rebate in basis points (never above the taker fee).
    pub maker_rebate_bps: u32,
    /// Root of the bids tree.
    pub bids_root: u32,
    /// Root of the asks tree.
    pub asks_root: u32,
    /// Root of the traders tree.
    pub traders_root: u32,
    /// Head of the free list.
    pub free_head: u32,
    /// Nodes on the free list.
    pub free_list_len: u32,
    /// Nodes ever handed out from the tail of the buffer.
    pub next_unused: u32,
    /// Resting orders.
    pub order_count: u32,
    /// Seats.
    pub trader_count: u32,
    /// [`MarketStatus`] as u32.
    pub status: u32,
    /// Free for the host program (e.g. a PDA bump).
    pub host_bump: u32,
    /// Cached best bid node, or NIL.
    pub best_bid: u32,
    /// Cached best ask node, or NIL.
    pub best_ask: u32,
    /// Reserved for upgrades.
    pub reserved: [u64; 15],
}

const _: () = assert!(core::mem::size_of::<MarketHeader>() == HEADER_SIZE);
