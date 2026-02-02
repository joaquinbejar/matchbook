# Disk Space Runbook

## Overview

This runbook covers diagnosing and resolving low disk space issues.

## Symptoms

- `DiskSpaceLow` alert firing (< 10% free for 5m)
- Services failing to write logs
- Database write failures
- Container image pulls failing

## Impact

- **Data Loss**: Unable to write new data
- **Service Failure**: Services crash when unable to write
- **Logging**: Lost observability data

## Diagnostic Steps

### 1. Check Disk Usage

```bash
# Kubernetes node
kubectl exec -it <pod-name> -n matchbook -- df -h

# Check PVC usage
kubectl exec -it postgres-0 -n matchbook -- df -h /var/lib/postgresql/data
```

### 2. Find Large Files

```bash
kubectl exec -it <pod-name> -n matchbook -- du -sh /* 2>/dev/null | sort -rh | head -10
```

### 3. Check Log Sizes

```bash
kubectl exec -it <pod-name> -n matchbook -- ls -lh /var/log/
```

## Resolution Steps

### Clean Up Logs

```bash
# Truncate large log files
kubectl exec -it <pod-name> -n matchbook -- truncate -s 0 /var/log/app.log
```

### Clean Up Database

```bash
# Vacuum to reclaim space
kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c "VACUUM FULL;"

# Delete old data
kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c \
  "DELETE FROM events WHERE created_at < now() - interval '30 days';"
```

### Expand PVC

```bash
# Edit PVC to increase size (if storage class supports)
kubectl patch pvc postgres-data -n matchbook -p '{"spec":{"resources":{"requests":{"storage":"100Gi"}}}}'
```

### Clean Up Container Images

```bash
# On Kubernetes nodes
docker system prune -af
```

## Prevention

1. **Log rotation**: Configure log rotation
2. **Data retention**: Implement data retention policies
3. **Monitoring**: Alert before critical levels
4. **Capacity planning**: Plan for growth

## Related Alerts

- `DiskSpaceLow` - Critical at < 10% free
