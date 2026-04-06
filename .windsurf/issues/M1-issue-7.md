# M1 Issue #7: Instruction: CreateMarket

## Overview

Implement the CreateMarket instruction that initializes a new trading market with all associated accounts (order books, event queue, vaults).

## Acceptance Criteria

- [ ] `CreateMarket` instruction with parameters:
  - `base_lot_size`: u64
  - `quote_lot_size`: u64
  - `tick_size`: u64
  - `maker_fee_bps`: i16
  - `taker_fee_bps`: i16
- [ ] Account validation:
  - Authority (signer, payer)
  - Base mint and quote mint (valid SPL tokens)
  - Market PDA (init)
  - Bids and Asks PDAs (init)
  - Event queue PDA (init)
  - Base vault and quote vault (init as token accounts)
  - Fee destination (valid token account)
- [ ] Initialize all accounts with correct values
- [ ] Validate lot sizes and tick size (non-zero, power of 10)
- [ ] Validate fee bounds (-100 to 10000 bps)
- [ ] Emit market creation event via logs
- [ ] Unit tests for successful creation and all error cases

## Technical Approach

### 1. Project Structure

```
program/src/
├── lib.rs                    # Update with instruction
├── instructions/
│   ├── mod.rs               # New - Instructions module
│   └── create_market.rs     # New - CreateMarket instruction
├── state/                   # Existing
└── error.rs                 # Existing
```

### 2. CreateMarket Accounts

Based on `.internalDoc/03-onchain-design.md`:

```rust
#[derive(Accounts)]
pub struct CreateMarket<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub authority: Signer<'info>,

    pub base_mint: Account<'info, Mint>,
    pub quote_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = payer,
        space = 8 + Market::INIT_SPACE,
        seeds = [MARKET_SEED, base_mint.key().as_ref(), quote_mint.key().as_ref()],
        bump
    )]
    pub market: Account<'info, Market>,

    #[account(
        init,
        payer = payer,
        space = 8 + OrderBookSideHeader::INIT_SPACE,
        seeds = [BIDS_SEED, market.key().as_ref()],
        bump
    )]
    pub bids: Account<'info, OrderBookSideHeader>,

    #[account(
        init,
        payer = payer,
        space = 8 + OrderBookSideHeader::INIT_SPACE,
        seeds = [ASKS_SEED, market.key().as_ref()],
        bump
    )]
    pub asks: Account<'info, OrderBookSideHeader>,

    #[account(
        init,
        payer = payer,
        space = 8 + EventQueueHeader::INIT_SPACE,
        seeds = [EVENT_QUEUE_SEED, market.key().as_ref()],
        bump
    )]
    pub event_queue: Account<'info, EventQueueHeader>,

    #[account(
        init,
        payer = payer,
        token::mint = base_mint,
        token::authority = market,
        seeds = [b"base_vault", market.key().as_ref()],
        bump
    )]
    pub base_vault: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = payer,
        token::mint = quote_mint,
        token::authority = market,
        seeds = [b"quote_vault", market.key().as_ref()],
        bump
    )]
    pub quote_vault: Account<'info, TokenAccount>,

    /// CHECK: Fee recipient can be any account
    pub fee_recipient: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}
```

### 3. CreateMarket Parameters

```rust
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CreateMarketParams {
    pub tick_size: u64,
    pub base_lot_size: u64,
    pub quote_lot_size: u64,
    pub min_order_size: u64,
    pub taker_fee_bps: u16,
    pub maker_fee_bps: i16,
}
```

### 4. Validation Rules

- `tick_size > 0`
- `base_lot_size > 0`
- `quote_lot_size > 0`
- `min_order_size > 0`
- `taker_fee_bps <= 10000` (max 100%)
- `maker_fee_bps >= -100 && maker_fee_bps <= 10000` (rebate up to 1%)
- `base_mint != quote_mint`

### 5. Market Initialization

```rust
market.bump = bump;
market.status = MarketStatus::Active;
market.base_mint = base_mint.key();
market.quote_mint = quote_mint.key();
market.base_vault = base_vault.key();
market.quote_vault = quote_vault.key();
market.bids = bids.key();
market.asks = asks.key();
market.event_queue = event_queue.key();
market.tick_size = params.tick_size;
market.base_lot_size = params.base_lot_size;
market.quote_lot_size = params.quote_lot_size;
market.min_order_size = params.min_order_size;
market.taker_fee_bps = params.taker_fee_bps;
market.maker_fee_bps = params.maker_fee_bps;
market.fee_recipient = fee_recipient.key();
market.authority = authority.key();
market.seq_num = 0;
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `program/src/instructions/mod.rs` | Create | Instructions module |
| `program/src/instructions/create_market.rs` | Create | CreateMarket instruction |
| `program/src/lib.rs` | Modify | Add instructions module and instruction handler |
| `program/src/state/market.rs` | Modify | Add missing fields if needed |

## Tests Needed

- `test_create_market_success` - Successful market creation
- `test_create_market_invalid_tick_size` - Zero tick size
- `test_create_market_invalid_lot_size` - Zero lot size
- `test_create_market_invalid_fee` - Fee out of bounds
- `test_create_market_same_mints` - Base and quote are same mint

## Notes

- Market authority can pause/close market later
- Vaults are PDAs owned by the program (market as authority)
- Order book accounts need to be large enough for expected depth
- Event queue needs to be large enough for expected throughput
