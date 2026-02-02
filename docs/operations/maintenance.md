# Maintenance Procedures

This document outlines routine maintenance procedures for Matchbook.

## Maintenance Schedule

| Task | Frequency | Window | Duration |
|------|-----------|--------|----------|
| Database vacuum | Weekly | Sunday 02:00 UTC | 1-2 hours |
| Log rotation | Daily | 00:00 UTC | Automatic |
| Certificate renewal | 30 days before expiry | Any | 5 minutes |
| Dependency updates | Weekly | Tuesday | Varies |
| Security patches | As needed | ASAP | Varies |

## Database Maintenance

### Vacuum and Analyze

Run weekly to reclaim space and update statistics:

```bash
# Connect to database
kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook

# Vacuum all tables
VACUUM ANALYZE;

# Vacuum specific large tables
VACUUM ANALYZE events;
VACUUM ANALYZE order_history;
VACUUM ANALYZE trades;

# Full vacuum (requires exclusive lock, use sparingly)
VACUUM FULL trades;
```

### Reindex

Run monthly or when index bloat is detected:

```bash
# Check index bloat
kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c "
SELECT
  schemaname || '.' || relname AS table,
  indexrelname AS index,
  pg_size_pretty(pg_relation_size(indexrelid)) AS index_size
FROM pg_stat_user_indexes
ORDER BY pg_relation_size(indexrelid) DESC
LIMIT 10;"

# Reindex concurrently (no lock)
kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c "
REINDEX INDEX CONCURRENTLY idx_orders_market_status;"

# Reindex entire table
kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c "
REINDEX TABLE CONCURRENTLY orders;"
```

### Data Retention

Clean up old data according to retention policy:

```bash
# Delete events older than 90 days
kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c "
DELETE FROM events WHERE created_at < now() - interval '90 days';"

# Delete old order history
kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c "
DELETE FROM order_history WHERE created_at < now() - interval '180 days';"

# Vacuum after large deletes
kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c "
VACUUM ANALYZE events;
VACUUM ANALYZE order_history;"
```

### Backup Verification

Monthly backup restoration test:

```bash
# List recent backups
aws s3 ls s3://matchbook-backups/postgres/ --recursive | tail -5

# Download backup
aws s3 cp s3://matchbook-backups/postgres/backup_YYYYMMDD.dump /tmp/

# Restore to test database
createdb matchbook_test
pg_restore -d matchbook_test /tmp/backup_YYYYMMDD.dump

# Verify data
psql -d matchbook_test -c "SELECT COUNT(*) FROM trades;"

# Clean up
dropdb matchbook_test
rm /tmp/backup_YYYYMMDD.dump
```

## Log Management

### Log Rotation

Logs are rotated automatically. Manual rotation if needed:

```bash
# Docker
docker logs matchbook-api --since 24h > /tmp/api-logs.txt
docker logs matchbook-api 2>&1 | head -n 0 > /dev/null

# Kubernetes (handled by container runtime)
# Check log sizes
kubectl exec -it <pod> -n matchbook -- ls -lh /var/log/
```

### Log Cleanup

```bash
# Clean up old log files
kubectl exec -it <pod> -n matchbook -- find /var/log -name "*.log" -mtime +7 -delete

# Truncate large logs
kubectl exec -it <pod> -n matchbook -- truncate -s 0 /var/log/app.log
```

### Log Export

Export logs for analysis:

```bash
# Export last 24 hours
kubectl logs deployment/api -n matchbook --since=24h > api-logs-$(date +%Y%m%d).txt

# Export with timestamps
kubectl logs deployment/api -n matchbook --timestamps --since=24h > api-logs-$(date +%Y%m%d).txt
```

## Certificate Management

### Check Certificate Expiry

```bash
# Check ingress certificate
kubectl get certificate -n matchbook
kubectl describe certificate matchbook-tls -n matchbook

# Check expiry date
echo | openssl s_client -connect api.matchbook.io:443 2>/dev/null | openssl x509 -noout -dates
```

### Renew Certificate

