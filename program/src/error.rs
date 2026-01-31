//! Error types for the Matchbook program.
//!
//! This module defines all error types using Anchor's error handling system.
//! Errors are grouped by category and provide clear feedback for failed
//! transactions.
//!
//! # Error Code Ranges
//!
//! - 6000-6009: Market errors
//! - 6010-6029: Order validation errors
//! - 6030-6039: Balance/fund errors
//! - 6040-6049: Order state errors
//! - 6050-6059: Matching errors
//! - 6060-6069: Arithmetic errors
//! - 6070-6079: Event queue errors
//! - 6080-6089: Access control errors

use anchor_lang::prelude::*;

/// Error category for grouping related errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Market-related errors.
    Market,
    /// Order validation errors.
    OrderValidation,
    /// Balance and fund errors.
    Balance,
    /// Order state errors.
    OrderState,
    /// Order matching errors.
    Matching,
    /// Arithmetic computation errors.
    Arithmetic,
    /// Event queue errors.
    Queue,
    /// Access control errors.
    Access,
}

/// Matchbook program errors.
///
/// All errors start at code 6000 following Anchor convention.
/// Each error includes a lowercase message describing the condition.
#[error_code]
pub enum MatchbookError {
    // ========================================================================
    // Market errors (6000-6009)
    // ========================================================================
    /// Market is not in active state.
    #[msg("market is not active")]
    MarketNotActive,

    /// Market is in cancel-only mode (paused).
    #[msg("market is in cancel-only mode")]
    MarketCancelOnly,

    /// Market state is invalid for the requested operation.
    #[msg("invalid market state")]
    InvalidMarketState,

    /// Market has been permanently closed.
    #[msg("market is closed")]
    MarketClosed,

    // ========================================================================
    // Order validation errors (6010-6029)
    // ========================================================================
    /// Price is not a multiple of the market's tick size.
    #[msg("price is not a multiple of tick size")]
    InvalidTickSize,

    /// Quantity is not a multiple of the market's lot size.
    #[msg("quantity is not a multiple of lot size")]
    InvalidLotSize,

    /// Order size is below the market's minimum order size.
    #[msg("order size below minimum")]
    OrderTooSmall,

    /// Price is zero or exceeds the maximum allowed price.
    #[msg("price is zero or exceeds maximum")]
    InvalidPrice,

    /// Quantity is zero or exceeds the maximum allowed quantity.
    #[msg("invalid quantity")]
    InvalidQuantity,

    /// Order side is invalid.
    #[msg("invalid order side")]
    InvalidSide,

    /// Order type is invalid or not supported.
    #[msg("invalid order type")]
    InvalidOrderType,

    /// Time-in-force value is invalid.
    #[msg("invalid time in force")]
    InvalidTimeInForce,

    /// Client order ID is invalid or already in use.
    #[msg("invalid client order id")]
    InvalidClientOrderId,

    // ========================================================================
    // Balance/fund errors (6030-6039)
    // ========================================================================
    /// User does not have sufficient funds deposited.
    #[msg("insufficient funds deposited")]
    InsufficientFunds,

    /// User does not have sufficient free balance for withdrawal.
    #[msg("insufficient free balance for withdrawal")]
    InsufficientFreeBalance,

    /// Operation would cause balance overflow.
    #[msg("balance overflow")]
    BalanceOverflow,

    /// Locked balance is insufficient for the operation.
    #[msg("insufficient locked balance")]
    InsufficientLockedBalance,

    // ========================================================================
    // Order state errors (6040-6049)
    // ========================================================================
    /// Order was not found in the order book.
    #[msg("order not found in book")]
    OrderNotFound,

    /// Caller is not authorized to modify this order.
    #[msg("not authorized to modify this order")]
    NotOrderOwner,

    /// User has too many open orders.
    #[msg("too many open orders")]
    TooManyOrders,

    /// No free order slot available in OpenOrders account.
    #[msg("no free order slot available")]
    NoFreeOrderSlot,

    /// Order has already been filled or cancelled.
    #[msg("order already processed")]
    OrderAlreadyProcessed,

    // ========================================================================
    // Matching errors (6050-6059)
    // ========================================================================
    /// Post-only order would immediately match (cross the spread).
    #[msg("post-only order would cross the spread")]
    PostOnlyWouldCross,

    /// Fill-or-kill order cannot be completely filled.
    #[msg("fill-or-kill order cannot be fully filled")]
    FillOrKillNotFillable,

    /// Order would result in a self-trade.
    #[msg("self-trade would occur")]
    SelfTradeError,

    /// No matching orders available.
    #[msg("no matching orders")]
    NoMatchingOrders,

    // ========================================================================
    // Arithmetic errors (6060-6069)
    // ========================================================================
    /// Arithmetic operation resulted in overflow.
    #[msg("arithmetic overflow")]
    ArithmeticOverflow,

