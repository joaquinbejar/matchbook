//! Decoded market accounts and exact quotes.

use core::fmt;

use matchbook_program::engine::{
    BookLevel, EngineError, Market, MarketHeader, NoopSink, RestingOrder, Side, SwapOutcome,
    SwapParams, TraderLeaf, HEADER_SIZE,
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
