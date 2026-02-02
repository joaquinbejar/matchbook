# Runbooks

This directory contains operational runbooks for troubleshooting and resolving common issues with the Matchbook system.

## Quick Reference

| Alert | Runbook | Severity |
|-------|---------|----------|
| `IndexerSlotLagHigh` | [indexer-lag.md](./indexer-lag.md) | Warning |
| `IndexerSlotLagCritical` | [indexer-lag.md](./indexer-lag.md) | Critical |
| `IndexerDown` | [service-down.md](./service-down.md) | Critical |
| `APIHighLatency` | [api-latency.md](./api-latency.md) | Warning |
| `APIHighErrorRate` | [api-errors.md](./api-errors.md) | Critical |
| `APIDown` | [service-down.md](./service-down.md) | Critical |
| `CrankHighFailureRate` | [crank-failures.md](./crank-failures.md) | Warning |
| `CrankDown` | [service-down.md](./service-down.md) | Critical |
| `CrankNotMatching` | [crank-not-matching.md](./crank-not-matching.md) | Critical |
| `CrankLowProfitability` | [crank-profitability.md](./crank-profitability.md) | Warning |
| `EventQueueNearFull` | [event-queue-full.md](./event-queue-full.md) | Critical |
| `DatabaseConnectionErrors` | [database-connections.md](./database-connections.md) | Critical |
| `HighMemoryUsage` | [high-memory.md](./high-memory.md) | Warning |
| `HighCPUUsage` | [high-cpu.md](./high-cpu.md) | Warning |
| `DiskSpaceLow` | [disk-space.md](./disk-space.md) | Critical |
| `WebSocketHighConnectionCount` | [websocket-connections.md](./websocket-connections.md) | Warning |

## Runbook Structure

Each runbook follows this structure:

1. **Overview** - What this runbook covers
2. **Symptoms** - How to detect the issue
3. **Impact** - What's affected
4. **Diagnostic Steps** - Commands to investigate
5. **Resolution Steps** - How to fix
6. **Prevention** - How to avoid in future
7. **Related Alerts** - Link to alert rules

## Related Documentation

- [Incident Response](../operations/incident-response.md)
- [Maintenance Procedures](../operations/maintenance.md)
- [Emergency Procedures](../operations/emergency-procedures.md)
- [Monitoring Guide](../monitoring.md)
