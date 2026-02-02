# Incident Response Guide

This document outlines the incident response process for Matchbook.

## Severity Levels

| Severity | Description | Response Time | Examples |
|----------|-------------|---------------|----------|
| **P1 - Critical** | Complete outage, data loss risk | 15 minutes | All trading halted, data corruption |
| **P2 - High** | Major feature unavailable | 30 minutes | Order matching delayed > 1 min |
| **P3 - Medium** | Minor feature degraded | 2 hours | Increased latency, partial data |
| **P4 - Low** | Cosmetic or minor issue | Next business day | UI glitch, non-critical bug |

## Incident Workflow

```
┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐
│ Detect  │───▶│ Triage  │───▶│ Respond │───▶│ Resolve │───▶│ Review  │
└─────────┘    └─────────┘    └─────────┘    └─────────┘    └─────────┘
     │              │              │              │              │
     ▼              ▼              ▼              ▼              ▼
  Alert or      Assign         Mitigate       Fix root       Document
  report        severity       impact         cause          learnings
  received      & owner
```

## Roles

| Role | Responsibilities |
|------|------------------|
| **Incident Commander (IC)** | Coordinates response, makes decisions, communicates status |
| **Technical Lead** | Investigates root cause, implements fixes |
| **Communications Lead** | Updates status page, notifies stakeholders |
| **Scribe** | Documents timeline, actions taken, decisions |

## Response Procedures

### P1 - Critical Incident

1. **Acknowledge** (within 5 minutes)
   - Acknowledge alert in PagerDuty/Slack
   - Join incident channel: `#incident-YYYYMMDD`

2. **Assemble Team** (within 10 minutes)
   - Page Incident Commander
   - Page Technical Lead
   - Notify Communications Lead

3. **Assess Impact**
   - What services are affected?
   - How many users impacted?
   - Is there data loss risk?

4. **Communicate**
   - Post initial status to status page
   - Notify key stakeholders
   - Update every 15 minutes

5. **Mitigate**
   - Can we rollback?
   - Can we failover?
   - Can we scale?

6. **Resolve**
   - Implement fix
   - Verify resolution
   - Monitor for recurrence

7. **Close**
   - Update status page
   - Notify stakeholders
   - Schedule post-mortem

### P2 - High Severity

1. **Acknowledge** (within 15 minutes)
2. **Assess** scope and impact
3. **Communicate** via Slack
4. **Investigate** root cause
5. **Implement** fix or workaround
6. **Document** in incident log

### P3/P4 - Medium/Low Severity

1. **Acknowledge** during business hours
2. **Create** ticket for tracking
3. **Investigate** when capacity allows
4. **Fix** in normal release cycle

## Communication Templates

### Initial Status Update

```
🔴 INCIDENT: [Brief description]

Status: Investigating
Impact: [Description of user impact]
Started: [Time]

We are aware of the issue and actively investigating.
Updates will be posted every [15/30] minutes.
```

### Progress Update

```
🟡 UPDATE: [Brief description]

Status: Identified / Mitigating
Impact: [Current impact]
Duration: [Time since start]

[What we've learned]
[What we're doing]

Next update in [X] minutes.
```

### Resolution Update

```
🟢 RESOLVED: [Brief description]

Status: Resolved
Duration: [Total duration]
Impact: [Summary of impact]

Root cause: [Brief explanation]
Resolution: [What was done]

A full post-mortem will be conducted.
```

## Escalation Matrix

| Condition | Escalate To |
|-----------|-------------|
| P1 not acknowledged in 15 min | Engineering Manager |
| P1 not mitigated in 1 hour | VP Engineering |
| Data breach suspected | Security Team + Legal |
| Financial impact > $X | Business Operations |

## Post-Incident Review

### Timeline

- **P1**: Post-mortem within 48 hours
- **P2**: Post-mortem within 1 week
- **P3/P4**: Optional, as needed

### Post-Mortem Template

```markdown
# Incident Post-Mortem: [Title]

## Summary
- **Date**: YYYY-MM-DD
- **Duration**: X hours Y minutes
- **Severity**: P1/P2/P3/P4
- **Impact**: [User/business impact]

## Timeline
| Time (UTC) | Event |
|------------|-------|
| HH:MM | Alert fired |
| HH:MM | IC assigned |
| HH:MM | Root cause identified |
| HH:MM | Fix deployed |
| HH:MM | Incident resolved |

## Root Cause
[Detailed explanation of what caused the incident]

## Resolution
[What was done to resolve the incident]

## Impact
- Users affected: X
- Revenue impact: $X
- Data loss: Yes/No

## Lessons Learned
### What went well
- [Item 1]
- [Item 2]

### What could be improved
- [Item 1]
- [Item 2]

## Action Items
| Action | Owner | Due Date |
|--------|-------|----------|
| [Action 1] | @person | YYYY-MM-DD |
| [Action 2] | @person | YYYY-MM-DD |
```

## Tools and Access

### Incident Management
- **PagerDuty**: Alert routing and escalation
- **Slack**: `#incidents` channel
- **Status Page**: https://status.matchbook.io

### Diagnostic Tools
- **Grafana**: http://grafana:3000
- **Prometheus**: http://prometheus:9092
- **Alertmanager**: http://alertmanager:9093

### Access Requirements
- Kubernetes cluster access
- Database read access
- Log aggregation access
- Solana RPC access

## Contacts

| Role | Contact |
|------|---------|
| On-call Engineer | PagerDuty rotation |
| Engineering Manager | @eng-manager |
| Security Team | @security |
| Business Operations | @biz-ops |

## Related Documentation

- [Runbooks](../runbooks/README.md)
- [Emergency Procedures](./emergency-procedures.md)
- [Monitoring Guide](../monitoring.md)
