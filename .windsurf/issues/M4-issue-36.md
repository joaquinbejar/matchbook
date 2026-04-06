# Issue #36: Monitoring setup with Prometheus and Grafana

## Acceptance Criteria

- [ ] Prometheus metrics in all services:
  - Indexer: slot_lag, accounts_processed, events_processed, parse_errors
  - API: request_count, request_latency, active_connections, error_rate
  - Crank: matches_executed, tx_success_rate, fees_earned, priority_fee
  - WebSocket: connections, messages_sent, subscriptions
- [ ] Prometheus configuration:
  - prometheus.yml with scrape configs
  - Service discovery for Kubernetes
  - Recording rules for common aggregations
- [ ] Grafana dashboards:
  - System Overview — All services health at a glance
  - Indexer Dashboard — Lag, throughput, errors
  - API Dashboard — Latency percentiles, error rates, top endpoints
  - Crank Dashboard — Matching performance, profitability
  - Market Dashboard — Volume, spread, book depth per market
- [ ] Dashboard provisioning via ConfigMaps
- [ ] Metrics endpoint security (internal only)
- [ ] Documentation for adding new metrics

## Technical Approach

### Directory Structure

```
monitoring/
├── prometheus/
│   ├── prometheus.yml
│   ├── alerts/
│   │   └── matchbook.yml
│   └── rules/
│       └── recording.yml
├── grafana/
│   ├── provisioning/
│   │   ├── datasources/
│   │   │   └── prometheus.yml
│   │   └── dashboards/
│   │       └── dashboards.yml
│   └── dashboards/
│       ├── overview.json
│       ├── indexer.json
│       ├── api.json
│       ├── crank.json
│       └── market.json
└── k8s/
    ├── prometheus-config.yaml
    ├── grafana-config.yaml
    └── servicemonitor.yaml
```

### Metrics Implementation

Use `prometheus` crate with lazy_static for metric registration:

```rust
use prometheus::{Counter, Histogram, Gauge, register_counter, register_histogram, register_gauge};

lazy_static! {
    pub static ref REQUEST_COUNT: Counter = register_counter!(
        "api_requests_total",
        "Total number of API requests"
    ).unwrap();
    
    pub static ref REQUEST_LATENCY: Histogram = register_histogram!(
        "api_request_duration_seconds",
        "Request latency in seconds"
    ).unwrap();
}
```

### Metric Types

| Type | Use Case | Example |
|------|----------|---------|
| Counter | Cumulative events | requests_total, errors_total |
| Gauge | Current state | active_connections, slot_lag |
| Histogram | Latency distribution | request_duration_seconds |

## Files to Create

### Monitoring Configuration
- `monitoring/prometheus/prometheus.yml`
- `monitoring/prometheus/alerts/matchbook.yml`
- `monitoring/prometheus/rules/recording.yml`
- `monitoring/grafana/provisioning/datasources/prometheus.yml`
- `monitoring/grafana/provisioning/dashboards/dashboards.yml`
- `monitoring/grafana/dashboards/*.json`

### Kubernetes Manifests
- `monitoring/k8s/prometheus-config.yaml`
- `monitoring/k8s/grafana-config.yaml`
- `monitoring/k8s/servicemonitor.yaml`

### Documentation
- `docs/monitoring.md`

## Implementation Steps

1. Create monitoring directory structure
2. Create Prometheus configuration
3. Create alert rules
4. Create recording rules
5. Create Grafana datasource provisioning
6. Create Grafana dashboards (JSON)
7. Create Kubernetes ConfigMaps
8. Add documentation
9. Update docker-compose with monitoring services

## Testing

- [ ] Prometheus config validates
- [ ] Grafana dashboards load correctly
- [ ] Kubernetes manifests apply successfully
