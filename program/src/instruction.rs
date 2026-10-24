//! Instruction discriminators, account orders and data layouts.
//!
//! Every instruction starts with an 8-byte [`Header`]: discriminator, three
//! instruction-specific bytes `a`, `b`, `c`, and a `u32` `hint`. For trader
//! instructions `hint` is the trader's seat index (any value, e.g.
//! `u32::MAX`, falls back to a lookup). All integers are little-endian and
//! every payload field is naturally aligned.

use bytemuck::{Pod, Zeroable};
use matchbook_engine::{OrderParams, OrderType, SelfTradeBehavior, Side};

use crate::error::MatchbookError;

/// Instruction discriminators (first byte of instruction data).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Ix {
    /// Accounts: payer (s,w), market (s,w), base_mint, quote_mint,
    /// base_vault (w), quote_vault (w), vault_authority, base_token_program,
    /// quote_token_program, associated_token_program, system_program.
    /// Payload: [`CreateMarketArgs`]. The market account is either a fresh
    /// keypair (created here with `initial_nodes`, at most 10 KiB) or an
    /// account the client already created, owned by this program and zeroed.
    CreateMarket = 0,
    /// Accounts: payer (s,w), owner (s), market (w), system_program.
    ClaimSeat = 1,
    /// Accounts: owner (s), market (w), user_token (w), vault (w), mint, token_program.
    /// Payload: amount u64.
    Deposit = 2,
    /// Accounts: owner (s), market (w), user_token (w), vault (w), mint,
    /// vault_authority, token_program. Payload: amount u64.
    Withdraw = 3,
    /// Accounts: owner (s,w), market (w), system_program.
    /// Payload: [`OrderArgs`]. Return data: the order id (u64, 0 if
    /// nothing rested).
    PlaceOrder = 4,
    /// Accounts: owner (s), market (w). Payload: order_id u64.
    CancelOrder = 5,
    /// Accounts: owner (s), market (w). `a`: side (0 bid, 1 ask, 2 both).
    CancelAll = 6,
    /// Accounts: owner (s), market (w). Payload: client_order_id u64.
    CancelByClientId = 7,
    /// Accounts: owner (s), market (w). Payload: order_id u64, new_base_lots u64.
    ReduceOrder = 8,
    /// Accounts: owner (s,w), market (w), system_program.
    /// `a`: cancel_all (0 none, 1 bids, 2 asks, 3 both), `b`: n_cancels,
    /// `c`: n_orders. Payload: n_cancels × order_id u64, then n_orders ×
    /// [`OrderArgs`]. Return data: one order id (u64, 0 if nothing rested)
    /// per order.
    BatchUpdate = 9,
    /// Accounts: user (s), market (w), user_base (w), user_quote (w),
    /// base_vault (w), quote_vault (w), base_mint, quote_mint, vault_authority,
    /// base_token_program, quote_token_program. `a`: side (0 buy base,
    /// 1 sell base). Payload: [`SwapArgs`].
    Swap = 10,
    /// Accounts: authority (s), market (w), destination (w), quote_vault (w),
    /// quote_mint, vault_authority, token_program.
    CollectFees = 11,
    /// Accounts: authority (s), market (w). `a`: status.
    SetStatus = 12,
    /// Accounts: market (w). `hint`: max orders to prune. Permissionless.
    PruneExpired = 13,
    /// Lets another key act for the owner's seat: a session key for an app
    /// or a bot's hot key. It can trade or cancel as its permissions allow,
    /// but never withdraw. Granting an existing delegate again updates it.
    /// Accounts: owner (s,w), market (w), system_program. Payload:
    /// [`DelegateArgs`].
    SetDelegate = 14,
    /// Revokes a delegate. Signed by the seat's owner or by the delegate
    /// itself. Accounts: signer (s), market (w). Payload: delegate key
    /// (32 bytes).
    RemoveDelegate = 15,
    /// Places a stop: an order that waits until a trade reaches its trigger
    /// price, with its funds locked meanwhile. Trades set off stops
    /// themselves, so no keeper is needed. Cancel it like an order.
    /// Accounts: owner (s,w), market (w), system_program. `a`: direction
    /// (0 above, 1 below). Payload: [`StopArgs`]. Return data: the stop id.
    PlaceStop = 16,
    /// Places stops whose trigger has been reached beyond those every trade
    /// places itself. Accounts: market (w). `hint`: max stops. Permissionless.
    TriggerStops = 17,
}

