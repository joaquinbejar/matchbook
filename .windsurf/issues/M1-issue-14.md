# M1 Issue #14: Instruction: MatchOrders (Crank)

## Overview

Implement the MatchOrders instruction that executes the matching algorithm, crossing orders from both sides of the book and generating fill events. This is the permissionless crank instruction.

## Acceptance Criteria

- [ ] `MatchOrders` instruction with parameters:
  - `limit`: u8 (max matches per call)
- [ ] Account validation:
  - Crank (signer, anyone can call)
  - Market (valid, active)
  - Bids and Asks accounts
  - Event queue
- [ ] Matching algorithm:
  - Find best bid and best ask
  - Match if bid price >= ask price
  - Execute at maker's price (price improvement for taker)
  - Reduce quantities, remove filled orders
  - Generate Fill events for both sides
- [ ] Calculate and record fees (maker/taker)
- [ ] Update market seq_num
- [ ] Respect compute budget with limit parameter
- [ ] Unit tests for matching scenarios

## Technical Approach

### 1. MatchOrders Accounts

Based on `.internalDoc/03-onchain-design.md`:

```rust
#[derive(Accounts)]
pub struct MatchOrders<'info> {
    /// Anyone can call this instruction (permissionless crank).
    pub crank: Signer<'info>,

    /// Market to match orders on.
    #[account(
        mut,
        has_one = bids @ MatchbookError::InvalidAccountData,
        has_one = asks @ MatchbookError::InvalidAccountData,
        has_one = event_queue @ MatchbookError::InvalidAccountData
    )]
    pub market: Account<'info, Market>,

    /// Bids order book account.
    /// CHECK: Validated via has_one on market.
    #[account(mut)]
    pub bids: UncheckedAccount<'info>,

    /// Asks order book account.
    /// CHECK: Validated via has_one on market.
    #[account(mut)]
    pub asks: UncheckedAccount<'info>,

    /// Event queue account.
    /// CHECK: Validated via has_one on market.
    #[account(mut)]
    pub event_queue: UncheckedAccount<'info>,
}
```

### 2. MatchOrders Parameters

```rust
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct MatchOrdersParams {
    pub limit: u8,  // Max matches to execute
}
```

### 3. Matching Algorithm

1. Get best bid (highest price) and best ask (lowest price)
2. Check if prices cross: bid_price >= ask_price
3. If crossed:
   - Match quantity = min(bid_quantity, ask_quantity)
   - Execute at maker's price (older order)
   - Calculate fees (taker/maker)
   - Generate FillEvent for both sides
   - Update or remove orders from book
   - Increment match count
4. Repeat until limit reached or no more crosses

### 4. Fee Calculation

```rust
// Taker fee: always positive
taker_fee = (quantity * price * quote_lot_size / base_lot_size) * taker_fee_bps / 10_000

// Maker fee: can be negative (rebate)
maker_fee = (quantity * price * quote_lot_size / base_lot_size) * maker_fee_bps / 10_000
```

### 5. Fill Event Structure

```rust
FillEvent {
    taker_side,
    maker: maker_open_orders,
    taker: taker_open_orders,
    maker_order_id,
    taker_order_id,
    price,
    quantity,
    taker_fee,
    maker_fee,
    seq_num,
}
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `program/src/instructions/match_orders.rs` | Create | MatchOrders instruction |
| `program/src/instructions/mod.rs` | Modify | Add match_orders module |
| `program/src/lib.rs` | Modify | Add MatchOrders accounts struct and instruction handler |

## Tests Needed

- `test_match_orders_params_valid` - Valid parameters
- `test_match_orders_params_zero_limit` - Zero limit (should fail)

## Notes

- Permissionless: anyone can call to earn priority
- Match price = maker's price (older order)
- Partial fills supported
- Fill events contain both maker and taker info
- ~50K CU base + ~100K CU per match
- Limit prevents compute exhaustion
