# M1 Issue #9: Instruction: Deposit

## Overview

Implement the Deposit instruction that transfers tokens from a user's wallet to the market vaults, crediting their OpenOrders account.

## Acceptance Criteria

- [ ] `Deposit` instruction with parameters:
  - `base_amount`: u64 (in base lots)
  - `quote_amount`: u64 (in quote lots)
- [ ] Account validation:
  - Authority (signer)
  - Market (valid, active)
  - OpenOrders (owned by authority)
  - User's base/quote token accounts
  - Market base/quote vaults
  - Token program
- [ ] Transfer tokens from user to vaults
- [ ] Update OpenOrders `base_free` and `quote_free`
- [ ] Validate amounts are non-negative
- [ ] Handle partial deposits (base only, quote only, or both)
- [ ] Unit tests for deposits and error cases

## Technical Approach

### 1. Deposit Accounts

Based on `.internalDoc/03-onchain-design.md`:

```rust
#[derive(Accounts)]
pub struct Deposit<'info> {
    pub owner: Signer<'info>,

    #[account(
        constraint = market.is_active() @ MatchbookError::MarketNotActive
    )]
    pub market: Account<'info, Market>,

    #[account(
        mut,
        seeds = [OPEN_ORDERS_SEED, market.key().as_ref(), owner.key().as_ref()],
        bump = open_orders.bump,
        has_one = owner @ MatchbookError::Unauthorized
    )]
    pub open_orders: Account<'info, OpenOrders>,

    #[account(
        mut,
        constraint = user_base_account.mint == market.base_mint @ MatchbookError::InvalidAccountData
    )]
    pub user_base_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_quote_account.mint == market.quote_mint @ MatchbookError::InvalidAccountData
    )]
    pub user_quote_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        address = market.base_vault @ MatchbookError::InvalidAccountData
    )]
    pub base_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        address = market.quote_vault @ MatchbookError::InvalidAccountData
    )]
    pub quote_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}
```

### 2. Deposit Parameters

```rust
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DepositParams {
    pub base_amount: u64,
    pub quote_amount: u64,
}
```

### 3. Validation Rules

- Market must be active
- OpenOrders must be owned by signer
- At least one amount must be > 0
- User token accounts must match market mints
- Vaults must match market vaults
- Balance overflow check required

### 4. Token Transfer

Use SPL Token transfer:
```rust
token::transfer(
    CpiContext::new(
        token_program,
        Transfer {
            from: user_base_account,
            to: base_vault,
            authority: owner,
        },
    ),
    base_amount,
)?;
```

### 5. Balance Update

```rust
open_orders.base_free = open_orders.base_free
    .checked_add(base_amount)
    .ok_or(MatchbookError::BalanceOverflow)?;
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `program/src/instructions/deposit.rs` | Create | Deposit instruction |
| `program/src/instructions/mod.rs` | Modify | Add deposit module |
| `program/src/lib.rs` | Modify | Add Deposit accounts struct and instruction handler |

## Tests Needed

- `test_deposit_params_valid` - Valid parameters
- `test_deposit_params_both_zero` - Both amounts zero (should fail)

## Notes

- Uses SPL Token transfer instruction
- Amounts in lot units (multiply by lot_size for raw amount)
- User must have approved or own the token accounts
- Balance overflow check required
