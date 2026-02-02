# Database Connections Runbook

## Overview

This runbook covers diagnosing and resolving database connection issues.

## Symptoms

- `DatabaseConnectionErrors` alert firing (> 10 errors/min for 2m)
- `DatabaseConnectionPoolExhausted` alert firing
- Services failing to connect to database
- Increased API error rates
- Slow query responses

## Impact

- **API**: Requests failing with database errors
- **Indexer**: Unable to persist market data
- **Data Integrity**: Potential data loss if writes fail

## Diagnostic Steps

### 1. Check Connection Errors

```bash
# Via Prometheus
curl -s "http://localhost:9092/api/v1/query?query=rate(db_connection_errors_total[1m])" | jq
```

### 2. Check Database Status

```bash
# Check if database is running
kubectl exec -it postgres-0 -n matchbook -- pg_isready -U matchbook

# Check database logs
kubectl logs postgres-0 -n matchbook --tail=100
```

### 3. Check Connection Pool

```bash
# Check pool metrics
curl -s http://api:8080/metrics | grep db_pool

# Check active connections
kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c \
  "SELECT count(*) FROM pg_stat_activity;"
```

### 4. Check Max Connections

```bash
# Check max connections setting
kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c \
  "SHOW max_connections;"

# Check current usage
kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c \
  "SELECT count(*), state FROM pg_stat_activity GROUP BY state;"
```

### 5. Check for Long-Running Queries

```bash
kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c \
  "SELECT pid, now() - pg_stat_activity.query_start AS duration, query, state
   FROM pg_stat_activity
   WHERE state != 'idle'
   ORDER BY duration DESC
   LIMIT 10;"
```

## Resolution Steps

### Connection Pool Exhausted

1. **Increase pool size**:
   ```bash
   kubectl set env deployment/api -n matchbook DATABASE_POOL_SIZE=50
   kubectl set env deployment/indexer -n matchbook DATABASE_POOL_SIZE=30
   ```

2. **Restart services to apply**:
   ```bash
   kubectl rollout restart deployment/api -n matchbook
   kubectl rollout restart deployment/indexer -n matchbook
   ```

### Too Many Connections

1. **Kill idle connections**:
   ```bash
   kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c \
     "SELECT pg_terminate_backend(pid) FROM pg_stat_activity 
      WHERE state = 'idle' AND query_start < now() - interval '10 minutes';"
   ```

2. **Increase max_connections** (requires restart):
   ```bash
   kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c \
     "ALTER SYSTEM SET max_connections = 200;"
   kubectl delete pod postgres-0 -n matchbook
   ```

### Database Overloaded

1. **Check CPU/memory**:
   ```bash
   kubectl top pod postgres-0 -n matchbook
   ```

2. **Kill long-running queries**:
   ```bash
   kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c \
     "SELECT pg_terminate_backend(pid) FROM pg_stat_activity 
      WHERE state = 'active' AND query_start < now() - interval '5 minutes';"
   ```

3. **Scale database resources**:
   ```bash
   kubectl set resources statefulset/postgres -n matchbook \
     --limits=cpu=4,memory=8Gi
   ```

### Network Issues

1. **Check DNS resolution**:
   ```bash
   kubectl exec -it deployment/api -n matchbook -- nslookup postgres
   ```

2. **Check network policy**:
   ```bash
   kubectl get networkpolicy -n matchbook
   ```

### Database Crashed

1. **Check pod status**:
   ```bash
   kubectl get pods postgres-0 -n matchbook
   kubectl describe pod postgres-0 -n matchbook
   ```

2. **Check persistent volume**:
   ```bash
   kubectl get pvc -n matchbook
   ```

3. **Restart database**:
   ```bash
   kubectl delete pod postgres-0 -n matchbook
   ```

## Prevention

1. **Connection pooling**: Use PgBouncer for connection pooling
2. **Pool sizing**: Size pools appropriately for workload
3. **Query optimization**: Optimize slow queries
4. **Monitoring**: Alert on connection pool usage
5. **Timeouts**: Set appropriate connection timeouts

## Related Alerts

- `DatabaseConnectionErrors` - Critical: > 10 errors/min
- `DatabaseConnectionPoolExhausted` - Critical: no available connections
- `DatabaseSlowQueries` - Warning: high slow query rate

## Escalation

If database issues persist:

1. Check database logs for errors
2. Review recent schema changes
3. Check disk space and I/O
4. Escalate to DBA team
