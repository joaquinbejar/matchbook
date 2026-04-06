# M2 Issue #24: Crank service: Monitor and execute matching

## Overview

Implement the crank service that monitors order books for crossing orders and submits MatchOrders transactions. This is a permissionless service that earns priority fees for executing matches.

## Acceptance Criteria

### Cross Detection
- [ ] Monitor best bid/ask from book builder
- [ ] Detect when bid >= ask (crossing condition)
- [ ] Estimate number of matches possible

### Transaction Building
- [ ] Build MatchOrders instruction with appropriate limit
- [ ] Build ConsumeEvents instruction for settlement
- [ ] Bundle into single transaction when possible

### Priority Fee Management
- [ ] Dynamic fee calculation based on network congestion
- [ ] Profitability check (fees earned > tx cost)
- [ ] Configurable min/max priority fee

### Transaction Submission
- [ ] Send with retries and confirmation tracking
- [ ] Handle transaction failures gracefully
- [ ] Avoid duplicate submissions

### Other
- [ ] Metrics: matches/sec, tx success rate, fees earned, latency
- [ ] Configuration for markets to crank
- [ ] Unit tests for cross detection and tx building

## Technical Approach

### 1. Project Structure

Create a new `crank` crate:

```
crank/
├── Cargo.toml
├── src/
│   ├── lib.rs           # Crate root
│   ├── config.rs        # Crank configuration
│   ├── detector.rs      # Cross detection logic
│   ├── builder.rs       # Transaction building
│   ├── submitter.rs     # Transaction submission
│   ├── service.rs       # Main crank service
│   └── metrics.rs       # Crank metrics
```

### 2. Core Types

```rust
pub struct CrankConfig {
    pub markets: Vec<[u8; 32]>,
    pub min_priority_fee: u64,
    pub max_priority_fee: u64,
    pub poll_interval_ms: u64,
    pub max_retries: u32,
}

pub struct CrossDetector {
    book_builder: Arc<RwLock<BookBuilder>>,
}

pub struct TransactionBuilder {
    program_id: Pubkey,
}

pub struct TransactionSubmitter {
    rpc_client: RpcClient,
    config: SubmitterConfig,
}

pub struct CrankService {
    config: CrankConfig,
    detector: CrossDetector,
    builder: TransactionBuilder,
    submitter: TransactionSubmitter,
    metrics: Arc<CrankMetrics>,
}
```

### 3. Cross Detection

```rust
pub struct CrossInfo {
    pub market: [u8; 32],
    pub best_bid: u64,
    pub best_ask: u64,
    pub estimated_matches: u32,
}

impl CrossDetector {
    pub async fn detect_cross(&self, market: &[u8; 32]) -> Option<CrossInfo>;
}
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `crank/Cargo.toml` | Create | Crank crate dependencies |
| `crank/src/lib.rs` | Create | Crate root |
| `crank/src/config.rs` | Create | Configuration |
| `crank/src/detector.rs` | Create | Cross detection |
| `crank/src/builder.rs` | Create | Transaction building |
| `crank/src/submitter.rs` | Create | Transaction submission |
| `crank/src/service.rs` | Create | Main service |
| `crank/src/metrics.rs` | Create | Metrics |
| `Cargo.toml` | Modify | Add crank to workspace |

## Notes

- Permissionless: competes with other cranks
- Priority fees incentivize faster inclusion
- Consider Jito bundles for MEV protection
- Multiple markets can be cranked in parallel
- Backoff on repeated failures
