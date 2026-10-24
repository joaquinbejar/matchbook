//! Instruction discriminators, account orders and data layouts.
//!
//! All integers are little-endian. `hint` is the trader's seat index (any
//! value, e.g. `u32::MAX`, falls back to a lookup).

use matchbook_engine::{OrderParams, OrderType, SelfTradeBehavior, Side};

use crate::error::MatchbookError;

/// Instruction discriminators (first byte of instruction data).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Ix {
    /// Accounts: payer (s,w), market (s,w), base_mint, quote_mint,
    /// base_vault (w), quote_vault (w), vault_authority, base_token_program,
    /// quote_token_program, associated_token_program, system_program.
    /// Data: [`CreateMarketArgs`]. The market account is either a fresh
    /// keypair (created here with `initial_nodes`, at most 10 KiB) or an
    /// account the client already created, owned by this program and zeroed.
    CreateMarket = 0,
    /// Accounts: payer (s,w), owner (s), market (w), system_program.
    ClaimSeat = 1,
    /// Accounts: owner (s), market (w), user_token (w), vault (w), mint, token_program.
    /// Data: hint u32, amount u64.
    Deposit = 2,
    /// Accounts: owner (s), market (w), user_token (w), vault (w), mint,
    /// vault_authority, token_program. Data: hint u32, amount u64.
    Withdraw = 3,
    /// Accounts: owner (s,w), market (w), system_program.
    /// Data: hint u32, [`OrderArgs`].
    PlaceOrder = 4,
    /// Accounts: owner (s), market (w). Data: hint u32, order_id u64.
    CancelOrder = 5,
    /// Accounts: owner (s), market (w). Data: hint u32, side u8 (0 bid, 1 ask, 2 both).
    CancelAll = 6,
    /// Accounts: owner (s), market (w). Data: hint u32, client_order_id u64.
    CancelByClientId = 7,
    /// Accounts: owner (s), market (w). Data: hint u32, order_id u64, new_base_lots u64.
    ReduceOrder = 8,
    /// Accounts: owner (s,w), market (w), system_program.
    /// Data: hint u32, cancel_all u8 (0 none, 1 bids, 2 asks, 3 both),
    /// n_cancels u8, n_orders u8, n_cancels × order_id u64, n_orders × [`OrderArgs`].
    BatchUpdate = 9,
    /// Accounts: user (s), market (w), user_base (w), user_quote (w),
    /// base_vault (w), quote_vault (w), base_mint, quote_mint, vault_authority,
    /// base_token_program, quote_token_program. Data: [`SwapArgs`].
    Swap = 10,
    /// Accounts: authority (s), market (w), destination (w), quote_vault (w),
    /// quote_mint, vault_authority, token_program.
    CollectFees = 11,
    /// Accounts: authority (s), market (w). Data: status u8.
    SetStatus = 12,
    /// Accounts: market (w). Data: limit u32. Permissionless.
    PruneExpired = 13,
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
            _ => return None,
        })
    }
}

/// Little-endian cursor over instruction data.
pub struct Reader<'a> {
    data: &'a [u8],
}

impl<'a> Reader<'a> {
    /// Wraps `data`.
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    fn take<const N: usize>(&mut self) -> Result<[u8; N], MatchbookError> {
        let (head, rest) = self
            .data
            .split_first_chunk::<N>()
            .ok_or(MatchbookError::InvalidInstructionData)?;
        self.data = rest;
        Ok(*head)
    }

    /// Reads a u8.
    pub fn u8(&mut self) -> Result<u8, MatchbookError> {
        Ok(self.take::<1>()?[0])
    }

    /// Reads a u32.
    pub fn u32(&mut self) -> Result<u32, MatchbookError> {
        Ok(u32::from_le_bytes(self.take()?))
    }

    /// Reads a u64.
    pub fn u64(&mut self) -> Result<u64, MatchbookError> {
        Ok(u64::from_le_bytes(self.take()?))
    }

