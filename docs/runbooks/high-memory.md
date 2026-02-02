# High Memory Usage Runbook

## Overview

This runbook covers diagnosing and resolving high memory usage issues.

## Symptoms

- `HighMemoryUsage` alert firing (> 90% for 10m)
- Services being OOM killed
- Slow response times
- Pod restarts

## Impact

- **Stability**: Services may crash due to OOM
- **Performance**: Degraded response times
- **Availability**: Service restarts cause downtime

## Diagnostic Steps

### 1. Check Memory Usage

```bash
# Kubernetes
kubectl top pods -n matchbook

# Docker
docker stats --no-stream
```

### 2. Identify High Memory Pods

```bash
kubectl top pods -n matchbook --sort-by=memory
```

### 3. Check for OOM Events

```bash
kubectl get events -n matchbook | grep -i oom
kubectl describe pod <pod-name> -n matchbook | grep -A 5 "Last State"
```

### 4. Check Memory Limits

```bash
kubectl get deployment/<service> -n matchbook -o yaml | grep -A 10 "resources:"
```

## Resolution Steps

### Increase Memory Limits

```bash
kubectl set resources deployment/<service> -n matchbook \
  --limits=memory=4Gi --requests=memory=2Gi
```

### Restart Service to Release Memory

```bash
kubectl rollout restart deployment/<service> -n matchbook
```

### Scale Horizontally

```bash
kubectl scale deployment/<service> -n matchbook --replicas=3
```

### Check for Memory Leaks

1. Enable heap profiling (if supported)
2. Analyze memory growth over time
3. Review recent code changes

## Prevention

1. **Memory limits**: Set appropriate limits
2. **Monitoring**: Alert before critical levels
3. **Load testing**: Test memory under load
4. **Profiling**: Regular memory profiling

## Related Alerts

- `HighMemoryUsage` - Warning at > 90% for 10m
