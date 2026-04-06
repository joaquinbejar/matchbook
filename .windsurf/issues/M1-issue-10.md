# M1 Issue #10: Instruction: Withdraw

## Overview

Implement the Withdraw instruction that transfers tokens from the market vaults back to a user's wallet, debiting their OpenOrders account.

## Acceptance Criteria

- [ ] `Withdraw` instruction with parameters:
  - `base_amount`: u64 (in base token smallest units)
  - `quote_amount`: u64 (in quote token smallest units)
- [ ] Account validation:
  - Authority (signer) - must be owner, NOT delegate
  - Market (valid) - withdrawal allowed even if paused/closed
  - OpenOrders (owned by authority)
  - User's base/quote token accounts
  - Market base/quote vaults
  - Token program
- [ ] Transfer tokens from vaults to user
- [ ] Update OpenOrders `base_free` and `quote_free`
- [ ] Validate sufficient free balance (not locked)
- [ ] Handle partial withdrawals
- [ ] Delegate CANNOT withdraw (only owner)
- [ ] Unit tests for withdrawals and error cases

## Technical Approach

### 1. Withdraw Accounts

Based on `.internalDoc/03-onchain-design.md`:

```rust
#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub owner: Signer<'info>,

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

### 2. Withdraw Parameters

```rust
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct WithdrawParams {
    pub base_amount: u64,
    pub quote_amount: u64,
}
```

### 3. Validation Rules

- Owner must sign (delegate cannot withdraw)
- At least one amount must be > 0
- Sufficient free balance available
- User token accounts must match market mints
- Vaults must match market vaults
- Withdrawal allowed even if market is paused/closed

### 4. Token Transfer (PDA signer)

Use SPL Token transfer with PDA signer:
```rust
token::transfer(
    CpiContext::new_with_signer(
        token_program,
        Transfer {
            from: base_vault,
            to: user_base_account,
            authority: market,
        },
        &[&market.signer_seeds(base_mint, quote_mint)],
    ),
    base_amount,
)?;
```

### 5. Balance Update

```rust
open_orders.base_free = open_orders.base_free
    .checked_sub(base_amount)
    .ok_or(MatchbookError::InsufficientFunds)?;
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `program/src/instructions/withdraw.rs` | Create | Withdraw instruction |
| `program/src/instructions/mod.rs` | Modify | Add withdraw module |
| `program/src/lib.rs` | Modify | Add Withdraw accounts struct and instruction handler |

## Tests Needed

- `test_withdraw_params_valid_base_only` - Valid base-only withdrawal
- `test_withdraw_params_valid_quote_only` - Valid quote-only withdrawal
- `test_withdraw_params_valid_both` - Valid both amounts
- `test_withdraw_params_both_zero` - Both amounts zero (should fail)

## Notes

- Only `free` balance can be withdrawn, not `locked`
- Uses PDA signer for vault transfers
- Withdrawal allowed even if market is paused/closed
- Balance underflow check required
- Delegate CANNOT withdraw (only owner)
