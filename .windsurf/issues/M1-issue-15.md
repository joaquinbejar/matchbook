# M1 Issue #15: Instruction: ConsumeEvents

## Overview

Implement the ConsumeEvents instruction that processes events from the event queue and settles funds to the appropriate OpenOrders accounts. This finalizes trades by updating balances.

## Acceptance Criteria

- [ ] `ConsumeEvents` instruction with parameters:
  - `limit`: u16 (max events to consume)
- [ ] Account validation:
  - Crank (signer, anyone can call)
  - Market (valid)
  - Event queue
  - OpenOrders accounts for affected users (remaining accounts)
- [ ] Process Fill events:
  - Credit maker with quote (for sells) or base (for buys)
  - Credit taker with base (for buys) or quote (for sells)
  - Deduct fees from appropriate party
  - Move funds from locked to free
- [ ] Process Out events:
  - Release locked funds to free balance
- [ ] Pop processed events from queue
- [ ] Handle missing OpenOrders gracefully (skip event)
- [ ] Unit tests for event processing

## Technical Approach

### 1. ConsumeEvents Accounts

Based on `.internalDoc/03-onchain-design.md`:

```rust
#[derive(Accounts)]
pub struct ConsumeEvents<'info> {
    /// Anyone can call this instruction (permissionless crank).
    pub crank: Signer<'info>,

    /// Market the events are for.
    #[account(
        has_one = event_queue @ MatchbookError::InvalidAccountData
    )]
    pub market: Account<'info, Market>,

    /// Event queue account.
    /// CHECK: Validated via has_one on market.
    #[account(mut)]
    pub event_queue: UncheckedAccount<'info>,

    // Remaining accounts: OpenOrders accounts for users in events
}
```

### 2. ConsumeEvents Parameters

```rust
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct ConsumeEventsParams {
    pub limit: u16,  // Max events to process
}
```

### 3. Event Processing Flow

1. Read events from queue head
2. For each event (up to limit):
   - If Fill event:
     - Find maker OpenOrders in remaining accounts
     - Find taker OpenOrders in remaining accounts
     - Settle maker: credit received tokens, debit locked tokens
     - Settle taker: credit received tokens, debit locked tokens
     - Apply fees
   - If Out event:
     - Find owner OpenOrders in remaining accounts
     - Release locked funds to free balance
   - Pop event from queue
3. Return number of events consumed

### 4. Fill Event Settlement

For a Fill where taker is buying (taker_side = Bid):
- Taker: receives base, pays quote + taker_fee
- Maker: receives quote - maker_fee (or + rebate), pays base

For a Fill where taker is selling (taker_side = Ask):
- Taker: receives quote - taker_fee, pays base
- Maker: receives base, pays quote - maker_fee (or + rebate)

### 5. Out Event Settlement

- Release `base_released` from base_locked to base_free
- Release `quote_released` from quote_locked to quote_free

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `program/src/instructions/consume_events.rs` | Create | ConsumeEvents instruction |
| `program/src/instructions/mod.rs` | Modify | Add consume_events module |
| `program/src/lib.rs` | Modify | Add ConsumeEvents accounts struct and instruction handler |

## Tests Needed

- `test_consume_events_params_valid` - Valid parameters
- `test_consume_events_params_zero_limit` - Zero limit (should fail)

## Notes

- Permissionless: anyone can call
- OpenOrders accounts passed as remaining accounts
- Events processed in FIFO order
- ~10K CU base + ~15K CU per event
- Missing OpenOrders = event skipped (retry later)
- Fees transferred to fee_destination (future enhancement)
