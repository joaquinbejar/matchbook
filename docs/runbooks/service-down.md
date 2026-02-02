# Service Down Runbook

## Overview

This runbook covers diagnosing and resolving issues when a service is completely unresponsive.

## Symptoms

- `IndexerDown`, `APIDown`, or `CrankDown` alert firing
- Service health endpoint not responding
- No metrics being scraped
- Users unable to access the service

## Impact

- **Indexer Down**: No market data updates, stale order books
- **API Down**: No trading possible, users locked out
- **Crank Down**: Orders not matched, event queue filling up

## Diagnostic Steps

### 1. Check Service Status

```bash
# Docker
docker ps | grep matchbook
docker logs matchbook-<service> --tail 50

# Kubernetes
kubectl get pods -l app.kubernetes.io/name=<service> -n matchbook
kubectl describe pod <pod-name> -n matchbook
```

### 2. Check Health Endpoint

```bash
# Indexer
curl -s http://indexer:9090/health

# API
curl -s http://api:8080/health

# Crank
curl -s http://crank:9091/health
```

### 3. Check Logs for Errors

```bash
# Docker
docker logs matchbook-<service> 2>&1 | grep -E "(ERROR|PANIC|fatal|crash)" | tail -20

# Kubernetes
kubectl logs deployment/<service> -n matchbook --tail=100 | grep -E "(ERROR|PANIC|fatal|crash)"
```

### 4. Check Resource Usage

```bash
# Docker
docker stats matchbook-<service> --no-stream

# Kubernetes
kubectl top pod -l app.kubernetes.io/name=<service> -n matchbook
```

### 5. Check Events

```bash
# Kubernetes events
kubectl get events -n matchbook --sort-by='.lastTimestamp' | tail -20
```

## Resolution Steps

### Pod CrashLoopBackOff

1. **Check crash reason**:
   ```bash
   kubectl describe pod <pod-name> -n matchbook | grep -A 10 "Last State"
   ```

2. **Check logs from previous instance**:
   ```bash
   kubectl logs <pod-name> -n matchbook --previous
   ```

3. **Common causes**:
   - OOM kill: Increase memory limits
   - Config error: Check environment variables
   - Dependency unavailable: Check database/Redis

### OOM Killed

1. **Verify OOM**:
   ```bash
   kubectl get events -n matchbook | grep -i oom
   ```

2. **Increase memory**:
   ```bash
   kubectl set resources deployment/<service> -n matchbook \
     --limits=memory=4Gi --requests=memory=2Gi
   ```

### Configuration Error

1. **Check environment variables**:
   ```bash
   kubectl get deployment/<service> -n matchbook -o yaml | grep -A 50 "env:"
   ```

2. **Check secrets**:
   ```bash
   kubectl get secret matchbook-secrets -n matchbook -o yaml
   ```

3. **Fix and restart**:
   ```bash
   kubectl set env deployment/<service> -n matchbook KEY=value
   ```

### Dependency Unavailable

1. **Check database**:
   ```bash
   kubectl exec -it postgres-0 -n matchbook -- pg_isready
   ```

2. **Check Redis**:
   ```bash
   kubectl exec -it redis-0 -n matchbook -- redis-cli ping
   ```

3. **Check Solana RPC**:
   ```bash
   curl -s ${SOLANA_RPC_URL} -X POST -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
   ```

### Manual Restart

```bash
# Docker
docker restart matchbook-<service>

# Kubernetes
kubectl rollout restart deployment/<service> -n matchbook
```

### Rollback

If service went down after deployment:

```bash
# Check rollout history
kubectl rollout history deployment/<service> -n matchbook

# Rollback to previous version
kubectl rollout undo deployment/<service> -n matchbook

# Rollback to specific revision
kubectl rollout undo deployment/<service> -n matchbook --to-revision=2
```

## Prevention

1. **Health checks**: Ensure liveness and readiness probes are configured
2. **Resource limits**: Set appropriate CPU and memory limits
3. **Graceful shutdown**: Handle SIGTERM properly
4. **Dependency checks**: Verify dependencies before starting
5. **Canary deployments**: Gradual rollouts to catch issues early

## Related Alerts

- `IndexerDown` - Indexer not responding for 2m
- `APIDown` - API not responding for 2m
- `CrankDown` - Crank not responding for 2m

## Escalation

If service cannot be restored:

1. Check infrastructure status (Kubernetes cluster, network)
2. Review recent changes (deployments, config changes)
3. Escalate to platform team
4. Consider failover to backup environment
