# Issue #38: Runbooks and Operational Documentation

## Context
Create comprehensive operational documentation including troubleshooting runbooks, incident response procedures, maintenance guides, and emergency procedures.

## Acceptance Criteria

- [ ] Troubleshooting runbooks:
  - `runbooks/indexer-lag.md` — Diagnose and resolve indexer lag
  - `runbooks/api-latency.md` — Investigate high API latency
  - `runbooks/event-queue-full.md` — Handle full event queue
  - `runbooks/websocket-disconnections.md` — Debug WS issues
  - `runbooks/crank-not-profitable.md` — Crank profitability issues
  - `runbooks/database-issues.md` — Database connection/performance
  - `runbooks/rpc-failures.md` — RPC endpoint problems
- [ ] Incident response documentation:
  - Severity level definitions (P1-P4)
  - Escalation procedures
  - Communication templates
  - Post-incident review process
- [ ] Maintenance procedures:
  - Database maintenance (vacuum, reindex)
  - Log rotation and cleanup
  - Certificate renewal
  - Dependency updates
- [ ] Emergency procedures:
  - Service restart procedures
  - Rollback procedures
  - Data recovery steps
  - Emergency contacts
- [ ] Each runbook includes:
  - Symptoms and detection
  - Diagnostic commands
  - Resolution steps
  - Prevention measures

## Technical Approach

### Directory Structure

```
docs/runbooks/
├── README.md                      # Index and overview
├── indexer-lag.md
├── api-latency.md
├── api-errors.md
├── event-queue-full.md
├── websocket-disconnections.md
├── crank-not-profitable.md
├── crank-failures.md
├── crank-not-matching.md
├── database-issues.md
├── rpc-failures.md
├── high-memory.md
├── high-cpu.md
├── disk-space.md
└── service-down.md

docs/operations/
├── incident-response.md
├── maintenance.md
└── emergency-procedures.md
```

### Runbook Template

Each runbook follows this structure:
1. **Overview** - What this runbook covers
2. **Symptoms** - How to detect the issue
3. **Impact** - What's affected
4. **Diagnostic Steps** - Commands to investigate
5. **Resolution Steps** - How to fix
6. **Prevention** - How to avoid in future
7. **Related Alerts** - Link to alert rules

## Files to Create

| File | Description |
|------|-------------|
| `docs/runbooks/README.md` | Index of all runbooks |
| `docs/runbooks/indexer-lag.md` | Indexer lag troubleshooting |
| `docs/runbooks/api-latency.md` | API latency troubleshooting |
| `docs/runbooks/api-errors.md` | API error rate troubleshooting |
| `docs/runbooks/event-queue-full.md` | Event queue troubleshooting |
| `docs/runbooks/websocket-*.md` | WebSocket troubleshooting |
| `docs/runbooks/crank-*.md` | Crank troubleshooting |
| `docs/runbooks/database-*.md` | Database troubleshooting |
| `docs/runbooks/service-down.md` | Service down troubleshooting |
| `docs/operations/incident-response.md` | Incident response guide |
| `docs/operations/maintenance.md` | Maintenance procedures |
| `docs/operations/emergency-procedures.md` | Emergency procedures |

## Testing

- [ ] All runbook links work
- [ ] Commands are valid and tested
- [ ] Documentation renders correctly