    /// Reads 32 bytes.
    pub fn bytes32(&mut self) -> Result<[u8; 32], MatchbookError> {
        self.take()
    }
}

/// Arguments of [`Ix::CreateMarket`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// Fee collector and status authority.
    pub authority: [u8; 32],
}

impl CreateMarketArgs {
    /// Encoded size.
    pub const LEN: usize = 8 * 3 + 4 * 3 + 32;

    /// Decodes.
    pub fn read(r: &mut Reader<'_>) -> Result<Self, MatchbookError> {
        Ok(Self {
            base_lot_size: r.u64()?,
            quote_lot_size: r.u64()?,
            min_base_lots: r.u64()?,
            taker_fee_bps: r.u32()?,
            maker_rebate_bps: r.u32()?,
            initial_nodes: r.u32()?,
            authority: r.bytes32()?,
        })
    }
}

/// Order arguments shared by [`Ix::PlaceOrder`] and [`Ix::BatchUpdate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderArgs {
    /// 0 bid, 1 ask.
    pub side: u8,
    /// [`OrderType`] wire value.
    pub order_type: u8,
    /// [`SelfTradeBehavior`] wire value.
    pub self_trade: u8,
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
    /// Max resting orders to touch (0 = default).
    pub match_limit: u32,
}

impl OrderArgs {
    /// Encoded size.
    pub const LEN: usize = 4 + 8 * 5 + 4;

    /// Decodes.
    pub fn read(r: &mut Reader<'_>) -> Result<Self, MatchbookError> {
        let side = r.u8()?;
        let order_type = r.u8()?;
        let self_trade = r.u8()?;
        let _pad = r.u8()?;
        Ok(Self {
            side,
            order_type,
            self_trade,
            price: r.u64()?,
            base_lots: r.u64()?,
            client_order_id: r.u64()?,
            expires_at: r.u64()?,
            quote_budget: r.u64()?,
            match_limit: r.u32()?,
        })
    }

    /// Converts to engine parameters.
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
        })
    }
}

/// Arguments of [`Ix::Swap`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapArgs {
    /// 0 buy base, 1 sell base.
    pub side: u8,
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
}

impl SwapArgs {
    /// Encoded size.
    pub const LEN: usize = 1 + 8 * 4 + 4;

    /// Decodes.
    pub fn read(r: &mut Reader<'_>) -> Result<Self, MatchbookError> {
        Ok(Self {
            side: r.u8()?,
            base_lots: r.u64()?,
            quote_budget: r.u64()?,
            limit_price: r.u64()?,
            min_out: r.u64()?,
            match_limit: r.u32()?,
        })
    }
}

/// Instruction data builders for clients (requires the `client` feature).
#[cfg(feature = "client")]
pub mod build {
    use super::*;
    use std::vec::Vec;

    fn start(ix: Ix) -> Vec<u8> {
        std::vec![ix as u8]
    }

    /// [`Ix::CreateMarket`] data.
    pub fn create_market(a: &CreateMarketArgs) -> Vec<u8> {
        let mut d = start(Ix::CreateMarket);
        d.extend_from_slice(&a.base_lot_size.to_le_bytes());
        d.extend_from_slice(&a.quote_lot_size.to_le_bytes());
        d.extend_from_slice(&a.min_base_lots.to_le_bytes());
        d.extend_from_slice(&a.taker_fee_bps.to_le_bytes());
        d.extend_from_slice(&a.maker_rebate_bps.to_le_bytes());
        d.extend_from_slice(&a.initial_nodes.to_le_bytes());
        d.extend_from_slice(&a.authority);
        d
    }

    /// [`Ix::ClaimSeat`] data.
    pub fn claim_seat() -> Vec<u8> {
        start(Ix::ClaimSeat)
    }

