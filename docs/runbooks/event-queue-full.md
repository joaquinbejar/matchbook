# Event Queue Full Runbook

## Overview

This runbook covers diagnosing and resolving issues when the on-chain event queue approaches capacity.

## Symptoms

- `EventQueueNearFull` alert firing (queue > 80% capacity for 5m)
- `EventQueueNotDraining` alert firing
- PlaceOrder transactions failing with "event queue full" error
- Orders not being processed

## Impact

- **Trading**: New orders cannot be placed
- **Matching**: Existing orders cannot be matched
- **User Experience**: Trading halted for affected markets
- **Revenue**: Direct loss of trading activity

## Diagnostic Steps

### 1. Check Event Queue Depth

```bash
# Via Prometheus
curl -s "http://localhost:9092/api/v1/query?query=orderbook_event_queue_depth" | jq

# Check capacity percentage
curl -s "http://localhost:9092/api/v1/query?query=orderbook_event_queue_depth/orderbook_event_queue_capacity" | jq
```

### 2. Check Crank Service

```bash
# Check if crank is running
kubectl get pods -l app.kubernetes.io/name=crank -n matchbook

# Check crank logs
kubectl logs deployment/crank -n matchbook --tail=100

# Check crank metrics
curl -s http://crank:9091/metrics | grep -E "(consume_events|crank)"
```

### 3. Check ConsumeEvents Rate

```bash
# Events consumed per minute
curl -s "http://localhost:9092/api/v1/query?query=rate(crank_consume_events_total[5m])*60" | jq
```

### 4. Check Crank Wallet Balance

```bash
# Get crank wallet address from config
kubectl get secret matchbook-secrets -n matchbook -o jsonpath='{.data.CRANK_KEYPAIR}' | base64 -d > /tmp/crank.json

# Check balance
solana balance $(solana-keygen pubkey /tmp/crank.json)

# Clean up
rm /tmp/crank.json
```

### 5. Check Market State

```bash
# Use CLI to check market info
matchbook-cli market-info --market <MARKET_ADDRESS>
```

## Resolution Steps

### Crank Not Running

1. **Check crank pod status**:
   ```bash
   kubectl get pods -l app.kubernetes.io/name=crank -n matchbook
   kubectl describe pod <crank-pod> -n matchbook
   ```

2. **Restart crank**:
   ```bash
   kubectl rollout restart deployment/crank -n matchbook
   ```

### Crank Out of Funds

1. **Check balance**:
   ```bash
   solana balance <CRANK_PUBKEY>
   ```

2. **Fund crank wallet**:
   ```bash
   solana transfer <CRANK_PUBKEY> 1 --from <FUNDING_KEYPAIR>
   ```

### Manual Event Consumption

If crank is unavailable, manually consume events:

1. **Using CLI**:
   ```bash
   matchbook-cli consume-events --market <MARKET_ADDRESS> --limit 50
   ```

2. **Using Solana CLI** (emergency):
   ```bash
   # Build and send ConsumeEvents instruction
   solana program invoke <PROGRAM_ID> \
     --data <CONSUME_EVENTS_INSTRUCTION_DATA>
   ```

### High Event Volume

1. **Increase crank frequency**:
   ```bash
   kubectl set env deployment/crank -n matchbook CRANK_INTERVAL_MS=100
   ```

2. **Increase events per transaction**:
   ```bash
   kubectl set env deployment/crank -n matchbook MAX_EVENTS_PER_TX=50
   ```

3. **Scale crank** (if multiple markets):
   ```bash
   kubectl scale deployment/crank -n matchbook --replicas=2
   ```

### RPC Issues Preventing Crank

1. **Check RPC health**:
   ```bash
   curl -s -X POST -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' \
     ${SOLANA_RPC_URL}
   ```

2. **Switch to backup RPC**:
   ```bash
   kubectl set env deployment/crank -n matchbook \
     SOLANA_RPC_URL=https://backup-rpc.example.com
   ```

## Prevention

1. **Monitor queue depth**: Alert before reaching critical levels
2. **Crank redundancy**: Run multiple crank instances
3. **Adequate funding**: Maintain sufficient SOL in crank wallet
4. **RPC redundancy**: Configure backup RPC endpoints
5. **Capacity planning**: Size event queue for expected volume

## Related Alerts

- `EventQueueNearFull` - Critical at queue > 80%
- `EventQueueNotDraining` - Queue growing, no consumption
- `CrankDown` - Crank service not responding

## Escalation

If event queue remains full:

1. Verify crank is operational and funded
2. Check Solana network status
3. Consider emergency market pause if critical
4. Escalate to platform team for manual intervention
