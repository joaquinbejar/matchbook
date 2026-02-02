# Crank Profitability Runbook

## Overview

This runbook covers diagnosing and resolving issues when the crank is not profitable.

## Symptoms

- `CrankLowProfitability` alert firing (profit < 0 for 10m)
- Crank skipping matches due to unprofitability
- Negative profit metrics
- Orders not being matched despite crossed book

## Impact

- **Matching**: Orders may not be matched if crank requires profit
- **Sustainability**: Crank operation costs exceed revenue
- **User Experience**: Delayed or no order execution

## Diagnostic Steps

### 1. Check Current Profitability

```bash
# Via Prometheus
curl -s "http://localhost:9092/api/v1/query?query=crank_profit_lamports" | jq

# Check fees paid
curl -s "http://localhost:9092/api/v1/query?query=crank_fees_paid_lamports" | jq
```

### 2. Check Priority Fees

```bash
# Current priority fee setting
kubectl get deployment/crank -n matchbook -o yaml | grep PRIORITY_FEE

# Network priority fees
curl -s -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getRecentPrioritizationFees"}' \
  ${SOLANA_RPC_URL} | jq
```

### 3. Check Crank Revenue

```bash
# Check crank reward per match
curl -s http://crank:9091/metrics | grep crank_reward
```

### 4. Check Transaction Costs

```bash
# Average transaction cost
curl -s "http://localhost:9092/api/v1/query?query=avg(crank_tx_cost_lamports)" | jq
```

## Resolution Steps

### Lower Priority Fee

1. **Check current fee**:
   ```bash
   kubectl get deployment/crank -n matchbook -o yaml | grep PRIORITY_FEE
   ```

2. **Reduce priority fee**:
   ```bash
   kubectl set env deployment/crank -n matchbook PRIORITY_FEE_LAMPORTS=1000
   ```

### Enable Subsidized Mode

If profitability is not required:

1. **Set min profit to zero**:
   ```bash
   kubectl set env deployment/crank -n matchbook MIN_PROFIT_LAMPORTS=0
   ```

2. **Enable subsidized cranking**:
   ```bash
   kubectl set env deployment/crank -n matchbook SUBSIDIZE_CRANK=true
   ```

### Optimize Transaction Batching

1. **Increase matches per transaction**:
   ```bash
   kubectl set env deployment/crank -n matchbook MAX_MATCHES_PER_TX=16
   ```

2. **Batch events consumption**:
   ```bash
   kubectl set env deployment/crank -n matchbook MAX_EVENTS_PER_TX=50
   ```

### Wait for Network Conditions

If network fees are temporarily high:

1. **Monitor fee trends**:
   ```bash
   # Check fee history
   curl -s "http://localhost:9092/api/v1/query_range?query=crank_tx_cost_lamports&start=$(date -d '1 hour ago' +%s)&end=$(date +%s)&step=60" | jq
   ```

2. **Set dynamic fee adjustment**:
   ```bash
   kubectl set env deployment/crank -n matchbook DYNAMIC_PRIORITY_FEE=true
   ```

### Increase Crank Rewards

If protocol allows:

1. **Check current reward settings**:
   ```bash
   matchbook-cli market-info --market <MARKET_ADDRESS> | grep crank_reward
   ```

2. **Increase crank reward** (requires authority):
   ```bash
   matchbook-cli set-crank-reward --market <MARKET_ADDRESS> --reward 5000
   ```

## Prevention

1. **Dynamic fees**: Implement dynamic priority fee adjustment
2. **Batching**: Maximize operations per transaction
3. **Monitoring**: Alert before profitability goes negative
4. **Subsidization**: Budget for subsidized cranking during low-profit periods

## Related Alerts

- `CrankLowProfitability` - Warning: profit < 0 for 10m
- `CrankNotMatching` - May be caused by profitability issues
- `CrankHighFailureRate` - Failed transactions waste fees

## Escalation

If crank cannot be made profitable:

1. Review fee structure with business team
2. Consider subsidized operation for critical markets
3. Evaluate crank reward settings
4. Escalate to platform team for protocol changes
