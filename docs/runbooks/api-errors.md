# API Errors Runbook

## Overview

This runbook covers diagnosing and resolving high API error rates (5xx errors).

## Symptoms

- `APIHighErrorRate` alert firing (error_rate > 5% for 5m)
- Users reporting failed requests
- Increased 500/502/503 responses in logs
- Client applications showing errors

## Impact

- **User Experience**: Failed trading operations
- **Trading**: Orders not placed/cancelled
- **Trust**: Users lose confidence in platform
- **Revenue**: Direct loss of trading activity

## Diagnostic Steps

### 1. Check Current Error Rate

```bash
# Via Prometheus
curl -s "http://localhost:9092/api/v1/query?query=sum(rate(api_requests_total{status=~\"5..\"}[5m]))/sum(rate(api_requests_total[5m]))" | jq '.data.result[0].value[1]'

# Check errors by status code
curl -s "http://localhost:9092/api/v1/query?query=sum(rate(api_requests_total{status=~\"5..\"}[5m]))by(status)" | jq
```

### 2. Check Error Logs

```bash
# Docker
docker logs matchbook-api --tail 500 | grep -E "(ERROR|PANIC|error|panic)"

# Kubernetes
kubectl logs deployment/api -n matchbook --tail=500 | grep -E "(ERROR|PANIC|error|panic)"
```

### 3. Identify Failing Endpoints

```bash
# Check which endpoints are failing
curl -s "http://localhost:9092/api/v1/query?query=sum(rate(api_requests_total{status=~\"5..\"}[5m]))by(path,method)" | jq
```

### 4. Check Downstream Dependencies

```bash
# Database connectivity
kubectl exec -it postgres-0 -n matchbook -- pg_isready -U matchbook

# Redis connectivity
kubectl exec -it redis-0 -n matchbook -- redis-cli ping

# Solana RPC
curl -s -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' \
  ${SOLANA_RPC_URL}
```

### 5. Check API Health

```bash
# Health endpoint
curl -s http://api:8080/health | jq

# Readiness probe
curl -s http://api:8080/ready | jq
```

## Resolution Steps

### Database Connection Errors

1. **Check database status**:
   ```bash
   kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c "SELECT 1;"
   ```

2. **Check connection pool**:
   ```bash
   curl -s http://api:8080/metrics | grep db_pool
   ```

3. **Restart API if pool is exhausted**:
   ```bash
   kubectl rollout restart deployment/api -n matchbook
   ```

### Redis Connection Errors

1. **Check Redis status**:
   ```bash
   kubectl exec -it redis-0 -n matchbook -- redis-cli ping
   ```

2. **Check Redis memory**:
   ```bash
   kubectl exec -it redis-0 -n matchbook -- redis-cli info memory
   ```

3. **Flush cache if corrupted**:
   ```bash
   kubectl exec -it redis-0 -n matchbook -- redis-cli FLUSHDB
   ```

### Solana RPC Errors

1. **Check RPC health**:
   ```bash
   curl -s -X POST -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' \
     ${SOLANA_RPC_URL}
   ```

2. **Switch to backup RPC**:
   ```bash
   kubectl set env deployment/api -n matchbook \
     SOLANA_RPC_URL=https://backup-rpc.example.com
   ```

### Application Bugs

1. **Check recent deployments**:
   ```bash
   kubectl rollout history deployment/api -n matchbook
   ```

2. **Rollback if recent deployment**:
   ```bash
   kubectl rollout undo deployment/api -n matchbook
   ```

3. **Check for panic/crash loops**:
   ```bash
   kubectl get pods -l app.kubernetes.io/name=api -n matchbook
   kubectl describe pod <pod-name> -n matchbook
   ```

### Resource Exhaustion

1. **Check OOM kills**:
   ```bash
   kubectl get events -n matchbook | grep -i oom
   ```

2. **Increase resources**:
   ```bash
   kubectl set resources deployment/api -n matchbook \
     --limits=cpu=4,memory=4Gi
   ```

## Prevention

1. **Error budgets**: Define acceptable error rates
2. **Circuit breakers**: Implement for downstream dependencies
3. **Graceful degradation**: Return cached data when possible
4. **Health checks**: Comprehensive health endpoints
5. **Canary deployments**: Gradual rollouts to catch issues early

## Related Alerts

- `APIHighErrorRate` - Critical at error_rate > 5%
- `APIDown` - Service not responding
- `DatabaseConnectionErrors` - May cause API errors

## Escalation

If error rate persists:

1. Check if errors are isolated to specific endpoints
2. Review error logs for stack traces
3. Check for infrastructure issues (network, DNS)
4. Escalate to development team for code review