    fn hint_amount(ix: Ix, hint: u32, amount: u64) -> Vec<u8> {
        let mut d = start(ix);
        d.extend_from_slice(&hint.to_le_bytes());
        d.extend_from_slice(&amount.to_le_bytes());
        d
    }

    /// [`Ix::Deposit`] data.
    pub fn deposit(hint: u32, amount: u64) -> Vec<u8> {
        hint_amount(Ix::Deposit, hint, amount)
    }

    /// [`Ix::Withdraw`] data.
    pub fn withdraw(hint: u32, amount: u64) -> Vec<u8> {
        hint_amount(Ix::Withdraw, hint, amount)
    }

    /// Appends encoded [`OrderArgs`].
    pub fn push_order(d: &mut Vec<u8>, o: &OrderArgs) {
        d.extend_from_slice(&[o.side, o.order_type, o.self_trade, 0]);
        for v in [o.price, o.base_lots, o.client_order_id, o.expires_at, o.quote_budget] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        d.extend_from_slice(&o.match_limit.to_le_bytes());
    }

    /// [`Ix::PlaceOrder`] data.
    pub fn place_order(hint: u32, o: &OrderArgs) -> Vec<u8> {
        let mut d = start(Ix::PlaceOrder);
        d.extend_from_slice(&hint.to_le_bytes());
        push_order(&mut d, o);
        d
    }

    /// [`Ix::CancelOrder`] data.
    pub fn cancel_order(hint: u32, order_id: u64) -> Vec<u8> {
        hint_amount(Ix::CancelOrder, hint, order_id)
    }

    /// [`Ix::CancelAll`] data (side: 0 bid, 1 ask, 2 both).
    pub fn cancel_all(hint: u32, side: u8) -> Vec<u8> {
        let mut d = start(Ix::CancelAll);
        d.extend_from_slice(&hint.to_le_bytes());
        d.push(side);
        d
    }

    /// [`Ix::CancelByClientId`] data.
    pub fn cancel_by_client_id(hint: u32, client_order_id: u64) -> Vec<u8> {
        hint_amount(Ix::CancelByClientId, hint, client_order_id)
    }

    /// [`Ix::ReduceOrder`] data.
    pub fn reduce_order(hint: u32, order_id: u64, new_base_lots: u64) -> Vec<u8> {
        let mut d = hint_amount(Ix::ReduceOrder, hint, order_id);
        d.extend_from_slice(&new_base_lots.to_le_bytes());
        d
    }

    /// [`Ix::BatchUpdate`] data.
    pub fn batch_update(hint: u32, cancel_all: u8, cancels: &[u64], orders: &[OrderArgs]) -> Vec<u8> {
        let mut d = start(Ix::BatchUpdate);
        d.extend_from_slice(&hint.to_le_bytes());
        d.push(cancel_all);
        d.push(cancels.len() as u8);
        d.push(orders.len() as u8);
        for c in cancels {
            d.extend_from_slice(&c.to_le_bytes());
        }
        for o in orders {
            push_order(&mut d, o);
        }
        d
    }

    /// [`Ix::Swap`] data.
    pub fn swap(a: &SwapArgs) -> Vec<u8> {
        let mut d = start(Ix::Swap);
        d.push(a.side);
        for v in [a.base_lots, a.quote_budget, a.limit_price, a.min_out] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        d.extend_from_slice(&a.match_limit.to_le_bytes());
        d
    }

    /// [`Ix::CollectFees`] data.
    pub fn collect_fees() -> Vec<u8> {
        start(Ix::CollectFees)
    }

    /// [`Ix::SetStatus`] data.
    pub fn set_status(status: u8) -> Vec<u8> {
        std::vec![Ix::SetStatus as u8, status]
    }

    /// [`Ix::PruneExpired`] data.
    pub fn prune_expired(limit: u32) -> Vec<u8> {
        let mut d = start(Ix::PruneExpired);
        d.extend_from_slice(&limit.to_le_bytes());
        d
    }
}
