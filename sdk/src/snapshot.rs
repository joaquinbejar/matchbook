//! Decoded market accounts and exact quotes.

use core::fmt;

use matchbook_program::engine::{
    order_id, BookLevel, DelegateLeaf, EngineError, Market, MarketHeader, NodeTag, NoopSink,
    OrderType, RestingOrder, SelfTradeBehavior, Side, StopLeaf, SwapOutcome, SwapParams,
    TraderLeaf, TriggerDirection, HEADER_SIZE, NIL, NODE_SIZE,
};
use solana_pubkey::Pubkey;

use crate::ix::MarketAccounts;
use crate::{pda, TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID};

/// Match limit the program uses when a swap passes 0.
pub const DEFAULT_SWAP_MATCH_LIMIT: u32 = 64;

/// Why a snapshot could not be read or a quote computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotError {
    /// The data is not a MatchBook market.
    NotAMarket,
    /// The vaults are not associated token accounts of either token program.
    UnknownTokenProgram,
    /// The engine refused the operation, as the program would.
    Engine(EngineError),
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAMarket => f.write_str("not a MatchBook market account"),
            Self::UnknownTokenProgram => f.write_str("vaults belong to no known token program"),
            Self::Engine(e) => write!(f, "engine error: {e:?}"),
        }
    }
}

impl std::error::Error for SnapshotError {}

impl From<EngineError> for SnapshotError {
    fn from(e: EngineError) -> Self {
        Self::Engine(e)
    }
}

type Result<T> = core::result::Result<T, SnapshotError>;

/// A stop waiting for its trigger, as [`MarketSnapshot::seat_stops`] lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeatStop {
    /// Stop id, the one cancels and `oco_with` take.
    pub stop_id: u64,
    /// When it triggers.
    pub direction: TriggerDirection,
    /// Trigger price (quote lots per base lot).
    pub trigger_price: u64,
    /// Side of the order it places.
    pub side: Side,
    /// Limit price of that order.
    pub price: u64,
    /// Size of that order in base lots.
    pub base_lots: u64,
    /// Type of that order.
    pub order_type: OrderType,
    /// Self-trade behavior of that order.
    pub self_trade: SelfTradeBehavior,
    /// Client order id.
    pub client_order_id: u64,
    /// Unix expiry, or 0.
    pub expires_at: u64,
    /// Id of the one-cancels-other partner.
    pub oco_with: Option<u64>,
}

/// A key allowed to act for a seat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeatDelegate {
    /// The delegate's public key.
    pub delegate: Pubkey,
    /// Seat index it acts for.
    pub seat: u32,
    /// Unix time after which it can no longer act, or 0.
    pub expires_at: u64,
    /// [`crate::engine::permission`] bits.
    pub permissions: u64,
}

/// A copy of a market account.
///
/// Reads and quotes run the program's own matching engine on a private
/// copy of the data, so [`MarketSnapshot::quote_swap`] returns exactly what
/// a swap executes against the same account state and clock.
#[derive(Clone)]
pub struct MarketSnapshot {
    key: Pubkey,
    // u64 words keep the node pool aligned for the engine.
    words: Vec<u64>,
    len: usize,
}

impl fmt::Debug for MarketSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MarketSnapshot")
            .field("key", &self.key)
            .field("len", &self.len)
            .finish_non_exhaustive()
    }
}

impl MarketSnapshot {
    /// Decodes the account data of market `key`.
    pub fn new(key: Pubkey, data: &[u8]) -> Result<Self> {
        let mut s = Self {
            key,
            words: Vec::new(),
            len: 0,
        };
        s.update(data)?;
        Ok(s)
    }

    /// Replaces the data with a newer copy of the account.
    pub fn update(&mut self, data: &[u8]) -> Result<()> {
        let mut words = vec![0u64; data.len().div_ceil(8)];
        let bytes = words_as_bytes(&mut words, data.len());
        bytes.copy_from_slice(data);
        Market::load_mut(bytes).map_err(|_| SnapshotError::NotAMarket)?;
        self.words = words;
        self.len = data.len();
        Ok(())
    }

