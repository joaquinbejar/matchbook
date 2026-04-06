# M1 Issue #13: Instruction: CancelAllOrders

## Overview

Implement the CancelAllOrders instruction that cancels all open orders for a user in a single transaction. This is useful for market makers and emergency situations.

## Acceptance Criteria

- [ ] `CancelAllOrders` instruction with parameters:
  - `side`: Option<Side> (None = both sides)
  - `limit`: u8 (max orders to cancel per call)
- [ ] Account validation:
  - Authority or delegate (signer)
  - Market (valid)
  - OpenOrders (owned by authority or delegate authorized)
  - Bids and Asks accounts
  - Event queue
- [ ] Iterate through OpenOrders slots
- [ ] Cancel each active order up to limit
- [ ] Release all locked funds to free balance
- [ ] Push Out events for each cancelled order
- [ ] Return number of orders cancelled
- [ ] Unit tests for batch cancellation

## Technical Approach

### 1. CancelAllOrders Accounts

Same as CancelOrder but needs both bids and asks:

```rust
#[derive(Accounts)]
pub struct CancelAllOrders<'info> {
    pub authority: Signer<'info>,

    pub market: Account<'info, Market>,

    #[account(
        mut,
        seeds = [OPEN_ORDERS_SEED, market.key().as_ref(), open_orders.owner.as_ref()],
        bump = open_orders.bump,
        constraint = open_orders.is_authorized(authority.key) @ MatchbookError::Unauthorized
    )]
    pub open_orders: Account<'info, OpenOrders>,

    /// CHECK: Validated via has_one on market
    #[account(mut)]
    pub bids: UncheckedAccount<'info>,

    /// CHECK: Validated via has_one on market
    #[account(mut)]
    pub asks: UncheckedAccount<'info>,

    /// CHECK: Validated via has_one on market
    #[account(mut)]
    pub event_queue: UncheckedAccount<'info>,
}
```

### 2. CancelAllOrders Parameters

```rust
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CancelAllOrdersParams {
    pub side: Option<Side>,  // None = both sides
    pub limit: u8,           // Max orders to cancel (for compute budget)
}
```

### 3. Validation Rules

- Authority must be owner or delegate
- Limit must be > 0
- Cancellation allowed even if market is paused (CancelOnly mode)
- Cancellation NOT allowed if market is closed

### 4. Cancellation Flow

1. Iterate through OpenOrders slots
2. For each active order:
   - Check if side matches filter (or None = all)
   - Remove order from book
   - Calculate and release locked funds
   - Clear order slot
   - Push Out event
   - Increment cancelled count
3. Stop when limit reached or no more orders
4. Return number of orders cancelled

### 5. Compute Budget

- ~40,000 base + ~20,000 per order cancelled
- Limit parameter prevents compute budget exhaustion
- May need multiple calls to cancel all orders

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `program/src/instructions/cancel_all_orders.rs` | Create | CancelAllOrders instruction |
| `program/src/instructions/mod.rs` | Modify | Add cancel_all_orders module |
| `program/src/lib.rs` | Modify | Add CancelAllOrders accounts struct and instruction handler |

## Tests Needed

- `test_cancel_all_orders_params_valid` - Valid parameters
- `test_cancel_all_orders_params_zero_limit` - Zero limit (should fail)
- `test_cancel_all_orders_params_with_side_filter` - With side filter

## Notes

- Limit parameter prevents compute budget exhaustion
- May need multiple calls to cancel all orders
- Useful for emergency market exit
- Reuses logic from CancelOrder for individual order cancellation
