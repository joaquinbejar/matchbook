# Monitoring Guide

This guide explains the monitoring infrastructure for Matchbook using Prometheus and Grafana.

## Overview

Matchbook uses:
- **Prometheus** for metrics collection and alerting
- **Grafana** for visualization and dashboards

## Quick Start (Local Development)

```bash
# Start all services including monitoring
docker-compose -f Docker/docker-compose.yml up -d

# Access Grafana
open http://localhost:3000
# Default credentials: admin/admin

# Access Prometheus
open http://localhost:9092
```

## Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Indexer   │     │     API     │     │    Crank    │
│   :9090     │     │   :8080     │     │   :9091     │
└──────┬──────┘     └──────┬──────┘     └──────┬──────┘
       │                   │                   │
       │    /metrics       │    /metrics       │    /metrics
       └───────────────────┼───────────────────┘
                           │
                    ┌──────▼──────┐
                    │  Prometheus │
                    │    :9092    │
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              │                         │
       ┌──────▼──────┐          ┌───────▼───────┐
       │   Grafana   │          │ Alertmanager  │
       │    :3000    │          │    :9093      │
       └─────────────┘          └───────┬───────┘
                                        │
                          ┌─────────────┼─────────────┐
                          │             │             │
                     ┌────▼────┐  ┌─────▼─────┐  ┌────▼────┐
                     │  Slack  │  │ PagerDuty │  │  Email  │
                     └─────────┘  └───────────┘  └─────────┘
```

## Metrics

### Indexer Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `indexer_slot_lag` | Gauge | Slots behind the tip |
| `indexer_events_processed_total` | Counter | Total events processed |
| `indexer_accounts_processed_total` | Counter | Total accounts processed |
| `indexer_parse_errors_total` | Counter | Parse errors by type |

### API Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `api_requests_total` | Counter | Total requests by method, path, status |
| `api_request_duration_seconds` | Histogram | Request latency |
| `api_active_connections` | Gauge | Current HTTP connections |
| `ws_active_connections` | Gauge | Current WebSocket connections |
| `ws_messages_sent_total` | Counter | WebSocket messages sent |
| `ws_subscriptions` | Gauge | Active subscriptions |

### Crank Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `crank_matches_executed_total` | Counter | Total matches executed |
| `crank_transactions_total` | Counter | Transactions by status |
| `crank_profit_lamports` | Gauge | Current profit |
| `crank_priority_fee_lamports` | Gauge | Current priority fee |
| `crank_fees_paid_lamports` | Counter | Total fees paid |

### Market Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `market_volume_total` | Counter | Trading volume by market |
| `market_trades_total` | Counter | Trade count by market |
| `market_order_count` | Gauge | Active orders by market/side |
| `market_spread_bps` | Gauge | Spread in basis points |

## Dashboards

### Overview Dashboard
- Service health status
- Slot lag
- Request rate
- Latency percentiles
- Active connections
- Crank matches

### Indexer Dashboard
- Slot lag over time
- Processing throughput
- Parse errors by type

### API Dashboard
- Request rate by endpoint
- Latency percentiles (p50, p95, p99)
- Error rate
- Requests by status code
- Active connections

### Crank Dashboard
- Matches over time
- Transaction success rate
- Profitability
- Transactions by status

### Market Dashboard
- 24h volume
- 24h trades
- Spread
- Order book depth
- Volume over time

## Alerting

Matchbook uses Prometheus Alertmanager for alert routing and notification.

### Access Alertmanager

```bash
# Local development
open http://localhost:9093

# View active alerts
curl http://localhost:9093/api/v2/alerts
```

### Severity Levels

| Severity | Response | Notification |
|----------|----------|--------------|
| **Critical** | Immediate action required | PagerDuty + Slack |
| **Warning** | Investigate soon | Slack |
| **Info** | Informational | Log only |

### Alert Rules

#### Critical Alerts

| Alert | Condition | Description |
|-------|-----------|-------------|
| `IndexerDown` | `up == 0` for 2m | Indexer not responding |
| `IndexerSlotLagCritical` | `slot_lag > 500` for 2m | Indexer severely behind |
| `APIDown` | `up == 0` for 2m | API not responding |
| `APIHighErrorRate` | `error_rate > 5%` for 5m | High API error rate |
| `CrankDown` | `up == 0` for 2m | Crank not responding |
| `CrankNotMatching` | Crossed orders, no matches for 5m | Crank not executing matches |
| `EventQueueNearFull` | Queue > 80% capacity for 5m | Event queue needs draining |
| `DatabaseConnectionErrors` | > 10 errors/min for 2m | Database connection issues |
| `DiskSpaceLow` | < 10% free for 5m | Disk space critical |

#### Warning Alerts

| Alert | Condition | Description |
|-------|-----------|-------------|
| `IndexerSlotLagHigh` | `slot_lag > 100` for 5m | Indexer falling behind |
| `APIHighLatency` | `p99 > 500ms` for 5m | High API latency |
| `CrankHighFailureRate` | `failure_rate > 10%` for 5m | Crank transactions failing |
| `CrankLowProfitability` | `profit < 0` for 10m | Crank not profitable |
| `HighMemoryUsage` | `> 90%` for 10m | High memory usage |
| `HighCPUUsage` | `> 90%` for 10m | High CPU usage |
| `WebSocketHighConnectionCount` | `> 5000` for 5m | Many WebSocket connections |

### Alertmanager Configuration

The Alertmanager configuration is in `monitoring/alertmanager/alertmanager.yml`:

```yaml
route:
  receiver: 'slack-warnings'
  group_by: ['alertname', 'service', 'severity']
  routes:
    - match:
        severity: critical
      receiver: 'pagerduty-critical'
    - match:
        severity: warning
      receiver: 'slack-warnings'