    /// Arithmetic operation resulted in underflow.
    #[msg("arithmetic underflow")]
    ArithmeticUnderflow,

    /// Division by zero attempted.
    #[msg("division by zero")]
    DivisionByZero,

    /// Numeric conversion failed.
    #[msg("numeric conversion failed")]
    ConversionFailed,

    // ========================================================================
    // Event queue errors (6070-6079)
    // ========================================================================
    /// Event queue is full and cannot accept more events.
    #[msg("event queue is full")]
    EventQueueFull,

    /// Event queue is empty.
    #[msg("event queue is empty")]
    EventQueueEmpty,

    /// Invalid event in queue.
    #[msg("invalid event")]
    InvalidEvent,

    // ========================================================================
    // Access control errors (6080-6089)
    // ========================================================================
    /// Caller is not authorized for this operation.
    #[msg("unauthorized")]
    Unauthorized,

    /// Invalid authority provided.
    #[msg("invalid authority")]
    InvalidAuthority,

    /// Invalid delegate provided.
    #[msg("invalid delegate")]
    InvalidDelegate,

    /// Signer is required but not present.
    #[msg("missing required signer")]
    MissingRequiredSigner,

    // ========================================================================
    // Account errors (6090-6099)
    // ========================================================================
    /// Account data is invalid or corrupted.
    #[msg("invalid account data")]
    InvalidAccountData,

    /// Account is not initialized.
    #[msg("account not initialized")]
    AccountNotInitialized,

    /// Account is already initialized.
    #[msg("account already initialized")]
    AccountAlreadyInitialized,

    /// Invalid PDA derivation.
    #[msg("invalid pda")]
    InvalidPda,
}

impl MatchbookError {
    /// Returns the error category for this error.
    #[must_use]
    pub const fn category(&self) -> ErrorCategory {
        match self {
            Self::MarketNotActive
            | Self::MarketCancelOnly
            | Self::InvalidMarketState
            | Self::MarketClosed => ErrorCategory::Market,

            Self::InvalidTickSize
            | Self::InvalidLotSize
            | Self::OrderTooSmall
            | Self::InvalidPrice
            | Self::InvalidQuantity
            | Self::InvalidSide
            | Self::InvalidOrderType
            | Self::InvalidTimeInForce
            | Self::InvalidClientOrderId => ErrorCategory::OrderValidation,

            Self::InsufficientFunds
            | Self::InsufficientFreeBalance
            | Self::BalanceOverflow
            | Self::InsufficientLockedBalance => ErrorCategory::Balance,

            Self::OrderNotFound
            | Self::NotOrderOwner
            | Self::TooManyOrders
            | Self::NoFreeOrderSlot
            | Self::OrderAlreadyProcessed => ErrorCategory::OrderState,

            Self::PostOnlyWouldCross
            | Self::FillOrKillNotFillable
            | Self::SelfTradeError
            | Self::NoMatchingOrders => ErrorCategory::Matching,

            Self::ArithmeticOverflow
            | Self::ArithmeticUnderflow
            | Self::DivisionByZero
            | Self::ConversionFailed => ErrorCategory::Arithmetic,

            Self::EventQueueFull | Self::EventQueueEmpty | Self::InvalidEvent => {
                ErrorCategory::Queue
            }

            Self::Unauthorized
            | Self::InvalidAuthority
            | Self::InvalidDelegate
            | Self::MissingRequiredSigner => ErrorCategory::Access,

            Self::InvalidAccountData
            | Self::AccountNotInitialized
            | Self::AccountAlreadyInitialized
            | Self::InvalidPda => ErrorCategory::Access,
        }
    }

