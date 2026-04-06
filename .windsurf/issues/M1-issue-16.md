# M1 Issue #16: Integration tests and Devnet deployment scripts

## Overview

Implement comprehensive integration tests for the full order lifecycle and create deployment scripts for devnet testing. This validates all instructions work together correctly.

## Acceptance Criteria

- [ ] Integration test: Full order lifecycle
  - CreateMarket → CreateOpenOrders → Deposit → PlaceOrder → MatchOrders → ConsumeEvents → Withdraw
- [ ] Integration test: Multiple users trading
- [ ] Integration test: Partial fills and order book depth
- [ ] Integration test: All order types (Limit, PostOnly, IOC, FOK)
- [ ] Integration test: Cancel scenarios (single, all, partial fill then cancel)
- [ ] Integration test: Error conditions (insufficient funds, invalid price, etc.)
- [ ] Integration test: Edge cases (empty book, self-trade prevention if any)
- [ ] Devnet deployment script (deploy-devnet.sh)
- [ ] Market initialization script (init-market.sh)
- [ ] Test fixtures for common scenarios
- [ ] CI integration for running tests

## Technical Approach

### 1. Integration Test Structure

Create a `tests/` directory with integration tests using Anchor's test framework:

```
program/tests/
├── common/
│   ├── mod.rs           # Common test utilities
│   └── fixtures.rs      # Test fixtures
├── lifecycle.rs         # Full order lifecycle test
├── trading.rs           # Multiple users trading
├── order_types.rs       # All order types
├── cancellation.rs      # Cancel scenarios
├── errors.rs            # Error conditions
└── edge_cases.rs        # Edge cases
```

### 2. Deployment Scripts

Create scripts in `scripts/` directory:

```
scripts/
├── deploy-devnet.sh     # Deploy program to devnet
├── init-market.sh       # Initialize a test market
└── README.md            # Script documentation
```

### 3. Test Fixtures

Common test parameters:
- Base lot size: 1_000_000 (1 token with 6 decimals)
- Quote lot size: 1_000 (0.001 token)
- Tick size: 100 (0.0001 quote per base)
- Taker fee: 30 bps (0.3%)
- Maker fee: -10 bps (0.1% rebate)

### 4. CI Integration

Update GitHub Actions to run integration tests.

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `program/tests/common/mod.rs` | Create | Common test utilities |
| `program/tests/common/fixtures.rs` | Create | Test fixtures |
| `program/tests/lifecycle.rs` | Create | Full lifecycle test |
| `scripts/deploy-devnet.sh` | Create | Devnet deployment script |
| `scripts/init-market.sh` | Create | Market initialization script |
| `scripts/README.md` | Create | Script documentation |

## Notes

- Use Anchor's test framework with `anchor test`
- Tests should be deterministic and repeatable
- Devnet deployment requires SOL airdrop
- Document test market parameters