receivers:
  - name: 'slack-warnings'
    slack_configs:
      - channel: '#matchbook-alerts'
  - name: 'pagerduty-critical'
    pagerduty_configs:
      - service_key: '<YOUR_KEY>'
```

### Setting Up Notification Channels

#### Slack

1. Create a Slack webhook: https://api.slack.com/messaging/webhooks
2. Set the webhook URL in `alertmanager.yml`:
   ```yaml
   global:
     slack_api_url: 'https://hooks.slack.com/services/YOUR/WEBHOOK/URL'
   ```

#### PagerDuty

1. Create a PagerDuty service and get the integration key
2. Set the service key in `alertmanager.yml`:
   ```yaml
   receivers:
     - name: 'pagerduty-critical'
       pagerduty_configs:
         - service_key: 'YOUR_SERVICE_KEY'
   ```

#### Email

1. Configure SMTP settings in `alertmanager.yml`:
   ```yaml
   global:
     smtp_smarthost: 'smtp.example.com:587'
     smtp_from: 'alertmanager@matchbook.io'
     smtp_auth_username: 'user'
     smtp_auth_password: 'password'
   ```

### Adding New Alerts

1. Add the alert rule to `monitoring/prometheus/alerts/matchbook.yml`:
   ```yaml
   - alert: MyNewAlert
     expr: my_metric > threshold
     for: 5m
     labels:
       severity: warning
       service: my-service
     annotations:
       summary: "Alert summary"
       description: "Detailed description with {{ $value }}"
       runbook_url: "https://github.com/joaquinbejar/matchbook/blob/main/docs/runbooks/my-alert.md"
   ```

2. Reload Prometheus:
   ```bash
   curl -X POST http://localhost:9092/-/reload
   ```

### Silencing Alerts

To temporarily silence an alert:

```bash
# Via Alertmanager UI
open http://localhost:9093/#/silences

# Via API
curl -X POST http://localhost:9093/api/v2/silences \
  -H "Content-Type: application/json" \
  -d '{
    "matchers": [{"name": "alertname", "value": "MyAlert", "isRegex": false}],
    "startsAt": "2024-01-01T00:00:00Z",
    "endsAt": "2024-01-01T01:00:00Z",
    "createdBy": "admin",
    "comment": "Maintenance window"
  }'
```

### Inhibition Rules

Alerts are automatically suppressed when related critical alerts fire:

- `IndexerDown` suppresses `IndexerSlotLagHigh`
- `APIDown` suppresses `APIHighLatency`, `APIHighErrorRate`
- `CrankDown` suppresses `CrankHighFailureRate`
- Critical alerts suppress warning alerts for the same service

## Adding New Metrics

### Rust Implementation

```rust
use prometheus::{Counter, Histogram, Gauge, register_counter, register_histogram, register_gauge};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref MY_COUNTER: Counter = register_counter!(
        "my_counter_total",
        "Description of my counter"
    ).unwrap();
    
    pub static ref MY_HISTOGRAM: Histogram = register_histogram!(
        "my_duration_seconds",
        "Description of my histogram"
    ).unwrap();
    
    pub static ref MY_GAUGE: Gauge = register_gauge!(
        "my_gauge",
        "Description of my gauge"
    ).unwrap();
}

// Usage
MY_COUNTER.inc();
MY_HISTOGRAM.observe(duration.as_secs_f64());
MY_GAUGE.set(value);
```

### Metric Naming Conventions

- Use snake_case
- Include unit suffix: `_seconds`, `_bytes`, `_total`
- Counters should end with `_total`
- Use labels for dimensions: `{method="GET", path="/v1/markets"}`

## Prometheus Configuration

### Scrape Configuration

```yaml
scrape_configs:
  - job_name: my-service
    static_configs:
      - targets:
          - my-service:9090
    relabel_configs:
      - source_labels: [__address__]
        target_label: instance
        regex: (.+):.*
        replacement: $1
```

### Recording Rules

Pre-compute expensive queries:

```yaml
groups:
  - name: my-rules
    rules:
      - record: my:metric:5m
        expr: sum(rate(my_counter_total[5m]))
```

### Alert Rules

```yaml
groups:
  - name: my-alerts
    rules:
      - alert: MyAlert
        expr: my_metric > 100
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "My alert fired"
          description: "Value is {{ $value }}"
```

## Kubernetes Deployment

### Using ServiceMonitor (Prometheus Operator)

```yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: my-service
spec:
  selector:
    matchLabels:
      app: my-service
  endpoints:
    - port: metrics
      interval: 15s
```

### Pod Annotations

```yaml
metadata:
  annotations:
    prometheus.io/scrape: "true"
    prometheus.io/port: "9090"
    prometheus.io/path: "/metrics"
```

## Troubleshooting

### Prometheus Not Scraping

1. Check target status in Prometheus UI (`/targets`)
2. Verify network connectivity
3. Check service annotations/labels

### Missing Metrics

1. Verify metrics endpoint: `curl http://service:port/metrics`
2. Check metric registration in code
3. Verify scrape config matches service

### Dashboard Not Loading

1. Check Grafana datasource configuration
2. Verify Prometheus is accessible from Grafana
3. Check dashboard JSON syntax

## Related Documentation

- [Docker Guide](./docker.md)
- [Kubernetes Guide](../k8s/README.md)
- [Operations Guide](../.internalDoc/08-operations.md)
