//! Engine errors.

use core::fmt;

/// Every way an engine operation can fail.
///
/// Operations validate up front, so an error never leaves the market
/// half-modified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum EngineError {
    /// Buffer is too small, misaligned, or not a market.
    InvalidAccountData = 0,
    /// Market was already initialized.
    AlreadyInitialized,
    /// Invalid market parameters.
    InvalidMarketParams,
    /// Not enough free nodes; grow the buffer and retry.
    OutOfSpace,
    /// Trader index or owner not found.
    TraderNotFound,
    /// Owner already has a seat on this market.
    SeatAlreadyClaimed,
    /// Seat still has open orders or balances.
    SeatNotEmpty,
    /// Price must be greater than zero.
    InvalidPrice,
    /// Quantity is zero or below the market minimum.
    InvalidQuantity,
    /// Free balance is too low for this operation.
    InsufficientFunds,
    /// Post-only order would take liquidity.
    PostOnlyWouldCross,
    /// Fill-or-kill order cannot be fully filled.
    FillOrKillNotFilled,
    /// Order would match against the same trader and the behavior is abort.
    SelfTrade,
    /// Order not found for this trader.
    OrderNotFound,
    /// Arithmetic overflow.
    Overflow,
    /// Market status does not allow this operation.
    MarketNotActive,
    /// A quote budget is only allowed on immediate-or-cancel orders.
    BudgetRequiresIoc,
    /// Order already expired when placed.
    AlreadyExpired,
    /// Internal invariant violated (bug).
    Corrupted,
    /// No such delegate for this seat.
    DelegateNotFound,
    /// The seat already has the maximum number of delegates.
    TooManyDelegates,
    /// The signer may not do this for the seat.
    NotPermitted,
    /// The delegate's session has ended.
    DelegateExpired,
    /// A delegate cannot be the seat's owner, and needs known permissions.
    InvalidDelegate,
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}
