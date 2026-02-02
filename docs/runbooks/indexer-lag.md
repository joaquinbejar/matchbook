# Indexer Lag Runbook

## Overview

This runbook covers diagnosing and resolving issues where the indexer falls behind the Solana blockchain tip.

## Symptoms

- `IndexerSlotLagHigh` alert firing (slot_lag > 100 for 5m)
- `IndexerSlotLagCritical` alert firing (slot_lag > 500 for 2m)
- Stale market data in API responses
- Order book not reflecting recent trades
- Users reporting delayed updates

## Impact

- **Market Data**: Prices and order book are stale
- **Trading**: Users may trade on outdated information
- **WebSocket**: Subscribers receive delayed updates
- **Crank**: May miss matching opportunities

## Diagnostic Steps

### 1. Check Current Lag

```bash
# Via Prometheus
curl -s "http://localhost:9092/api/v1/query?query=indexer_slot_lag" | jq '.data.result[0].value[1]'

# Via metrics endpoint
curl -s http://indexer:9090/metrics | grep indexer_slot_lag
```

### 2. Check Indexer Logs

```bash
# Docker
docker logs matchbook-indexer --tail 100 | grep -E "(error|warn|lag)"

# Kubernetes
kubectl logs -f deployment/indexer -n matchbook --tail=100 | grep -E "(error|warn|lag)"
```

### 3. Check Geyser Connection

```bash
# Check if Geyser is connected
curl -s http://indexer:9090/metrics | grep geyser_connected

# Check Geyser endpoint health
curl -s ${GEYSER_ENDPOINT}/health
```

### 4. Check Database Performance

```bash
# Check active queries
kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c \
  "SELECT pid, now() - pg_stat_activity.query_start AS duration, query 
   FROM pg_stat_activity 
   WHERE state = 'active' AND query NOT LIKE '%pg_stat_activity%'
   ORDER BY duration DESC LIMIT 10;"

# Check table sizes
kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c \
  "SELECT relname, pg_size_pretty(pg_total_relation_size(relid)) as size
   FROM pg_catalog.pg_statio_user_tables
   ORDER BY pg_total_relation_size(relid) DESC LIMIT 10;"
```

### 5. Check Resource Usage

```bash
# Docker
docker stats matchbook-indexer --no-stream

# Kubernetes
kubectl top pod -l app.kubernetes.io/name=indexer -n matchbook
```

## Resolution Steps

### Geyser Connection Issues

1. **Verify Geyser endpoint is accessible**:
   ```bash
   curl -v ${GEYSER_ENDPOINT}/health
   ```

2. **Check Geyser token**:
   ```bash
   # Verify GEYSER_X_TOKEN is set
   kubectl get secret matchbook-secrets -n matchbook -o jsonpath='{.data.GEYSER_X_TOKEN}' | base64 -d
   ```

3. **Restart indexer to reconnect**:
   ```bash
   # Docker
   docker restart matchbook-indexer
   
   # Kubernetes
   kubectl rollout restart deployment/indexer -n matchbook
   ```

### Database Bottleneck

1. **Kill long-running queries**:
   ```bash
   kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c \
     "SELECT pg_terminate_backend(pid) FROM pg_stat_activity 
      WHERE duration > interval '5 minutes' AND state = 'active';"
   ```

2. **Run vacuum if needed**:
   ```bash
   kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c \
     "VACUUM ANALYZE events; VACUUM ANALYZE order_history;"
   ```

3. **Check connection pool**:
   ```bash
   curl -s http://indexer:9090/metrics | grep db_pool
   ```

### Resource Exhaustion

1. **Scale up resources** (Kubernetes):
   ```bash
   kubectl set resources deployment/indexer -n matchbook \
     --limits=cpu=4,memory=8Gi \
     --requests=cpu=2,memory=4Gi
   ```

2. **Add more replicas** (if stateless):
   ```bash
   kubectl scale deployment/indexer -n matchbook --replicas=2
   ```

### High Event Volume

1. **Check event processing rate**:
   ```bash
   curl -s http://indexer:9090/metrics | grep indexer_events_processed
   ```

2. **Temporarily increase batch size** (if configurable):
   ```bash
   kubectl set env deployment/indexer -n matchbook BATCH_SIZE=1000
   ```

## Prevention

1. **Monitor lag trends**: Set up Grafana dashboard to track lag over time
2. **Capacity planning**: Scale resources before high-volume events
3. **Database maintenance**: Schedule regular vacuum and reindex
4. **Geyser redundancy**: Configure backup Geyser endpoints
5. **Alert tuning**: Adjust thresholds based on normal operation

## Related Alerts

- `IndexerSlotLagHigh` - Warning at slot_lag > 100
- `IndexerSlotLagCritical` - Critical at slot_lag > 500
- `IndexerDown` - Service not responding

## Escalation

If lag persists after following these steps:

1. Check Solana network status: https://status.solana.com
2. Contact Geyser provider if connection issues persist
3. Escalate to platform team for infrastructure review
