# M1 Issue #12: Instruction: CancelOrder

## Overview

Implement the CancelOrder instruction that removes a specific order from the order book and releases the locked funds back to the user's free balance.

## Acceptance Criteria

- [ ] `CancelOrder` instruction with parameters:
  - `order_id`: u128
  - `side`: Side (Bid/Ask)
- [ ] Account validation:
  - Authority or delegate (signer)
  - Market (valid)
  - OpenOrders (owned by authority or delegate authorized)
  - Bids or Asks account (based on side)
  - Event queue
- [ ] Find order in book by order_id
- [ ] Verify order belongs to the signer's OpenOrders
- [ ] Remove order from book
- [ ] Release locked funds to free balance
- [ ] Clear order slot in OpenOrders
- [ ] Push Out event to event queue
- [ ] Unit tests for successful cancel and error cases

## Technical Approach

### 1. CancelOrder Accounts

Based on `.internalDoc/03-onchain-design.md`:

```rust
#[derive(Accounts)]
pub struct CancelOrder<'info> {
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

### 2. CancelOrder Parameters

```rust
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CancelOrderParams {
    pub order_id: u128,
    pub side: Side,
}
```

### 3. Validation Rules

- Authority must be owner or delegate
- Order must exist in the book
- Order must belong to the signer's OpenOrders account
- Cancellation allowed even if market is paused (CancelOnly mode)

### 4. Order Removal Flow

1. Find order in OpenOrders by order_id
2. Get order slot index
3. Find and remove order from book (bids or asks based on side)
4. Calculate locked amount to release:
   - Bids: release quote = quantity * price * quote_lot_size / base_lot_size
   - Asks: release base = quantity * base_lot_size
5. Release locked funds to free balance
6. Clear order slot in OpenOrders
7. Push Out event to event queue

### 5. Out Event

```rust
OutEvent {
    side,
    owner: open_orders.key(),
    order_id,
    client_order_id,
    base_released,
    quote_released,
    reason: OutReason::Cancelled,
}
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `program/src/instructions/cancel_order.rs` | Create | CancelOrder instruction |
| `program/src/instructions/mod.rs` | Modify | Add cancel_order module |
| `program/src/lib.rs` | Modify | Add CancelOrder accounts struct and instruction handler |

## Tests Needed

- `test_cancel_order_params_valid` - Valid parameters
- `test_cancel_order_params_zero_order_id` - Zero order ID (should fail)

## Notes

- Order lookup by ID is O(log n) in critbit tree
- Must verify ownership before cancellation
- Out event records the cancellation for off-chain tracking
- Cancellation allowed even if market is paused (CancelOnly mode)
- Cancellation NOT allowed if market is closed
