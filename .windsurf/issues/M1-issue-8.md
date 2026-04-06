# M1 Issue #8: Instruction: CreateOpenOrders

## Overview

Implement the CreateOpenOrders instruction that initializes a user's trading account for a specific market. Users must create this account before they can deposit funds or place orders.

## Acceptance Criteria

- [ ] `CreateOpenOrders` instruction with parameters:
  - `delegate`: Option<Pubkey> (optional trading delegate)
- [ ] Account validation:
  - Authority (signer, payer)
  - Market (valid, active)
  - OpenOrders PDA (init)
- [ ] Initialize OpenOrders with:
  - All balances set to 0
  - All order slots empty
  - Delegate if provided
- [ ] Validate market is active
- [ ] Unit tests for successful creation and error cases

## Technical Approach

### 1. CreateOpenOrders Accounts

Based on `.internalDoc/03-onchain-design.md`:

```rust
#[derive(Accounts)]
pub struct CreateOpenOrders<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub owner: Signer<'info>,

    #[account(
        constraint = market.is_active() @ MatchbookError::MarketNotActive
    )]
    pub market: Account<'info, Market>,

    #[account(
        init,
        payer = payer,
        space = 8 + OpenOrders::INIT_SPACE,
        seeds = [OPEN_ORDERS_SEED, market.key().as_ref(), owner.key().as_ref()],
        bump
    )]
    pub open_orders: Account<'info, OpenOrders>,

    pub system_program: Program<'info, System>,
}
```

### 2. CreateOpenOrders Parameters

```rust
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CreateOpenOrdersParams {
    pub delegate: Option<Pubkey>,
}
```

### 3. Validation Rules

- Market must be active (not paused or closed)
- Owner must sign the transaction
- OpenOrders PDA must not already exist

### 4. OpenOrders Initialization

```rust
open_orders.bump = bump;
open_orders.market = market.key();
open_orders.owner = owner.key();
open_orders.delegate = delegate.unwrap_or(Pubkey::default());
open_orders.base_locked = 0;
open_orders.quote_locked = 0;
open_orders.base_free = 0;
open_orders.quote_free = 0;
open_orders.referrer_rebates = 0;
open_orders.num_orders = 0;
open_orders.reserved = [0u8; 64];
open_orders.orders = [OrderSlot::default(); MAX_ORDERS];
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `program/src/instructions/create_open_orders.rs` | Create | CreateOpenOrders instruction |
| `program/src/instructions/mod.rs` | Modify | Add create_open_orders module |
| `program/src/lib.rs` | Modify | Add CreateOpenOrders accounts struct and instruction handler |

## Tests Needed

- `test_create_open_orders_success` - Successful creation
- `test_create_open_orders_with_delegate` - Creation with delegate
- `test_create_open_orders_market_not_active` - Market is paused/closed

## Notes

- One OpenOrders account per user per market
- PDA ensures uniqueness: `["open_orders", market, owner]`
- Account size must accommodate max order slots (128)
- Delegate can be set/changed later via separate instruction