impl Ix {
    /// Decodes a discriminator.
    pub const fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::CreateMarket,
            1 => Self::ClaimSeat,
            2 => Self::Deposit,
            3 => Self::Withdraw,
            4 => Self::PlaceOrder,
            5 => Self::CancelOrder,
            6 => Self::CancelAll,
            7 => Self::CancelByClientId,
            8 => Self::ReduceOrder,
            9 => Self::BatchUpdate,
            10 => Self::Swap,
            11 => Self::CollectFees,
            12 => Self::SetStatus,
            13 => Self::PruneExpired,
            14 => Self::SetDelegate,
            15 => Self::RemoveDelegate,
            16 => Self::PlaceStop,
            17 => Self::TriggerStops,
            _ => return None,
        })
    }
}

/// Size of the header every instruction starts with.
pub const HEADER_LEN: usize = 8;

/// The 8-byte header every instruction starts with: the discriminator,
/// three instruction-specific bytes and a `u32` that is the seat hint for
/// trader instructions. It keeps every payload that follows 8-byte aligned,
/// which lets the program read arguments in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// Instruction.
    pub ix: Ix,
    /// Instruction-specific bytes.
    pub a: u8,
    /// Instruction-specific bytes.
    pub b: u8,
    /// Instruction-specific bytes.
    pub c: u8,
    /// Seat hint (or another instruction-specific `u32`).
    pub hint: u32,
}

impl Header {
    /// Splits instruction data into its header and payload.
    pub fn parse(data: &[u8]) -> Result<(Self, Reader<'_>), MatchbookError> {
        let (h, rest) = data
            .split_first_chunk::<HEADER_LEN>()
            .ok_or(MatchbookError::InvalidInstruction)?;
        let [tag, a, b, c, h0, h1, h2, h3] = *h;
        let ix = Ix::from_u8(tag).ok_or(MatchbookError::InvalidInstruction)?;
        Ok((
            Self {
                ix,
                a,
                b,
                c,
                hint: u32::from_le_bytes([h0, h1, h2, h3]),
            },
            Reader::new(rest),
        ))
    }

    /// Encodes the header.
    pub const fn encode(ix: Ix, a: u8, b: u8, c: u8, hint: u32) -> [u8; HEADER_LEN] {
        let h = hint.to_le_bytes();
        [ix as u8, a, b, c, h[0], h[1], h[2], h[3]]
    }
}

/// Little-endian cursor over an instruction payload. Reads are in place when
/// the data is aligned, as it is in the runtime's input buffer.
pub struct Reader<'a> {
    data: &'a [u8],
}

impl<'a> Reader<'a> {
    /// Wraps `data`.
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Reads one plain-old-data value.
    #[inline(always)]
    pub fn pod<T: Pod>(&mut self) -> Result<T, MatchbookError> {
        let n = core::mem::size_of::<T>();
        if self.data.len() < n {
            return Err(MatchbookError::InvalidInstructionData);
        }
        let (head, rest) = self.data.split_at(n);
        self.data = rest;
        Ok(match bytemuck::try_from_bytes::<T>(head) {
            Ok(v) => *v,
            Err(_) => bytemuck::pod_read_unaligned(head),
        })
    }

    /// Reads a u64.
    #[inline(always)]
    pub fn u64(&mut self) -> Result<u64, MatchbookError> {
        self.pod()
    }

