//! Public parameter and result types.

/// Order side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Side {
    /// Buy base with quote.
    Bid = 0,
    /// Sell base for quote.
    Ask = 1,
}

impl Side {
    /// The other side.
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::Bid => Self::Ask,
            Self::Ask => Self::Bid,
        }
    }

    /// Decodes a wire value.
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Bid),
            1 => Some(Self::Ask),
            _ => None,
        }
    }
}

/// How an order interacts with the book.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum OrderType {
    /// Match what crosses, rest the remainder.
    #[default]
    Limit = 0,
    /// Rest only; rejected if it would cross.
    PostOnly = 1,
    /// Rest only; if it would cross, reprice one tick behind the best opposite order.
    PostOnlySlide = 2,
    /// Match what crosses, drop the remainder.
    ImmediateOrCancel = 3,
    /// Fill completely or fail without side effects.
    FillOrKill = 4,
}

impl OrderType {
    /// Decodes a wire value.
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Limit),
            1 => Some(Self::PostOnly),
            2 => Some(Self::PostOnlySlide),
            3 => Some(Self::ImmediateOrCancel),
            4 => Some(Self::FillOrKill),
            _ => None,
        }
    }

    pub(crate) const fn takes(self) -> bool {
        !matches!(self, Self::PostOnly | Self::PostOnlySlide)
    }

    pub(crate) const fn rests(self) -> bool {
        matches!(self, Self::Limit | Self::PostOnly | Self::PostOnlySlide)
    }
}

/// What happens when an order would match another order of the same trader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum SelfTradeBehavior {
    /// Cancel the resting order and keep matching.
    #[default]
    CancelProvide = 0,
    /// Reduce both orders by the overlap without trading.
    DecrementTake = 1,
    /// Fail the whole instruction.
    Abort = 2,
}

impl SelfTradeBehavior {
    /// Decodes a wire value.
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::CancelProvide),
            1 => Some(Self::DecrementTake),
            2 => Some(Self::Abort),
            _ => None,
        }
    }
}

/// Parameters of an order placed by a seated trader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderParams {
    /// Side.
    pub side: Side,
    /// Limit price in quote lots per base lot.
    pub price: u64,
    /// Quantity in base lots.
    pub base_lots: u64,
    /// Order type.
    pub order_type: OrderType,
    /// Self-trade behavior.
    pub self_trade: SelfTradeBehavior,
    /// Client id, echoed in events and usable for cancels.
    pub client_order_id: u64,
    /// Unix timestamp at which a resting order dies (0 = never).
    pub expires_at: u64,
    /// For immediate-or-cancel bids: maximum quote atoms to spend, fees included.
    pub quote_budget: Option<u64>,
    /// Maximum resting orders to touch while matching (bounds compute).
    pub match_limit: u32,
    /// Makes the resting part a reverse order: each fill re-posts the filled
    /// size on the other side, this many parts per million away from the
    /// fill price (0 = an ordinary order). See [`REVERSE_SPREAD_SCALE`].
    pub reverse_spread: u32,
}

/// Parts per million: a reverse spread of 1,000 is 0.1%.
pub const REVERSE_SPREAD_SCALE: u32 = 1_000_000;

impl OrderParams {
    /// Default match limit.
    pub const DEFAULT_MATCH_LIMIT: u32 = 64;

    /// A good-till-cancel limit order.
    #[must_use]
    pub const fn limit(side: Side, price: u64, base_lots: u64) -> Self {
        Self {
            side,
            price,
            base_lots,
            order_type: OrderType::Limit,
            self_trade: SelfTradeBehavior::CancelProvide,
            client_order_id: 0,
            expires_at: 0,
            quote_budget: None,
            match_limit: Self::DEFAULT_MATCH_LIMIT,
            reverse_spread: 0,
        }
    }

    /// Sets the order type.
    #[must_use]
    pub const fn with_type(mut self, t: OrderType) -> Self {
        self.order_type = t;
        self
    }

    /// Sets the client order id.
    #[must_use]
    pub const fn with_client_id(mut self, id: u64) -> Self {
        self.client_order_id = id;
        self
    }

    /// Sets the expiry timestamp.
    #[must_use]
    pub const fn with_expiry(mut self, ts: u64) -> Self {
        self.expires_at = ts;
        self
    }

    /// Sets the self-trade behavior.
    #[must_use]
    pub const fn with_self_trade(mut self, b: SelfTradeBehavior) -> Self {
        self.self_trade = b;
        self
    }

    /// Sets a quote budget (immediate-or-cancel bids only).
    #[must_use]
    pub const fn with_budget(mut self, atoms: u64) -> Self {
        self.quote_budget = Some(atoms);
        self
    }

    /// Sets the match limit.
    #[must_use]
    pub const fn with_match_limit(mut self, n: u32) -> Self {
        self.match_limit = n;
        self
    }

    /// Makes the order a reverse order with `spread` parts per million.
    #[must_use]
    pub const fn with_reverse(mut self, spread: u32) -> Self {
        self.reverse_spread = spread;
        self
    }
}

/// When a stop triggers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TriggerDirection {
    /// When a trade happens at or above the trigger price (stop buys,
    /// take-profit sells).
    Above = 0,
    /// When a trade happens at or below the trigger price (stop-loss sells,
    /// take-profit buys).
    Below = 1,
}

impl TriggerDirection {
    /// Decodes a wire value.
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Above),
            1 => Some(Self::Below),
            _ => None,
        }
    }
}

