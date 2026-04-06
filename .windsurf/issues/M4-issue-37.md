# Issue #37: Alerting Rules for Critical Conditions

## Context
Define alerting rules for critical conditions that require immediate attention, using Prometheus Alertmanager for notification routing and escalation.

## Acceptance Criteria

- [ ] Alert rules for critical conditions:
  - `IndexerLagHigh` — Slot lag > 100 for 5 minutes
  - `IndexerDown` — No metrics for 2 minutes
  - `APIHighLatency` — p99 latency > 500ms for 5 minutes
  - `APIHighErrorRate` — Error rate > 5% for 5 minutes
  - `APIDown` — No metrics for 2 minutes
  - `EventQueueNearFull` — Queue > 80% capacity
  - `CrankNotMatching` — Crossed orders but no matches for 5 minutes
  - `DatabaseConnectionErrors` — Connection failures > 10/min
  - `HighMemoryUsage` — Memory > 90% for 10 minutes
  - `HighCPUUsage` — CPU > 90% for 10 minutes
- [ ] Alertmanager configuration:
  - Routing rules by severity
  - Notification channels (Slack, PagerDuty, email)
  - Silencing and inhibition rules
  - Grouping to reduce noise
- [ ] Severity levels:
  - Critical: Immediate action required (page)
  - Warning: Investigate soon (Slack)
  - Info: Informational (log only)
- [ ] Runbook links in alert annotations
- [ ] Test alerts for verification
- [ ] Documentation for adding new alerts

## Technical Approach

### 1. Update Alert Rules

Update `monitoring/prometheus/alerts/matchbook.yml` with all required alerts:
- Adjust thresholds per acceptance criteria
- Add missing alerts (EventQueueNearFull, CrankNotMatching, DatabaseConnectionErrors)
- Add runbook links to annotations

### 2. Create Alertmanager Configuration

Create `monitoring/alertmanager/alertmanager.yml`:
- Route configuration by severity
- Receiver definitions (Slack, PagerDuty, email)
- Inhibition rules to suppress redundant alerts
- Grouping configuration

### 3. Kubernetes Manifests

Create `monitoring/k8s/alertmanager.yaml`:
- Deployment
- Service
- ConfigMap for configuration
- Secret template for credentials

### 4. Docker Compose

Update `docker-compose.yml`:
- Add Alertmanager service
- Mount configuration

### 5. Documentation

Update `docs/monitoring.md`:
- Alerting section
- How to add new alerts
- Notification channel setup

## Files to Create/Modify

| File | Action |
|------|--------|
| `monitoring/prometheus/alerts/matchbook.yml` | Update with all required alerts |
| `monitoring/alertmanager/alertmanager.yml` | Create Alertmanager config |
| `monitoring/alertmanager/templates/` | Create notification templates |
| `monitoring/k8s/alertmanager.yaml` | Create K8s manifests |
| `docker-compose.yml` | Add Alertmanager service |
| `docs/monitoring.md` | Update documentation |

## Testing

- [ ] Alertmanager starts successfully
- [ ] Alert rules are loaded by Prometheus
- [ ] Test alert fires correctly
- [ ] Notifications route to correct channels
