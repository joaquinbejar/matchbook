# M1 Issue #6: Error Types and Error Handling

## Overview

Define all error types for the on-chain program using Anchor's error handling. These errors provide clear feedback for failed transactions and enable proper client-side error handling.

## Acceptance Criteria

- [ ] `MatchbookError` enum with all error variants:
  - Market errors: `InvalidMarketState`, `MarketNotActive`, `InvalidLotSize`, `InvalidTickSize`
  - Order errors: `InvalidPrice`, `InvalidQuantity`, `InvalidSide`, `InvalidOrderType`, `OrderNotFound`, `TooManyOrders`
  - Balance errors: `InsufficientFunds`, `BalanceOverflow`
  - Queue errors: `EventQueueFull`, `EventQueueEmpty`
  - Access errors: `Unauthorized`, `InvalidAuthority`, `InvalidDelegate`
  - Arithmetic errors: `ArithmeticOverflow`, `ArithmeticUnderflow`, `DivisionByZero`
- [ ] Use Anchor `#[error_code]` macro
- [ ] Error codes start at 6000 (Anchor convention)
- [ ] Clear, lowercase error messages
- [ ] Documentation for each error variant

## Technical Approach

### 1. Project Structure

```
program/src/
├── lib.rs              # Update to include error module
├── error.rs            # New - Error types
└── state/              # Existing
```

### 2. Error Categories and Codes

Based on `.internalDoc/03-onchain-design.md`:

```rust
#[error_code]
pub enum MatchbookError {
    // Market errors (6000-6009)
    #[msg("market is not active")]
    MarketNotActive,
    #[msg("market is in cancel-only mode")]
    MarketCancelOnly,
    #[msg("invalid market state")]
    InvalidMarketState,
    #[msg("market is closed")]
    MarketClosed,

    // Order validation errors (6010-6029)
    #[msg("price is not a multiple of tick size")]
    InvalidTickSize,
    #[msg("quantity is not a multiple of lot size")]
    InvalidLotSize,
    #[msg("order size below minimum")]
    OrderTooSmall,
    #[msg("price is zero or exceeds maximum")]
    InvalidPrice,
    #[msg("invalid quantity")]
    InvalidQuantity,
    #[msg("invalid order side")]
    InvalidSide,
    #[msg("invalid order type")]
    InvalidOrderType,
    #[msg("invalid time in force")]
    InvalidTimeInForce,

    // Fund errors (6030-6039)
    #[msg("insufficient funds deposited")]
    InsufficientFunds,
    #[msg("insufficient free balance for withdrawal")]
    InsufficientFreeBalance,
    #[msg("balance overflow")]
    BalanceOverflow,

    // Order state errors (6040-6049)
    #[msg("order not found in book")]
    OrderNotFound,
    #[msg("not authorized to modify this order")]
    NotOrderOwner,
    #[msg("too many open orders")]
    TooManyOrders,
    #[msg("no free order slot available")]
    NoFreeOrderSlot,

    // Matching errors (6050-6059)
    #[msg("post-only order would cross the spread")]
    PostOnlyWouldCross,
    #[msg("fill-or-kill order cannot be fully filled")]
    FillOrKillNotFillable,
    #[msg("self-trade would occur")]
    SelfTradeError,

    // Arithmetic errors (6060-6069)
    #[msg("arithmetic overflow")]
    ArithmeticOverflow,
    #[msg("arithmetic underflow")]
    ArithmeticUnderflow,
    #[msg("division by zero")]
    DivisionByZero,

    // Event queue errors (6070-6079)
    #[msg("event queue is full")]
    EventQueueFull,
    #[msg("event queue is empty")]
    EventQueueEmpty,

    // Access control errors (6080-6089)
    #[msg("unauthorized")]
    Unauthorized,
    #[msg("invalid authority")]
    InvalidAuthority,
    #[msg("invalid delegate")]
    InvalidDelegate,
}
```

### 3. Helper Functions

```rust
impl MatchbookError {
    /// Returns true if this is a recoverable error.
    pub fn is_recoverable(&self) -> bool;
    
    /// Returns the error category.
    pub fn category(&self) -> ErrorCategory;
}

pub enum ErrorCategory {
    Market,
    Order,
    Balance,
    Matching,
    Arithmetic,
    Queue,
    Access,
}
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `program/src/error.rs` | Create | Error enum and helpers |
| `program/src/lib.rs` | Modify | Add error module and re-export |

## Tests Needed

- `test_error_messages` - Verify error messages are lowercase
- `test_error_codes` - Verify error codes are unique
- `test_error_category` - Verify category classification

## Notes

- Error codes start at 6000 per Anchor convention
- Messages are lowercase for consistency
- Each error has documentation explaining when it occurs
- Categories help with error handling on client side
