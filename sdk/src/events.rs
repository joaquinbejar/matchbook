//! Decoding of the events the program logs with `sol_log_data`.

use matchbook_program::engine::{OutReason, Side};
use solana_pubkey::Pubkey;

/// A maker order traded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fill {
    /// Owner of the maker order.
    pub maker: Pubkey,
    /// Maker order id.
    pub maker_order_id: u64,
    /// Maker client order id.
    pub maker_client_order_id: u64,
    /// Execution price (the maker's price).
    pub price: u64,
    /// Lots traded.
    pub base_lots: u64,
    /// Lots left on the maker order (0 when it is gone).
    pub maker_remaining: u64,
}

/// An order or stop left the book other than by a fill or its owner's cancel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Out {
    /// Owner of the order.
    pub owner: Pubkey,
    /// Order or stop id.
    pub order_id: u64,
    /// Client order id.
    pub client_order_id: u64,
    /// Side.
    pub side: Side,
    /// Limit price.
    pub price: u64,
    /// Lots that left.
    pub base_lots: u64,
    /// Why it left.
    pub reason: OutReason,
}

/// A reverse order traded and re-posted the filled size on the other side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reversed {
    /// Owner of the order.
    pub owner: Pubkey,
    /// The order that received the size (new, or the existing partner).
    pub order_id: u64,
    /// Client order id.
    pub client_order_id: u64,
    /// Side of the re-posted order.
    pub side: Side,
    /// Its price.
    pub price: u64,
    /// Lots added.
    pub base_lots: u64,
    /// Unix expiry, or 0.
    pub expires_at: u64,
}

/// Any MatchBook event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// See [`Fill`].
    Fill(Fill),
    /// See [`Out`].
    Out(Out),
    /// See [`Reversed`].
    Reversed(Reversed),
}

/// The payload held an unknown tag or was cut short.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BadEvent {
    /// Byte offset of the event.
    pub offset: usize,
}

impl core::fmt::Display for BadEvent {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "malformed MatchBook event at byte {}", self.offset)
    }
}

impl std::error::Error for BadEvent {}

const FILL_LEN: usize = 73;
const OUT_LEN: usize = 67;
const REVERSED_LEN: usize = 74;

fn u64_at(d: &[u8], i: usize) -> u64 {
    d.get(i..i + 8)
        .and_then(|b| b.try_into().ok())
        .map_or(0, u64::from_le_bytes)
}

fn key_at(d: &[u8], i: usize) -> Pubkey {
    d.get(i..i + 32)
        .and_then(|b| <[u8; 32]>::try_from(b).ok())
        .map_or_else(Pubkey::default, Pubkey::new_from_array)
}

fn side_at(d: &[u8], i: usize) -> Side {
    if d.get(i) == Some(&1) {
        Side::Ask
    } else {
        Side::Bid
    }
}

fn reason(v: u8) -> OutReason {
    match v {
        0 => OutReason::Filled,
        1 => OutReason::Cancelled,
        2 => OutReason::Expired,
        3 => OutReason::SelfTrade,
        4 => OutReason::Triggered,
        _ => OutReason::OcoCancelled,
    }
}

/// Decodes the events packed in one `sol_log_data` payload.
pub fn decode(data: &[u8]) -> Result<Vec<Event>, BadEvent> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let fits = |len: usize| i + len <= data.len();
        match data.get(i) {
            Some(1) if fits(FILL_LEN) => {
                out.push(Event::Fill(Fill {
                    maker: key_at(data, i + 1),
                    maker_order_id: u64_at(data, i + 33),
                    maker_client_order_id: u64_at(data, i + 41),
                    price: u64_at(data, i + 49),
                    base_lots: u64_at(data, i + 57),
                    maker_remaining: u64_at(data, i + 65),
                }));
                i += FILL_LEN;
            }
            Some(2) if fits(OUT_LEN) => {
                out.push(Event::Out(Out {
                    owner: key_at(data, i + 1),
                    order_id: u64_at(data, i + 33),
                    client_order_id: u64_at(data, i + 41),
                    side: side_at(data, i + 49),
                    price: u64_at(data, i + 50),
                    base_lots: u64_at(data, i + 58),
                    reason: reason(data.get(i + 66).copied().unwrap_or(0)),
                }));
                i += OUT_LEN;
            }
            Some(3) if fits(REVERSED_LEN) => {
                out.push(Event::Reversed(Reversed {
                    owner: key_at(data, i + 1),
                    order_id: u64_at(data, i + 33),
                    client_order_id: u64_at(data, i + 41),
                    side: side_at(data, i + 49),
                    price: u64_at(data, i + 50),
                    base_lots: u64_at(data, i + 58),
                    expires_at: u64_at(data, i + 66),
                }));
                i += REVERSED_LEN;
            }
            _ => return Err(BadEvent { offset: i }),
        }
    }
    Ok(out)
}