    /// The market's address.
    #[must_use]
    pub fn key(&self) -> Pubkey {
        self.key
    }

    /// The market header.
    #[must_use]
    pub fn header(&self) -> MarketHeader {
        let bytes: &[u8] = bytemuck::cast_slice(&self.words);
        // `update` checked that the data holds a whole header.
        bytes
            .get(..HEADER_SIZE)
            .map(bytemuck::pod_read_unaligned)
            .unwrap_or_else(bytemuck::Zeroable::zeroed)
    }

    /// Base mint.
    #[must_use]
    pub fn base_mint(&self) -> Pubkey {
        Pubkey::new_from_array(self.header().base_mint)
    }

    /// Quote mint.
    #[must_use]
    pub fn quote_mint(&self) -> Pubkey {
        Pubkey::new_from_array(self.header().quote_mint)
    }

    /// The vaults, vault authority and token programs, worked out from the
    /// header alone: each vault is the vault authority's associated token
    /// account under the token program that owns its mint.
    pub fn accounts(&self) -> Result<MarketAccounts> {
        let h = self.header();
        let base_mint = Pubkey::new_from_array(h.base_mint);
        let quote_mint = Pubkey::new_from_array(h.quote_mint);
        let base_tp = token_program_of(&self.key, &base_mint, &h.base_vault)?;
        let quote_tp = token_program_of(&self.key, &quote_mint, &h.quote_vault)?;
        Ok(MarketAccounts::new(
            self.key, base_mint, quote_mint, base_tp, quote_tp,
        ))
    }