    /// Returns true if this error is potentially recoverable by the user.
    ///
    /// Recoverable errors are those where the user can take action to fix
    /// the issue (e.g., deposit more funds, reduce order size).
    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::InsufficientFunds
                | Self::InsufficientFreeBalance
                | Self::TooManyOrders
                | Self::NoFreeOrderSlot
                | Self::EventQueueFull
                | Self::PostOnlyWouldCross
                | Self::FillOrKillNotFillable
                | Self::OrderTooSmall
        )
    }

    /// Returns true if this error indicates a validation failure.
    #[must_use]
    pub const fn is_validation_error(&self) -> bool {
        matches!(self.category(), ErrorCategory::OrderValidation)
    }

    /// Returns true if this error indicates an arithmetic issue.
    #[must_use]
    pub const fn is_arithmetic_error(&self) -> bool {
        matches!(self.category(), ErrorCategory::Arithmetic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_category_market() {
        assert_eq!(
            MatchbookError::MarketNotActive.category(),
            ErrorCategory::Market
        );
        assert_eq!(
            MatchbookError::MarketCancelOnly.category(),
            ErrorCategory::Market
        );
        assert_eq!(
            MatchbookError::InvalidMarketState.category(),
            ErrorCategory::Market
        );
        assert_eq!(
            MatchbookError::MarketClosed.category(),
            ErrorCategory::Market
        );
    }

    #[test]
    fn test_error_category_order_validation() {
        assert_eq!(
            MatchbookError::InvalidTickSize.category(),
            ErrorCategory::OrderValidation
        );
        assert_eq!(
            MatchbookError::InvalidLotSize.category(),
            ErrorCategory::OrderValidation
        );
        assert_eq!(
            MatchbookError::InvalidPrice.category(),
            ErrorCategory::OrderValidation
        );
        assert_eq!(
            MatchbookError::InvalidQuantity.category(),
            ErrorCategory::OrderValidation
        );
    }

    #[test]
    fn test_error_category_balance() {
        assert_eq!(
            MatchbookError::InsufficientFunds.category(),
            ErrorCategory::Balance
        );
        assert_eq!(
            MatchbookError::InsufficientFreeBalance.category(),
            ErrorCategory::Balance
        );
        assert_eq!(
            MatchbookError::BalanceOverflow.category(),
            ErrorCategory::Balance
        );
    }

    #[test]
    fn test_error_category_order_state() {
        assert_eq!(
            MatchbookError::OrderNotFound.category(),
            ErrorCategory::OrderState
        );
        assert_eq!(
            MatchbookError::NotOrderOwner.category(),
            ErrorCategory::OrderState
        );
        assert_eq!(
            MatchbookError::TooManyOrders.category(),
            ErrorCategory::OrderState
        );
    }

    #[test]
    fn test_error_category_matching() {
        assert_eq!(
            MatchbookError::PostOnlyWouldCross.category(),
            ErrorCategory::Matching
        );
        assert_eq!(
            MatchbookError::FillOrKillNotFillable.category(),
            ErrorCategory::Matching
        );
        assert_eq!(
            MatchbookError::SelfTradeError.category(),
            ErrorCategory::Matching
        );
    }

    #[test]
    fn test_error_category_arithmetic() {
        assert_eq!(
            MatchbookError::ArithmeticOverflow.category(),
            ErrorCategory::Arithmetic
        );
        assert_eq!(
            MatchbookError::ArithmeticUnderflow.category(),
            ErrorCategory::Arithmetic
        );
        assert_eq!(
            MatchbookError::DivisionByZero.category(),
            ErrorCategory::Arithmetic
        );
    }

    #[test]
    fn test_error_category_queue() {
        assert_eq!(
            MatchbookError::EventQueueFull.category(),
            ErrorCategory::Queue
        );
        assert_eq!(
            MatchbookError::EventQueueEmpty.category(),
            ErrorCategory::Queue
        );
    }

    #[test]
    fn test_error_category_access() {
        assert_eq!(
            MatchbookError::Unauthorized.category(),
            ErrorCategory::Access
        );
        assert_eq!(
            MatchbookError::InvalidAuthority.category(),
            ErrorCategory::Access
        );
        assert_eq!(
            MatchbookError::InvalidDelegate.category(),
            ErrorCategory::Access
        );
    }

    #[test]
    fn test_is_recoverable() {
        // Recoverable errors
        assert!(MatchbookError::InsufficientFunds.is_recoverable());
        assert!(MatchbookError::TooManyOrders.is_recoverable());
        assert!(MatchbookError::EventQueueFull.is_recoverable());
        assert!(MatchbookError::PostOnlyWouldCross.is_recoverable());

        // Non-recoverable errors
        assert!(!MatchbookError::MarketClosed.is_recoverable());
        assert!(!MatchbookError::ArithmeticOverflow.is_recoverable());
        assert!(!MatchbookError::Unauthorized.is_recoverable());
        assert!(!MatchbookError::OrderNotFound.is_recoverable());
    }

    #[test]
    fn test_is_validation_error() {
        assert!(MatchbookError::InvalidPrice.is_validation_error());
        assert!(MatchbookError::InvalidQuantity.is_validation_error());
        assert!(MatchbookError::InvalidTickSize.is_validation_error());

        assert!(!MatchbookError::InsufficientFunds.is_validation_error());
        assert!(!MatchbookError::ArithmeticOverflow.is_validation_error());
    }

    #[test]
    fn test_is_arithmetic_error() {
        assert!(MatchbookError::ArithmeticOverflow.is_arithmetic_error());
        assert!(MatchbookError::ArithmeticUnderflow.is_arithmetic_error());
        assert!(MatchbookError::DivisionByZero.is_arithmetic_error());

        assert!(!MatchbookError::InvalidPrice.is_arithmetic_error());
        assert!(!MatchbookError::InsufficientFunds.is_arithmetic_error());
    }
}
