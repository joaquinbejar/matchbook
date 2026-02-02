# API Latency Runbook

## Overview

This runbook covers diagnosing and resolving high API latency issues.

## Symptoms

- `APIHighLatency` alert firing (p99 > 500ms for 5m)
- Slow API responses reported by users
- Client timeouts
- Increased error rates due to timeouts

## Impact

- **User Experience**: Slow trading interface
- **Trading**: Order placement delays
- **Integrations**: Third-party systems timing out
- **Revenue**: Potential loss of trading activity

## Diagnostic Steps

### 1. Check Current Latency

```bash
# Via Prometheus
curl -s "http://localhost:9092/api/v1/query?query=histogram_quantile(0.99,rate(api_request_duration_seconds_bucket[5m]))" | jq '.data.result[0].value[1]'

# Check latency by endpoint
curl -s "http://localhost:9092/api/v1/query?query=histogram_quantile(0.99,sum(rate(api_request_duration_seconds_bucket[5m]))by(le,path))" | jq
```

### 2. Identify Slow Endpoints

```bash
# Check which endpoints are slow
curl -s http://api:8080/metrics | grep api_request_duration_seconds_sum | sort -t' ' -k2 -rn | head -10
```

### 3. Check Database Query Performance

```bash
# Check slow queries
kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c \
  "SELECT query, calls, mean_time, max_time, total_time
   FROM pg_stat_statements
   ORDER BY mean_time DESC
   LIMIT 10;"

# Check active connections
kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c \
  "SELECT count(*) FROM pg_stat_activity WHERE state = 'active';"
```

### 4. Check Redis Cache

```bash
# Check Redis hit rate
kubectl exec -it redis-0 -n matchbook -- redis-cli info stats | grep -E "(hits|misses)"

# Check Redis memory
kubectl exec -it redis-0 -n matchbook -- redis-cli info memory | grep used_memory_human
```

### 5. Check API Resource Usage

```bash
# Docker
docker stats matchbook-api --no-stream

# Kubernetes
kubectl top pod -l app.kubernetes.io/name=api -n matchbook
```

### 6. Check Request Volume

```bash
# Current request rate
curl -s "http://localhost:9092/api/v1/query?query=sum(rate(api_requests_total[5m]))" | jq '.data.result[0].value[1]'
```

## Resolution Steps

### High Database Latency

1. **Analyze slow queries**:
   ```bash
   kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c \
     "SELECT query, calls, mean_time FROM pg_stat_statements 
      WHERE mean_time > 100 ORDER BY mean_time DESC LIMIT 5;"
   ```

2. **Add missing indexes** (if identified):
   ```sql
   CREATE INDEX CONCURRENTLY idx_orders_market_status 
   ON orders(market_id, status) WHERE status = 'open';
   ```

3. **Increase connection pool**:
   ```bash
   kubectl set env deployment/api -n matchbook DATABASE_POOL_SIZE=50
   ```

### Cache Miss Issues

1. **Check cache configuration**:
   ```bash
   kubectl exec -it redis-0 -n matchbook -- redis-cli config get maxmemory
   ```

2. **Warm up cache** (if recently restarted):
   ```bash
   # Trigger cache population for hot markets
   curl http://api:8080/v1/markets
   ```

3. **Increase cache TTL** for stable data:
   ```bash
   kubectl set env deployment/api -n matchbook CACHE_TTL_SECONDS=300
   ```

### High Request Volume

1. **Scale API horizontally**:
   ```bash
   kubectl scale deployment/api -n matchbook --replicas=5
   ```

2. **Enable rate limiting** (if not already):
   ```bash
   kubectl annotate ingress matchbook-ingress -n matchbook \
     nginx.ingress.kubernetes.io/limit-rps="100"
   ```

3. **Check for abuse**:
   ```bash
   # Find top requesters
   kubectl logs deployment/api -n matchbook | grep "GET\|POST" | \
     awk '{print $1}' | sort | uniq -c | sort -rn | head -10
   ```

### Resource Exhaustion

1. **Increase API resources**:
   ```bash
   kubectl set resources deployment/api -n matchbook \
     --limits=cpu=4,memory=4Gi \
     --requests=cpu=2,memory=2Gi
   ```

2. **Check for memory leaks**:
   ```bash
   # Monitor memory over time
   kubectl top pod -l app.kubernetes.io/name=api -n matchbook --containers
   ```

## Prevention

1. **Query optimization**: Regular review of slow query logs
2. **Caching strategy**: Cache frequently accessed, rarely changing data
3. **Connection pooling**: Properly sized database connection pools
4. **Load testing**: Regular performance testing before releases
5. **Autoscaling**: Configure HPA for API deployment

## Related Alerts

- `APIHighLatency` - Warning at p99 > 500ms
- `APIHighErrorRate` - Often correlated with latency issues
- `DatabaseConnectionsHigh` - May cause latency

## Escalation

If latency persists:

1. Check if issue is isolated to specific endpoints
2. Review recent deployments for regressions
3. Escalate to platform team for infrastructure review