    /// Looks over the [`OrderArgs`] following `skip` u64 values without
    /// consuming them.
    #[allow(clippy::chunks_exact_to_as_chunks)]
    pub fn scan_orders(&self, skip: usize) -> OrderScan {
        let rest = self.data.get(skip.saturating_mul(8)..).unwrap_or_default();
        let read = |c: &[u8]| match bytemuck::try_from_bytes::<OrderArgs>(c) {
            Ok(o) => (o.expires_at, o.reverse_spread),
            Err(_) => {
                let o = bytemuck::pod_read_unaligned::<OrderArgs>(c);
                (o.expires_at, o.reverse_spread)
            }
        };
        let (mut expiry, mut any_reverse) = (0u64, 0u32);
        for c in rest.chunks_exact(OrderArgs::LEN) {
            let (e, r) = read(c);
            expiry |= e;
            any_reverse |= r;
        }
        // Counting is slower than the scan above; only reverse orders pay it.
        let reverse = if any_reverse == 0 {
            0
        } else {
            rest.chunks_exact(OrderArgs::LEN)
                .filter(|c| read(c).1 != 0)
                .count() as u32
        };
        OrderScan {
            any_expiry: expiry != 0,
            reverse,
        }
    }
}

/// What [`Reader::scan_orders`] finds in a batch's orders.
#[derive(Debug, Clone, Copy, Default)]
pub struct OrderScan {
    /// Whether any order expires.
    pub any_expiry: bool,
    /// How many are reverse orders.
    pub reverse: u32,
}

/// Arguments of [`Ix::CreateMarket`].
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Pod, Zeroable)]
pub struct CreateMarketArgs {
    /// Base atoms per lot.
    pub base_lot_size: u64,
    /// Quote atoms per lot.
    pub quote_lot_size: u64,
    /// Minimum order size in lots.
    pub min_base_lots: u64,
    /// Taker fee (bps).
    pub taker_fee_bps: u32,
    /// Maker rebate (bps).
    pub maker_rebate_bps: u32,
    /// Nodes to preallocate.
    pub initial_nodes: u32,
    /// Must be zero.
    pub reserved: u32,
    /// Fee collector and status authority.
    pub authority: [u8; 32],
}

impl CreateMarketArgs {
    /// Encoded size.
    pub const LEN: usize = core::mem::size_of::<Self>();
}

/// Order arguments shared by [`Ix::PlaceOrder`] and [`Ix::BatchUpdate`].
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Pod, Zeroable)]
pub struct OrderArgs {
    /// 0 bid, 1 ask.
    pub side: u8,
    /// [`OrderType`] wire value.
    pub order_type: u8,
    /// [`SelfTradeBehavior`] wire value.
    pub self_trade: u8,
    /// Must be zero.
    pub reserved: u8,
    /// Max resting orders to touch (0 = default).
    pub match_limit: u32,
    /// Limit price (quote lots per base lot).
    pub price: u64,
    /// Size in base lots.
    pub base_lots: u64,
    /// Client order id.
    pub client_order_id: u64,
    /// Expiry unix timestamp (0 = never).
    pub expires_at: u64,
    /// Quote budget for IOC bids (0 = none).
    pub quote_budget: u64,
    /// Reverse spread in parts per million (0 = an ordinary order). A
    /// reverse order re-posts each fill on the other side, this far from
    /// the fill price.
    pub reverse_spread: u32,
    /// Must be zero.
    pub reserved2: u32,
}

impl OrderArgs {
    /// Encoded size.
    pub const LEN: usize = core::mem::size_of::<Self>();

    /// A plain limit order.
    pub const fn limit(side: u8, price: u64, base_lots: u64) -> Self {
        Self {
            side,
            order_type: 0,
            self_trade: 0,
            reserved: 0,
            match_limit: 0,
            price,
            base_lots,
            client_order_id: 0,
            expires_at: 0,
            quote_budget: 0,
            reverse_spread: 0,
            reserved2: 0,
        }
    }