    /// Runs `f` on a private copy of the market.
    fn with_copy<R>(&self, f: impl FnOnce(&mut Market<'_>) -> Result<R>) -> Result<R> {
        let mut words = self.words.clone();
        let bytes = words_as_bytes(&mut words, self.len);
        let mut m = Market::load_mut(bytes).map_err(|_| SnapshotError::NotAMarket)?;
        f(&mut m)
    }

    /// Up to `depth` aggregated levels on `side`, best first, skipping
    /// orders expired at `now` (unix seconds).
    pub fn levels(&self, side: Side, now: u64, depth: usize) -> Result<Vec<BookLevel>> {
        self.with_copy(|m| {
            let mut out = Vec::new();
            if depth > 0 {
                m.for_each_level(side, now, |l| {
                    out.push(l);
                    out.len() < depth
                })?;
            }
            Ok(out)
        })
    }

    /// Live orders on `side` in priority order.
    pub fn orders(&self, side: Side, now: u64) -> Result<Vec<RestingOrder>> {
        self.with_copy(|m| {
            let mut out = Vec::new();
            m.for_each_order(side, now, |o| {
                out.push(o);
                true
            })?;
            Ok(out)
        })
    }

    /// The seat of `owner`: its index (the hint trader instructions take)
    /// and its balances.
    pub fn seat(&self, owner: &Pubkey) -> Result<Option<(u32, TraderLeaf)>> {
        self.with_copy(|m| {
            let Some(i) = m.find_trader(&owner.to_bytes())? else {
                return Ok(None);
            };
            Ok(Some((i, *m.trader(i)?)))
        })
    }

    /// The resting orders of seat `seat`, newest first.
    pub fn seat_orders(&self, seat: u32) -> Result<Vec<RestingOrder>> {
        self.with_copy(|m| {
            let mut out = Vec::new();
            m.for_each_trader_order(seat, |_, o| out.push(o))?;
            Ok(out)
        })
    }

    /// The stops of seat `seat`, newest first.
    pub fn seat_stops(&self, seat: u32) -> Result<Vec<SeatStop>> {
        self.with_copy(|m| {
            let mut raw: Vec<(u32, StopLeaf)> = Vec::new();
            m.for_each_trader_stop(seat, |i, s| raw.push((i, *s)))?;
            let mut out = Vec::with_capacity(raw.len());
            for (i, s) in &raw {
                let partner = s.partner();
                let oco_with = if partner == NIL {
                    None
                } else {
                    Some(order_id(m.stop(partner)?.seq(), partner))
                };
                out.push(SeatStop {
                    stop_id: order_id(s.seq(), *i),
                    direction: if s.is_above() {
                        TriggerDirection::Above
                    } else {
                        TriggerDirection::Below
                    },
                    trigger_price: s.trigger_price(),
                    side: if s.is_ask() { Side::Ask } else { Side::Bid },
                    price: s.price,
                    base_lots: s.base_lots,
                    order_type: OrderType::from_u8(s.order_type).unwrap_or_default(),
                    self_trade: SelfTradeBehavior::from_u8(s.self_trade).unwrap_or_default(),
                    client_order_id: s.client_order_id,
                    expires_at: s.expires_at,
                    oco_with,
                });
            }
            Ok(out)
        })
    }

    /// Base and quote atoms seat `seat` has locked in orders and stops.
    pub fn locked(&self, seat: u32) -> Result<(u64, u64)> {
        self.with_copy(|m| Ok(m.locked(seat)?))
    }

    /// Every delegate of the market.
    #[must_use]
    pub fn delegates(&self) -> Vec<SeatDelegate> {
        let bytes: &[u8] = bytemuck::cast_slice(&self.words);
        let bytes = bytes.get(..self.len).unwrap_or_default();
        let used = self.header().next_unused as usize;
        bytes
            .get(HEADER_SIZE..)
            .unwrap_or_default()
            .as_chunks::<NODE_SIZE>()
            .0
            .iter()
            .take(used)
            .filter_map(|n| {
                let d: DelegateLeaf = bytemuck::pod_read_unaligned(n);
                (d.tag == NodeTag::Delegate as u32).then(|| SeatDelegate {
                    delegate: Pubkey::new_from_array(d.delegate),
                    seat: d.trader,
                    expires_at: d.expires_at,
                    permissions: d.permissions,
                })
            })
            .collect()
    }

    /// What a swap with these parameters executes at time `now` (unix
    /// seconds). `match_limit` 0 means the program's default.
    pub fn quote_swap(&self, p: &SwapParams, now: u64) -> Result<SwapOutcome> {
        let mut p = *p;
        if p.match_limit == 0 {
            p.match_limit = DEFAULT_SWAP_MATCH_LIMIT;
        }
        self.with_copy(|m| Ok(m.swap(&p, now, &mut NoopSink)?))
    }

    /// Quotes spending exactly up to `amount_in` atoms: base atoms to sell
    /// (ask) or quote atoms to buy with, fees included (bid). The outcome
    /// says what was used: `base_atoms` for an ask and
    /// [`SwapOutcome::quote_net`] for a bid.
    pub fn quote_exact_in(&self, side: Side, amount_in: u64, now: u64) -> Result<SwapOutcome> {
        self.quote_swap(&self.exact_in_params(side, amount_in), now)
    }

    /// The swap parameters [`MarketSnapshot::quote_exact_in`] uses, ready
    /// for [`crate::SwapArgs`].
    #[must_use]
    pub fn exact_in_params(&self, side: Side, amount_in: u64) -> SwapParams {
        let h = self.header();
        match side {
            Side::Ask => SwapParams {
                side,
                base_lots: amount_in / h.base_lot_size.max(1),
                quote_budget: 0,
                limit_price: 1,
                match_limit: DEFAULT_SWAP_MATCH_LIMIT,
            },
            Side::Bid => SwapParams {
                side,
                base_lots: u64::MAX / h.base_lot_size.max(1),
                quote_budget: amount_in,
                limit_price: u64::MAX,
                match_limit: DEFAULT_SWAP_MATCH_LIMIT,
            },
        }
    }
}

fn token_program_of(market: &Pubkey, mint: &Pubkey, vault: &[u8; 32]) -> Result<Pubkey> {
    [TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID]
        .into_iter()
        .find(|tp| pda::vault(market, mint, tp).to_bytes() == *vault)
        .ok_or(SnapshotError::UnknownTokenProgram)
}

/// The first `len` bytes of `words`, which hold the account data.
fn words_as_bytes(words: &mut [u64], len: usize) -> &mut [u8] {
    let bytes: &mut [u8] = bytemuck::cast_slice_mut(words);
    let len = len.min(bytes.len());
    let (head, _) = bytes.split_at_mut(len);
    head
}
