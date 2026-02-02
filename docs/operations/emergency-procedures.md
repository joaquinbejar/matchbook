# Emergency Procedures

This document outlines emergency procedures for critical situations.

## Emergency Contacts

| Role | Contact | When to Contact |
|------|---------|-----------------|
| On-call Engineer | PagerDuty | Any P1/P2 incident |
| Engineering Manager | @eng-manager | P1 not resolved in 1 hour |
| Security Team | @security | Security incidents |
| Business Operations | @biz-ops | Customer communication needed |

## Service Restart Procedures

### Restart Single Service

```bash
# Docker
docker restart matchbook-<service>

# Kubernetes
kubectl rollout restart deployment/<service> -n matchbook

# Verify restart
kubectl get pods -l app.kubernetes.io/name=<service> -n matchbook -w
```

### Restart All Services

```bash
# Docker
docker-compose restart

# Kubernetes (ordered)
kubectl rollout restart deployment/indexer -n matchbook
kubectl rollout status deployment/indexer -n matchbook
kubectl rollout restart deployment/api -n matchbook
kubectl rollout status deployment/api -n matchbook
kubectl rollout restart deployment/crank -n matchbook
kubectl rollout status deployment/crank -n matchbook
```

### Emergency Service Shutdown

If a critical bug is discovered:

```bash
# Scale down all services
kubectl scale deployment --all --replicas=0 -n matchbook

# Or delete deployments entirely
kubectl delete deployment --all -n matchbook
```

## Rollback Procedures

### Application Rollback

```bash
# Check rollout history
kubectl rollout history deployment/<service> -n matchbook

# Rollback to previous version
kubectl rollout undo deployment/<service> -n matchbook

# Rollback to specific revision
kubectl rollout undo deployment/<service> -n matchbook --to-revision=<N>

# Verify rollback
kubectl rollout status deployment/<service> -n matchbook
```

### Docker Image Rollback

```bash
# List available tags
docker images ghcr.io/joaquinbejar/matchbook-api

# Update to specific version
kubectl set image deployment/api api=ghcr.io/joaquinbejar/matchbook-api:v1.2.3 -n matchbook
```

### Database Rollback

If a migration caused issues:

```bash
# Check migration status
kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c "SELECT * FROM schema_migrations ORDER BY version DESC LIMIT 5;"

# Rollback last migration (if supported)
kubectl exec -it deployment/api -n matchbook -- ./matchbook migrate rollback

# Manual rollback (use with caution)
kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c "
BEGIN;
-- Reverse migration SQL here
DELETE FROM schema_migrations WHERE version = 'YYYYMMDDHHMMSS';
COMMIT;"
```

## Market Emergency Procedures

### Pause Market

If critical bug in matching logic:

```bash
# Set market to cancel-only mode
matchbook-cli set-market-state \
  --market <MARKET_ADDRESS> \
  --state cancel-only \
  --authority /path/to/authority.json

# Verify state
matchbook-cli market-info --market <MARKET_ADDRESS>
```

### Resume Market

After fix is deployed:

```bash
# Restore market to active
matchbook-cli set-market-state \
  --market <MARKET_ADDRESS> \
  --state active \
  --authority /path/to/authority.json
```

### Emergency Market Closure

For severe issues:

```bash
# Close market entirely
matchbook-cli close-market \
  --market <MARKET_ADDRESS> \
  --authority /path/to/authority.json
```

## Data Recovery Procedures

### Database Recovery

#### From Backup

```bash
# List available backups
aws s3 ls s3://matchbook-backups/postgres/ --recursive | tail -10

# Download backup
aws s3 cp s3://matchbook-backups/postgres/backup_YYYYMMDD.dump /tmp/

# Stop services
kubectl scale deployment --all --replicas=0 -n matchbook

# Restore database
kubectl exec -it postgres-0 -n matchbook -- pg_restore \
  -U matchbook -d matchbook -c /tmp/backup_YYYYMMDD.dump

# Restart services
kubectl scale deployment/indexer --replicas=1 -n matchbook
kubectl scale deployment/api --replicas=2 -n matchbook
kubectl scale deployment/crank --replicas=1 -n matchbook
```

#### Point-in-Time Recovery

If using continuous archiving:

```bash
# Stop database
kubectl scale statefulset/postgres --replicas=0 -n matchbook

# Restore to specific time
kubectl exec -it postgres-0 -n matchbook -- pg_restore \
  --target-time="2024-01-15 10:30:00" \
  -d matchbook

# Start database
kubectl scale statefulset/postgres --replicas=1 -n matchbook
```

### Redis Recovery