If using cert-manager (automatic):

```bash
# Force renewal
kubectl delete certificate matchbook-tls -n matchbook
# cert-manager will automatically create new certificate
```

Manual renewal:

```bash
# Generate new certificate
certbot certonly --dns-cloudflare -d api.matchbook.io

# Update secret
kubectl create secret tls matchbook-tls \
  --cert=/etc/letsencrypt/live/api.matchbook.io/fullchain.pem \
  --key=/etc/letsencrypt/live/api.matchbook.io/privkey.pem \
  -n matchbook --dry-run=client -o yaml | kubectl apply -f -

# Restart ingress controller
kubectl rollout restart deployment/ingress-nginx-controller -n ingress-nginx
```

## Dependency Updates

### Rust Dependencies

```bash
# Check for updates
cargo outdated

# Update Cargo.lock
cargo update

# Update specific dependency
cargo update -p tokio

# Run tests after update
cargo test --all-features
```

### npm Dependencies

```bash
cd ts-sdk

# Check for updates
npm outdated

# Update dependencies
npm update

# Update to latest major versions
npm install <package>@latest

# Run tests
npm test
```

### Docker Base Images

```bash
# Check for newer base images
docker pull rust:1.93-bookworm
docker pull timescale/timescaledb:latest-pg15

# Rebuild images
docker-compose build --no-cache
```

### GitHub Actions

Review Dependabot PRs weekly and merge after CI passes.

## Redis Maintenance

### Memory Management

```bash
# Check memory usage
kubectl exec -it redis-0 -n matchbook -- redis-cli info memory

# Check key count
kubectl exec -it redis-0 -n matchbook -- redis-cli dbsize

# Find large keys
kubectl exec -it redis-0 -n matchbook -- redis-cli --bigkeys
```

### Cache Cleanup

```bash
# Flush specific database
kubectl exec -it redis-0 -n matchbook -- redis-cli SELECT 0
kubectl exec -it redis-0 -n matchbook -- redis-cli FLUSHDB

# Delete keys by pattern
kubectl exec -it redis-0 -n matchbook -- redis-cli KEYS "cache:*" | xargs redis-cli DEL
```

## Kubernetes Maintenance

### Node Maintenance

```bash
# Cordon node (prevent new pods)
kubectl cordon <node-name>

# Drain node (evict pods)
kubectl drain <node-name> --ignore-daemonsets --delete-emptydir-data

# Perform maintenance...

# Uncordon node
kubectl uncordon <node-name>
```

### Resource Cleanup

```bash
# Delete completed jobs
kubectl delete jobs --field-selector status.successful=1 -n matchbook

# Delete failed pods
kubectl delete pods --field-selector status.phase=Failed -n matchbook

# Clean up old ReplicaSets
kubectl delete rs -n matchbook $(kubectl get rs -n matchbook | awk '{if ($2 == 0) print $1}' | tail -n +2)
```

## Monitoring Maintenance

### Prometheus

```bash
# Check storage usage
kubectl exec -it prometheus-0 -n matchbook -- df -h /prometheus

# Clean up old data (if needed)
kubectl exec -it prometheus-0 -n matchbook -- promtool tsdb clean /prometheus
```

### Grafana

```bash
# Backup dashboards
kubectl exec -it grafana-0 -n matchbook -- grafana-cli admin export-dashboard > dashboards-backup.json
```

## Pre-Maintenance Checklist

Before any maintenance:

- [ ] Notify stakeholders of maintenance window
- [ ] Verify backups are current
- [ ] Check current system health
- [ ] Prepare rollback plan
- [ ] Have on-call engineer available

## Post-Maintenance Checklist

After maintenance:

- [ ] Verify all services are healthy
- [ ] Check metrics for anomalies
- [ ] Verify no alerts firing
- [ ] Update maintenance log
- [ ] Notify stakeholders of completion

## Related Documentation

- [Emergency Procedures](./emergency-procedures.md)
- [Incident Response](./incident-response.md)
- [Monitoring Guide](../monitoring.md)
