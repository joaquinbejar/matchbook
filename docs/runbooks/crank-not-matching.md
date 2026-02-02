# Crank Not Matching Runbook

## Overview

This runbook covers diagnosing and resolving issues when the crank is not executing matches despite crossed orders in the book.

## Symptoms

- `CrankNotMatching` alert firing (crossed orders, no matches for 5m)
- Orders sitting in book that should match
- Users complaining orders not filling
- Spread showing negative (crossed book)

## Impact

- **Trading**: Orders not executed despite matching prices
- **User Experience**: Frustration with unfilled orders
- **Market Quality**: Crossed book indicates dysfunction
- **Arbitrage**: Opportunity for others to exploit

## Diagnostic Steps

### 1. Verify Crossed Orders Exist

```bash
# Check for crossed orders metric
curl -s "http://localhost:9092/api/v1/query?query=market_crossed_orders" | jq

# Check via API
curl -s http://api:8080/v1/markets/<MARKET_ID>/orderbook | jq '.bids[0], .asks[0]'
```

### 2. Check Crank Status

```bash
# Is crank running?
kubectl get pods -l app.kubernetes.io/name=crank -n matchbook

# Check crank logs
kubectl logs deployment/crank -n matchbook --tail=100

# Check match execution rate
curl -s "http://localhost:9092/api/v1/query?query=rate(crank_matches_executed_total[5m])" | jq
```

### 3. Check Profitability Settings

```bash
# Check min profit threshold
kubectl get deployment/crank -n matchbook -o yaml | grep -A 5 "MIN_PROFIT"

# Check current profit metric
curl -s http://crank:9091/metrics | grep crank_profit
```

### 4. Check Crank Wallet

```bash
# Check balance
solana balance <CRANK_PUBKEY>
```

### 5. Check Market State

```bash
# Check if market is active
matchbook-cli market-info --market <MARKET_ADDRESS>

# Check event queue
curl -s "http://localhost:9092/api/v1/query?query=orderbook_event_queue_depth" | jq
```

## Resolution Steps

### Crank Not Running

1. **Check pod status**:
   ```bash
   kubectl get pods -l app.kubernetes.io/name=crank -n matchbook
   ```

2. **Restart crank**:
   ```bash
   kubectl rollout restart deployment/crank -n matchbook
   ```

### Profitability Threshold Too High

1. **Check current threshold**:
   ```bash
   kubectl get deployment/crank -n matchbook -o yaml | grep MIN_PROFIT
   ```

2. **Lower threshold**:
   ```bash
   kubectl set env deployment/crank -n matchbook MIN_PROFIT_LAMPORTS=0
   ```

3. **Enable subsidized cranking** (accept losses):
   ```bash
   kubectl set env deployment/crank -n matchbook SUBSIDIZE_CRANK=true
   ```

### Crank Out of Funds

1. **Check balance**:
   ```bash
   solana balance <CRANK_PUBKEY>
   ```

2. **Fund wallet**:
   ```bash
   solana transfer <CRANK_PUBKEY> 5 --from <FUNDING_KEYPAIR>
   ```

### Market Paused or Inactive

1. **Check market state**:
   ```bash
   matchbook-cli market-info --market <MARKET_ADDRESS>
   ```

2. **Reactivate market** (requires authority):
   ```bash
   matchbook-cli set-market-state --market <MARKET_ADDRESS> --state active
   ```

### Manual Match Execution

If crank cannot be fixed immediately:

1. **Execute match manually**:
   ```bash
   matchbook-cli match-orders --market <MARKET_ADDRESS> --limit 10
   ```

### RPC Issues

1. **Check RPC connectivity**:
   ```bash
   curl -s -X POST -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' \
     ${SOLANA_RPC_URL}
   ```

2. **Switch RPC**:
   ```bash
   kubectl set env deployment/crank -n matchbook \
     SOLANA_RPC_URL=https://backup-rpc.example.com
   ```

## Prevention

1. **Profitability monitoring**: Alert when profit drops near threshold
2. **Wallet monitoring**: Alert when balance is low
3. **Crank redundancy**: Run backup crank instance
4. **Subsidized mode**: Consider always-on subsidized cranking for critical markets

## Related Alerts

- `CrankNotMatching` - Critical: crossed orders, no matches
- `CrankLowProfitability` - Warning: profit below zero
- `CrankDown` - Critical: service not responding

## Escalation

If matches cannot be executed:

1. Verify market state on-chain
2. Check for program bugs or exploits
3. Consider emergency market pause
4. Escalate to platform team immediately
