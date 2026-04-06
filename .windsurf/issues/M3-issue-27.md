# M3 Issue #27: Rust SDK: Instruction builders for transaction construction

## Overview

Implement instruction builders that construct Solana transactions for all Matchbook operations. These builders handle account resolution, PDA derivation, and instruction serialization.

## Acceptance Criteria

### Instruction Builders
- [ ] `CreateMarketBuilder`
- [ ] `CreateOpenOrdersBuilder`
- [ ] `DepositBuilder`
- [ ] `WithdrawBuilder`
- [ ] `PlaceOrderBuilder`
- [ ] `CancelOrderBuilder`
- [ ] `CancelAllOrdersBuilder`
- [ ] `MatchOrdersBuilder`
- [ ] `ConsumeEventsBuilder`

### Requirements
- [ ] Automatic PDA derivation for all accounts
- [ ] Builder pattern with fluent API
- [ ] Error handling with descriptive messages
- [ ] Unit tests for each builder
- [ ] Documentation with usage examples

## Technical Approach

### 1. Project Structure

Add to existing `sdk` crate:

```
sdk/src/
├── instructions/
│   ├── mod.rs           # Module exports
│   ├── pda.rs           # PDA derivation utilities
│   ├── create_market.rs
│   ├── create_open_orders.rs
│   ├── deposit.rs
│   ├── withdraw.rs
│   ├── place_order.rs
│   ├── cancel_order.rs
│   ├── cancel_all_orders.rs
│   ├── match_orders.rs
│   └── consume_events.rs
```

### 2. PDA Derivation

Based on `.internalDoc/03-onchain-design.md`:

| Account | Seeds |
|---------|-------|
| Market | `[b"market", base_mint, quote_mint]` |
| Bids | `[market, b"bids"]` |
| Asks | `[market, b"asks"]` |
| Event Queue | `[market, b"event_queue"]` |
| Base Vault | `[market, b"base_vault"]` |
| Quote Vault | `[market, b"quote_vault"]` |
| OpenOrders | `[b"open_orders", market, owner]` |

### 3. Builder Pattern

Each builder follows this pattern:
```rust
pub struct PlaceOrderBuilder {
    program_id: Pubkey,
    market: Pubkey,
    owner: Pubkey,
    // ... other required fields
}

impl PlaceOrderBuilder {
    pub fn new(program_id: Pubkey) -> Self { ... }
    pub fn market(mut self, market: Pubkey) -> Self { ... }
    pub fn owner(mut self, owner: Pubkey) -> Self { ... }
    pub fn build(self) -> Result<Instruction, SdkError> { ... }
}
```

### 4. Instruction Data Serialization

Use borsh for instruction data serialization matching on-chain format.

## Files to Create/Modify

| File | Description |
|------|-------------|
| `sdk/src/instructions/mod.rs` | Module exports |
| `sdk/src/instructions/pda.rs` | PDA derivation utilities |
| `sdk/src/instructions/create_market.rs` | CreateMarket builder |
| `sdk/src/instructions/create_open_orders.rs` | CreateOpenOrders builder |
| `sdk/src/instructions/deposit.rs` | Deposit builder |
| `sdk/src/instructions/withdraw.rs` | Withdraw builder |
| `sdk/src/instructions/place_order.rs` | PlaceOrder builder |
| `sdk/src/instructions/cancel_order.rs` | CancelOrder builder |
| `sdk/src/instructions/cancel_all_orders.rs` | CancelAllOrders builder |
| `sdk/src/instructions/match_orders.rs` | MatchOrders builder |
| `sdk/src/instructions/consume_events.rs` | ConsumeEvents builder |
| `sdk/src/lib.rs` | Add instructions module |
| `sdk/Cargo.toml` | Add solana-sdk, borsh dependencies |

## Notes

- Use solana-sdk for Pubkey, Instruction, AccountMeta
- Use borsh for instruction data serialization
- Cache PDA bumps where possible for efficiency
- All builders return Result<Instruction, SdkError>