    /// Converts to engine parameters.
    #[inline(always)]
    pub fn to_params(&self) -> Result<OrderParams, MatchbookError> {
        let bad = MatchbookError::InvalidInstructionData;
        Ok(OrderParams {
            side: Side::from_u8(self.side).ok_or(bad)?,
            price: self.price,
            base_lots: self.base_lots,
            order_type: OrderType::from_u8(self.order_type).ok_or(bad)?,
            self_trade: SelfTradeBehavior::from_u8(self.self_trade).ok_or(bad)?,
            client_order_id: self.client_order_id,
            expires_at: self.expires_at,
            quote_budget: (self.quote_budget != 0).then_some(self.quote_budget),
            match_limit: if self.match_limit == 0 {
                OrderParams::DEFAULT_MATCH_LIMIT
            } else {
                self.match_limit
            },
            reverse_spread: self.reverse_spread,
        })
    }
}

/// Arguments of [`Ix::SetDelegate`].
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Pod, Zeroable)]
pub struct DelegateArgs {
    /// The delegate's public key.
    pub delegate: [u8; 32],
    /// Unix time when the delegate stops being able to act (0 = never).
    pub expires_at: u64,
    /// [`matchbook_engine::permission`] bits.
    pub permissions: u64,
}

/// Arguments of [`Ix::PlaceStop`]. The direction travels in the header.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Pod, Zeroable)]
pub struct StopArgs {
    /// The order to place when triggered. No quote budget or reverse spread.
    pub order: OrderArgs,
    /// Trigger price (quote lots per base lot).
    pub trigger_price: u64,
    /// Id of one of the seat's stops to pair as one-cancels-other (0 = none).
    pub oco_with: u64,
}

/// Arguments of [`Ix::Swap`]. The side travels in the header.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Pod, Zeroable)]
pub struct SwapArgs {
    /// Ask: lots to sell. Bid: max lots to buy.
    pub base_lots: u64,
    /// Bid: max quote atoms to spend including fees.
    pub quote_budget: u64,
    /// Worst price.
    pub limit_price: u64,
    /// Minimum atoms received (base for bids, quote for asks).
    pub min_out: u64,
    /// Max resting orders to touch (0 = default).
    pub match_limit: u32,
    /// Must be zero.
    pub reserved: u32,
}

impl SwapArgs {
    /// Encoded size.
    pub const LEN: usize = core::mem::size_of::<Self>();
}

/// Instruction data builders for clients (requires the `client` feature).
#[cfg(feature = "client")]
pub mod build {
    use super::*;
    use std::vec::Vec;

    fn with(ix: Ix, a: u8, b: u8, c: u8, hint: u32, payload: &[u8]) -> Vec<u8> {
        let mut d = Vec::with_capacity(HEADER_LEN + payload.len());
        d.extend_from_slice(&Header::encode(ix, a, b, c, hint));
        d.extend_from_slice(payload);
        d
    }

    /// [`Ix::CreateMarket`] data.
    pub fn create_market(a: &CreateMarketArgs) -> Vec<u8> {
        with(Ix::CreateMarket, 0, 0, 0, 0, bytemuck::bytes_of(a))
    }

    /// [`Ix::ClaimSeat`] data.
    pub fn claim_seat() -> Vec<u8> {
        with(Ix::ClaimSeat, 0, 0, 0, 0, &[])
    }

    /// [`Ix::Deposit`] data.
    pub fn deposit(hint: u32, amount: u64) -> Vec<u8> {
        with(Ix::Deposit, 0, 0, 0, hint, &amount.to_le_bytes())
    }

    /// [`Ix::Withdraw`] data.
    pub fn withdraw(hint: u32, amount: u64) -> Vec<u8> {
        with(Ix::Withdraw, 0, 0, 0, hint, &amount.to_le_bytes())
    }

