# High CPU Usage Runbook

## Overview

This runbook covers diagnosing and resolving high CPU usage issues.

## Symptoms

- `HighCPUUsage` alert firing (> 90% for 10m)
- Slow response times
- Request timeouts
- Service degradation

## Impact

- **Performance**: Degraded response times
- **Throughput**: Reduced request handling capacity
- **User Experience**: Slow or unresponsive interface

## Diagnostic Steps

### 1. Check CPU Usage

```bash
# Kubernetes
kubectl top pods -n matchbook --sort-by=cpu

# Docker
docker stats --no-stream
```

### 2. Check Process Details

```bash
kubectl exec -it <pod-name> -n matchbook -- top -bn1 | head -20
```

### 3. Check Request Volume

```bash
curl -s "http://localhost:9092/api/v1/query?query=sum(rate(api_requests_total[5m]))" | jq
```

## Resolution Steps

### Scale Horizontally

```bash
kubectl scale deployment/<service> -n matchbook --replicas=5
```

### Increase CPU Limits

```bash
kubectl set resources deployment/<service> -n matchbook \
  --limits=cpu=4 --requests=cpu=2
```

### Enable Rate Limiting

```bash
kubectl annotate ingress matchbook-ingress -n matchbook \
  nginx.ingress.kubernetes.io/limit-rps="100"
```

### Identify Hot Paths

1. Enable CPU profiling
2. Analyze flame graphs
3. Optimize hot code paths

## Prevention

1. **CPU limits**: Set appropriate limits
2. **Autoscaling**: Configure HPA
3. **Load testing**: Test under expected load
4. **Profiling**: Regular CPU profiling

## Related Alerts

- `HighCPUUsage` - Warning at > 90% for 10m