/// A stop: an order placed for its seat once the last trade price reaches
/// `trigger_price`. Its funds are locked from the start, so it cannot fail
/// for lack of them when it triggers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StopParams {
    /// Price that triggers the stop.
    pub trigger_price: u64,
    /// Which side of `trigger_price` the last trade must reach.
    pub direction: TriggerDirection,
    /// The order to place. Its expiry also applies to the stop. Its match
    /// limit is capped at [`MAX_STOP_MATCH_LIMIT`].
    pub order: OrderParams,
    /// Id of another of the seat's stops to pair with: when either triggers,
    /// the other is cancelled (a bracket of take-profit and stop-loss).
    /// 0 for none.
    pub oco_with: u64,
}

/// Highest match limit of an order placed by a stop, which bounds the
/// compute a trigger adds to whoever's trade sets it off.
pub const MAX_STOP_MATCH_LIMIT: u32 = 16;

/// Stops triggered automatically at the end of each trade, at most. Anyone
/// can trigger more with [`crate::Market::trigger_stops`].
pub const AUTO_TRIGGER_LIMIT: u32 = 2;

/// Result of [`crate::Market::place_order`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlaceOutcome {
    /// Id of the resting order, if any part rested.
    pub order_id: Option<u64>,
    /// Price the remainder rested at (differs from the limit for slid post-only orders).
    pub rested_price: u64,
    /// Base lots rested.
    pub rested_base_lots: u64,
    /// Base lots filled as taker.
    pub filled_base_lots: u64,
    /// Quote atoms exchanged as taker, before fees.
    pub filled_quote_atoms: u64,
    /// Taker fee paid in quote atoms.
    pub fee_atoms: u64,
    /// Resting orders touched (filled, expired or self-trade cancelled).
    pub makers_touched: u32,
}

/// A taker-only trade with no seat, e.g. from an aggregator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapParams {
    /// Ask: sell base. Bid: buy base.
    pub side: Side,
    /// Ask: base lots to sell. Bid: maximum base lots to buy.
    pub base_lots: u64,
    /// Bid: maximum quote atoms to spend, fees included. Ignored for asks.
    pub quote_budget: u64,
    /// Worst acceptable price in quote lots per base lot.
    pub limit_price: u64,
    /// Maximum resting orders to touch.
    pub match_limit: u32,
}

/// Result of [`crate::Market::swap`]: what the taker must pay and receive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SwapOutcome {
    /// Base lots traded.
    pub base_lots: u64,
    /// Base atoms traded.
    pub base_atoms: u64,
    /// Quote atoms traded before fees.
    pub quote_atoms: u64,
    /// Taker fee in quote atoms.
    pub fee_atoms: u64,
    /// Resting orders touched.
    pub makers_touched: u32,
}

impl SwapOutcome {
    /// Quote atoms the taker pays (bid) or receives (ask), fees applied.
    #[must_use]
    pub fn quote_net(&self, side: Side) -> u64 {
        match side {
            Side::Bid => self.quote_atoms.saturating_add(self.fee_atoms),
            Side::Ask => self.quote_atoms.saturating_sub(self.fee_atoms),
        }
    }
}

/// One aggregated price level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BookLevel {
    /// Price in quote lots per base lot.
    pub price: u64,
    /// Total base lots.
    pub base_lots: u64,
    /// Number of orders.
    pub orders: u32,
}

/// A resting order as seen by readers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestingOrder {
    /// Order id (sequence number).
    pub order_id: u64,
    /// Side.
    pub side: Side,
    /// Price in quote lots per base lot.
    pub price: u64,
    /// Remaining base lots.
    pub base_lots: u64,
    /// Client order id.
    pub client_order_id: u64,
    /// Expiry (0 = never).
    pub expires_at: u64,
    /// Owner.
    pub owner: [u8; 32],
}

/// Permission bits a seat owner grants a delegate.
pub mod permission {
    /// Place, reduce and cancel orders, and batch updates.
    pub const TRADE: u64 = 1;
    /// Cancel and reduce orders only.
    pub const CANCEL: u64 = 1 << 1;
    /// Every defined bit.
    pub const ALL: u64 = TRADE | CANCEL;
}

/// Who is acting for a seat: its owner, or one of its delegates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Actor {
    /// The seat acted for.
    pub trader: u32,
    /// Whether the signer is a delegate rather than the owner.
    pub delegate: bool,
    /// The delegate's [`permission`] bits (all of them for the owner).
    pub permissions: u64,
    /// When the delegate's session ends, or 0.
    pub expires_at: u64,
}

impl Actor {
    /// The owner of seat `trader`.
    #[must_use]
    pub const fn owner(trader: u32) -> Self {
        Self {
            trader,
            delegate: false,
            permissions: permission::ALL,
            expires_at: 0,
        }
    }

    /// Fails unless the actor holds one of the `any_of` permissions and, for
    /// a delegate, its session is still open at unix time `now`.
    pub fn require(&self, any_of: u64, now: u64) -> crate::Result<u32> {
        if self.delegate {
            if self.permissions & any_of == 0 {
                return Err(crate::EngineError::NotPermitted);
            }
            if self.expires_at != 0 && now >= self.expires_at {
                return Err(crate::EngineError::DelegateExpired);
            }
        }
        Ok(self.trader)
    }

    /// Fails unless the actor is the seat's owner.
    pub fn require_owner(&self) -> crate::Result<u32> {
        if self.delegate {
            return Err(crate::EngineError::NotPermitted);
        }
        Ok(self.trader)
    }
}