```bash
# Check if AOF is enabled
kubectl exec -it redis-0 -n matchbook -- redis-cli config get appendonly

# Restore from AOF
kubectl exec -it redis-0 -n matchbook -- redis-cli BGREWRITEAOF

# If Redis is corrupted, flush and let services repopulate
kubectl exec -it redis-0 -n matchbook -- redis-cli FLUSHALL
```

### On-Chain Data Recovery

On-chain data is immutable. Recovery involves:

1. Re-indexing from Solana:
   ```bash
   # Reset indexer state
   kubectl exec -it postgres-0 -n matchbook -- psql -U matchbook -c "
   TRUNCATE events, order_history, trades CASCADE;
   UPDATE indexer_state SET last_slot = 0;"
   
   # Restart indexer to re-sync
   kubectl rollout restart deployment/indexer -n matchbook
   ```

## Infrastructure Emergency

### Kubernetes Cluster Issues

```bash
# Check cluster health
kubectl get nodes
kubectl get componentstatuses

# Check system pods
kubectl get pods -n kube-system

# If node is unhealthy, cordon and drain
kubectl cordon <node>
kubectl drain <node> --ignore-daemonsets --delete-emptydir-data
```

### Network Issues

```bash
# Check DNS
kubectl run -it --rm debug --image=busybox --restart=Never -- nslookup postgres

# Check connectivity
kubectl run -it --rm debug --image=busybox --restart=Never -- wget -qO- http://api:8080/health

# Check network policies
kubectl get networkpolicy -n matchbook
```

### Storage Issues

```bash
# Check PVC status
kubectl get pvc -n matchbook

# Check PV status
kubectl get pv

# If PVC is stuck, check events
kubectl describe pvc <pvc-name> -n matchbook
```

## Security Incidents

### Suspected Breach

1. **Contain**
   ```bash
   # Isolate affected services
   kubectl scale deployment/<service> --replicas=0 -n matchbook
   
   # Revoke API keys
   kubectl delete secret api-keys -n matchbook
   ```

2. **Preserve Evidence**
   ```bash
   # Export logs
   kubectl logs deployment/<service> -n matchbook --since=24h > incident-logs.txt
   
   # Snapshot database
   pg_dump -U matchbook matchbook > incident-snapshot.sql
   ```

3. **Notify**
   - Security team
   - Legal team
   - Affected users (if required)

### Compromised Credentials

```bash
# Rotate database password
kubectl create secret generic postgres-secret \
  --from-literal=password=$(openssl rand -hex 32) \
  -n matchbook --dry-run=client -o yaml | kubectl apply -f -

# Rotate API keys
kubectl create secret generic api-keys \
  --from-literal=key=$(openssl rand -hex 32) \
  -n matchbook --dry-run=client -o yaml | kubectl apply -f -

# Restart services to pick up new credentials
kubectl rollout restart deployment --all -n matchbook
```

### Compromised Crank Wallet

```bash
# Stop crank immediately
kubectl scale deployment/crank --replicas=0 -n matchbook

# Generate new keypair
solana-keygen new -o /tmp/new-crank.json

# Transfer remaining funds
solana transfer $(solana-keygen pubkey /tmp/new-crank.json) ALL \
  --from <OLD_KEYPAIR> --allow-unfunded-recipient

# Update secret
kubectl create secret generic crank-keypair \
  --from-file=keypair=/tmp/new-crank.json \
  -n matchbook --dry-run=client -o yaml | kubectl apply -f -

# Restart crank
kubectl scale deployment/crank --replicas=1 -n matchbook

# Clean up
rm /tmp/new-crank.json
```

## Communication During Emergency

### Status Page Updates

```bash
# Update status page (example using Statuspage API)
curl -X POST https://api.statuspage.io/v1/pages/<PAGE_ID>/incidents \
  -H "Authorization: OAuth <TOKEN>" \
  -H "Content-Type: application/json" \
  -d '{
    "incident": {
      "name": "Service Degradation",
      "status": "investigating",
      "body": "We are investigating reports of service issues.",
      "component_ids": ["<COMPONENT_ID>"],
      "component_status": "degraded_performance"
    }
  }'
```

### Slack Notification

```bash
curl -X POST https://hooks.slack.com/services/<WEBHOOK> \
  -H "Content-Type: application/json" \
  -d '{
    "text": "🚨 EMERGENCY: [Description]. IC: @oncall. Channel: #incident-YYYYMMDD"
  }'
```

## Post-Emergency Checklist

- [ ] All services restored and healthy
- [ ] Metrics showing normal operation
- [ ] No alerts firing
- [ ] Status page updated to resolved
- [ ] Stakeholders notified
- [ ] Incident documented
- [ ] Post-mortem scheduled
- [ ] Evidence preserved for analysis

## Related Documentation

- [Incident Response](./incident-response.md)
- [Maintenance Procedures](./maintenance.md)
- [Runbooks](../runbooks/README.md)