    /// [`Ix::PlaceOrder`] data.
    pub fn place_order(hint: u32, o: &OrderArgs) -> Vec<u8> {
        with(Ix::PlaceOrder, 0, 0, 0, hint, bytemuck::bytes_of(o))
    }

    /// [`Ix::CancelOrder`] data.
    pub fn cancel_order(hint: u32, order_id: u64) -> Vec<u8> {
        with(Ix::CancelOrder, 0, 0, 0, hint, &order_id.to_le_bytes())
    }

    /// [`Ix::CancelAll`] data (side: 0 bid, 1 ask, 2 both).
    pub fn cancel_all(hint: u32, side: u8) -> Vec<u8> {
        with(Ix::CancelAll, side, 0, 0, hint, &[])
    }

    /// [`Ix::CancelByClientId`] data.
    pub fn cancel_by_client_id(hint: u32, client_order_id: u64) -> Vec<u8> {
        with(
            Ix::CancelByClientId,
            0,
            0,
            0,
            hint,
            &client_order_id.to_le_bytes(),
        )
    }

    /// [`Ix::ReduceOrder`] data.
    pub fn reduce_order(hint: u32, order_id: u64, new_base_lots: u64) -> Vec<u8> {
        let mut d = with(Ix::ReduceOrder, 0, 0, 0, hint, &order_id.to_le_bytes());
        d.extend_from_slice(&new_base_lots.to_le_bytes());
        d
    }

    /// [`Ix::BatchUpdate`] data.
    pub fn batch_update(
        hint: u32,
        cancel_all: u8,
        cancels: &[u64],
        orders: &[OrderArgs],
    ) -> Vec<u8> {
        let mut d = with(
            Ix::BatchUpdate,
            cancel_all,
            cancels.len() as u8,
            orders.len() as u8,
            hint,
            &[],
        );
        for c in cancels {
            d.extend_from_slice(&c.to_le_bytes());
        }
        for o in orders {
            d.extend_from_slice(bytemuck::bytes_of(o));
        }
        d
    }

    /// [`Ix::Swap`] data (side: 0 buy base, 1 sell base).
    pub fn swap(side: u8, a: &SwapArgs) -> Vec<u8> {
        with(Ix::Swap, side, 0, 0, 0, bytemuck::bytes_of(a))
    }

    /// [`Ix::SetDelegate`] data.
    pub fn set_delegate(hint: u32, a: &DelegateArgs) -> Vec<u8> {
        with(Ix::SetDelegate, 0, 0, 0, hint, bytemuck::bytes_of(a))
    }

    /// [`Ix::RemoveDelegate`] data.
    pub fn remove_delegate(hint: u32, delegate: &[u8; 32]) -> Vec<u8> {
        with(Ix::RemoveDelegate, 0, 0, 0, hint, delegate)
    }

    /// [`Ix::PlaceStop`] data (direction: 0 above, 1 below).
    pub fn place_stop(hint: u32, direction: u8, s: &StopArgs) -> Vec<u8> {
        with(Ix::PlaceStop, direction, 0, 0, hint, bytemuck::bytes_of(s))
    }

    /// [`Ix::TriggerStops`] data.
    pub fn trigger_stops(limit: u32) -> Vec<u8> {
        with(Ix::TriggerStops, 0, 0, 0, limit, &[])
    }

    /// [`Ix::CollectFees`] data.
    pub fn collect_fees() -> Vec<u8> {
        with(Ix::CollectFees, 0, 0, 0, 0, &[])
    }

    /// [`Ix::SetStatus`] data.
    pub fn set_status(status: u8) -> Vec<u8> {
        with(Ix::SetStatus, status, 0, 0, 0, &[])
    }

    /// [`Ix::PruneExpired`] data.
    pub fn prune_expired(limit: u32) -> Vec<u8> {
        with(Ix::PruneExpired, 0, 0, 0, limit, &[])
    }
}
