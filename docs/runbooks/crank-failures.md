# Crank Failures Runbook

## Overview

This runbook covers diagnosing and resolving high crank transaction failure rates.

## Symptoms

- `CrankHighFailureRate` alert firing (failure_rate > 10% for 5m)
- Orders not being matched despite crossed book
- Increased transaction errors in crank logs

## Impact

- **Matching**: Orders not matched in timely manner
- **User Experience**: Delayed trade execution
- **Profitability**: Wasted transaction fees on failed attempts

## Diagnostic Steps

### 1. Check Failure Rate

```bash
# Via Prometheus
curl -s "http://localhost:9092/api/v1/query?query=rate(crank_transactions_total{status=\"failed\"}[5m])/rate(crank_transactions_total[5m])" | jq
```

### 2. Check Crank Logs

```bash
# Docker
docker logs matchbook-crank --tail 200 | grep -E "(error|failed|Error)"

# Kubernetes
kubectl logs deployment/crank -n matchbook --tail=200 | grep -E "(error|failed|Error)"
```

### 3. Check Transaction Errors

```bash
# Common error types
kubectl logs deployment/crank -n matchbook | grep -E "(InsufficientFunds|BlockhashNotFound|AccountNotFound)" | tail -20
```

### 4. Check Solana Network

```bash
# Check slot
curl -s -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getSlot"}' \
  ${SOLANA_RPC_URL} | jq

# Check recent blockhash
curl -s -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLatestBlockhash"}' \
  ${SOLANA_RPC_URL} | jq
```

### 5. Check Crank Wallet

```bash
# Check balance
solana balance <CRANK_PUBKEY>

# Check recent transactions
solana transaction-history <CRANK_PUBKEY> --limit 10
```

## Resolution Steps

### Insufficient Funds

1. **Check balance**:
   ```bash
   solana balance <CRANK_PUBKEY>
   ```

2. **Fund wallet**:
   ```bash
   solana transfer <CRANK_PUBKEY> 5 --from <FUNDING_KEYPAIR>
   ```

### Blockhash Expired

1. **Reduce transaction build time**:
   ```bash
   kubectl set env deployment/crank -n matchbook TX_BUILD_TIMEOUT_MS=500
   ```

2. **Use more recent blockhash**:
   ```bash
   kubectl set env deployment/crank -n matchbook USE_DURABLE_NONCE=false
   ```

### RPC Issues

1. **Check RPC health**:
   ```bash
   curl -s -X POST -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' \
     ${SOLANA_RPC_URL}
   ```

2. **Switch RPC endpoint**:
   ```bash
   kubectl set env deployment/crank -n matchbook \
     SOLANA_RPC_URL=https://backup-rpc.example.com
   ```

### Account State Changed

1. **Check for concurrent crankers**:
   ```bash
   # Look for other crank transactions on same market
   solana transaction-history <MARKET_ADDRESS> --limit 20
   ```

2. **Implement optimistic locking** or reduce concurrency

### Priority Fee Too Low

1. **Check current priority fees**:
   ```bash
   curl -s -X POST -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","id":1,"method":"getRecentPrioritizationFees"}' \
     ${SOLANA_RPC_URL} | jq
   ```

2. **Increase priority fee**:
   ```bash
   kubectl set env deployment/crank -n matchbook PRIORITY_FEE_LAMPORTS=10000
   ```

## Prevention

1. **Adequate funding**: Maintain buffer in crank wallet
2. **RPC redundancy**: Configure multiple RPC endpoints
3. **Retry logic**: Implement exponential backoff
4. **Priority fees**: Dynamic fee adjustment based on network
5. **Monitoring**: Alert on failure rate before critical

## Related Alerts

- `CrankHighFailureRate` - Warning at failure_rate > 10%
- `CrankDown` - Crank service not responding
- `CrankNotMatching` - No matches despite crossed orders

## Escalation

If failure rate persists:

1. Check Solana network status
2. Review transaction error details
3. Consider pausing crank temporarily
4. Escalate to platform team
