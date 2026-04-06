# M1 Issue #11: Instruction: PlaceOrder

## Overview

Implement the PlaceOrder instruction that inserts a new order into the order book. This is the core trading instruction that handles order validation, balance locking, and book insertion.

## Acceptance Criteria

- [ ] `PlaceOrder` instruction with parameters:
  - `side`: Side (Bid/Ask)
  - `price`: u64 (in ticks)
  - `quantity`: u64 (in base lots)
  - `order_type`: OrderType (Limit, PostOnly, ImmediateOrCancel, FillOrKill)
  - `client_order_id`: u64
- [ ] Account validation:
  - Authority or delegate (signer)
  - Market (valid, active)
  - OpenOrders (owned by authority or delegate authorized)
  - Bids and Asks accounts
  - Event queue
- [ ] Validate price alignment to tick size
- [ ] Validate quantity > 0
- [ ] Calculate required funds and lock balance
- [ ] Find free order slot in OpenOrders
- [ ] Insert order into appropriate book side
- [ ] Handle PostOnly rejection if would cross
- [ ] Increment market seq_num for order ID
- [ ] Unit tests for all order types and error cases

## Technical Approach

### 1. PlaceOrder Accounts

Based on `.internalDoc/03-onchain-design.md`:

```rust
#[derive(Accounts)]
pub struct PlaceOrder<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        constraint = market.is_active() @ MatchbookError::MarketNotActive,
        has_one = bids @ MatchbookError::InvalidAccountData,
        has_one = asks @ MatchbookError::InvalidAccountData,
        has_one = event_queue @ MatchbookError::InvalidAccountData
    )]
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

### 2. PlaceOrder Parameters

```rust
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct PlaceOrderParams {
    pub side: Side,
    pub price: u64,
    pub quantity: u64,
    pub order_type: OrderType,
    pub client_order_id: u64,
}
```

### 3. OrderType Enum

```rust
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum OrderType {
    Limit,
    ImmediateOrCancel,
    PostOnly,
    FillOrKill,
}
```

### 4. Validation Rules

- Market must be active
- Authority must be owner or delegate
- Price must be > 0 and aligned to tick_size
- Quantity must be > 0 and >= min_order_size
- Sufficient free balance for the order

### 5. Balance Locking

- **Bids**: Lock quote = quantity * price * tick_size / lot_size
- **Asks**: Lock base = quantity * lot_size

### 6. Order ID Generation

```rust
let seq_num = market.next_seq_num()?;
let order_id = match side {
    Side::Bid => OrderId::new_bid(price, seq_num),
    Side::Ask => OrderId::new_ask(price, seq_num),
};
```

### 7. PostOnly Handling

Check if order would cross the spread before inserting:
- Bid at price >= best ask → reject
- Ask at price <= best bid → reject

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `program/src/state/orderbook.rs` | Modify | Add OrderType enum |
| `program/src/instructions/place_order.rs` | Create | PlaceOrder instruction |
| `program/src/instructions/mod.rs` | Modify | Add place_order module |
| `program/src/lib.rs` | Modify | Add PlaceOrder accounts struct and instruction handler |

## Tests Needed

- `test_place_order_params_valid` - Valid parameters
- `test_place_order_params_zero_price` - Zero price (should fail)
- `test_place_order_params_zero_quantity` - Zero quantity (should fail)
- `test_order_type_conversion` - OrderType enum conversion

## Notes

- Order ID = `(price << 64) | seq_num` for price-time priority
- Bids sorted descending, asks sorted ascending
- PostOnly orders rejected if they would match immediately
- IOC/FOK handled during matching, not placement (future issue)
- Lock quote for bids, base for asks
- This issue focuses on order placement only, not matching
