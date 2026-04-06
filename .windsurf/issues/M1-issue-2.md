# M1 Issue #2: Market Account Structure

## Overview

Implement the Market account structure that holds all market-level configuration and state. This is the central account that references all other market components.

## Acceptance Criteria

- [ ] `Market` struct with all required fields:
  - `bump`: u8 (PDA bump seed)
  - `authority`: Pubkey (market admin)
  - `base_mint` / `quote_mint`: Pubkey
  - `base_vault` / `quote_vault`: Pubkey
  - `bids` / `asks`: Pubkey (order book accounts)
  - `event_queue`: Pubkey
  - `base_lot_size` / `quote_lot_size`: u64
  - `tick_size`: u64
  - `min_order_size`: u64
  - `maker_fee_bps` / `taker_fee_bps`: i16/u16
  - `fee_destination`: Pubkey
  - `seq_num`: u64
  - `status`: MarketStatus enum
  - `reserved`: [u8; 64] for future use
- [ ] PDA derivation: `["market", base_mint, quote_mint]`
- [ ] `MarketStatus` enum (Active, Paused, Closed)
- [ ] Serialization with Anchor `#[account]` macro
- [ ] Unit tests for PDA derivation
- [ ] Documentation for all public items

## Technical Approach

### 1. Project Structure

```
program/src/
├── lib.rs              # Program entry point (modify)
├── state/
│   ├── mod.rs          # State module (new)
│   └── market.rs       # Market account (new)
└── constants.rs        # Seeds and constants (new)
```

### 2. Market Account Design

Based on `.internalDoc/03-onchain-design.md`:

```rust
#[account]
#[derive(Debug)]
pub struct Market {
    /// Bump seed for PDA derivation.
    pub bump: u8,
    
    /// Current market status.
    pub status: MarketStatus,
    
    /// Base token mint.
    pub base_mint: Pubkey,
    
    /// Quote token mint.
    pub quote_mint: Pubkey,
    
    /// Base token vault (PDA-controlled).
    pub base_vault: Pubkey,
    
    /// Quote token vault (PDA-controlled).
    pub quote_vault: Pubkey,
    
    /// Bids order book account.
    pub bids: Pubkey,
    
    /// Asks order book account.
    pub asks: Pubkey,
    
    /// Event queue account.
    pub event_queue: Pubkey,
    
    /// Authority that can modify market parameters.
    pub authority: Pubkey,
    
    /// Fee recipient account.
    pub fee_destination: Pubkey,
    
    /// Minimum base token quantity per lot.
    pub base_lot_size: u64,
    
    /// Minimum quote token quantity per lot.
    pub quote_lot_size: u64,
    
    /// Minimum price increment (quote atoms per tick).
    pub tick_size: u64,
    
    /// Minimum order size in lots.
    pub min_order_size: u64,
    
    /// Taker fee in basis points.
    pub taker_fee_bps: u16,
    
    /// Maker fee in basis points (can be negative for rebates).
    pub maker_fee_bps: i16,
    
    /// Sequence number for event ordering.
    pub seq_num: u64,
    
    /// Reserved for future use.
    pub reserved: [u8; 64],
}
```

### 3. MarketStatus Enum

```rust
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[repr(u8)]
pub enum MarketStatus {
    /// Normal operation - all actions allowed.
    #[default]
    Active = 0,
    /// Only cancellations and withdrawals allowed.
    Paused = 1,
    /// Market is permanently closed.
    Closed = 2,
}
```

### 4. PDA Seeds

```rust
pub const MARKET_SEED: &[u8] = b"market";

impl Market {
    pub const SEED_PREFIX: &'static [u8] = MARKET_SEED;
    
    pub fn derive_pda(
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        program_id: &Pubkey,
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[Self::SEED_PREFIX, base_mint.as_ref(), quote_mint.as_ref()],
            program_id,
        )
    }
}
```

### 5. Space Calculation

```
8 (discriminator) + 1 (bump) + 1 (status) + 32*10 (pubkeys) + 8*5 (u64s) + 2 + 2 (fees) + 64 (reserved)
= 8 + 1 + 1 + 320 + 40 + 4 + 64 = 438 bytes
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `program/src/state/mod.rs` | Create | State module exports |
| `program/src/state/market.rs` | Create | Market struct and MarketStatus enum |
| `program/src/constants.rs` | Create | PDA seeds and constants |
| `program/src/lib.rs` | Modify | Add module declarations |

## Tests Needed

- `test_market_pda_derivation` - Verify PDA seeds produce consistent addresses
- `test_market_status_default` - Verify default status is Active
- `test_market_space_calculation` - Verify space constant matches struct size

## Notes

- Use `#[repr(C)]` is not needed with Anchor's `#[account]` macro as it handles serialization
- Fees use basis points: 1 bp = 0.01%, so 30 bp = 0.3%
- Negative maker fee represents a rebate to market makers
- Reserved bytes allow future field additions without breaking compatibility
