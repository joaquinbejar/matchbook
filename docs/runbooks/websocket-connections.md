# WebSocket Connections Runbook

## Overview

This runbook covers diagnosing and resolving WebSocket connection issues.

## Symptoms

- `WebSocketHighConnectionCount` alert firing (> 5000 for 5m)
- `WebSocketMessageBacklog` alert firing
- Users reporting disconnections
- Delayed market data updates

## Impact

- **Real-time Data**: Users not receiving live updates
- **Trading**: Delayed price information
- **User Experience**: Frustration with disconnections

## Diagnostic Steps

### 1. Check Connection Count

```bash
curl -s "http://localhost:9092/api/v1/query?query=ws_active_connections" | jq
```

### 2. Check Message Backlog

```bash
curl -s "http://localhost:9092/api/v1/query?query=ws_message_queue_size" | jq
```

### 3. Check API Logs

```bash
kubectl logs deployment/api -n matchbook --tail=100 | grep -i websocket
```

### 4. Check Load Balancer

```bash
kubectl describe ingress matchbook-ingress -n matchbook
```

## Resolution Steps

### High Connection Count

1. **Scale API**:
   ```bash
   kubectl scale deployment/api -n matchbook --replicas=5
   ```

2. **Check for connection leaks**:
   ```bash
   # Monitor connection growth
   watch -n 5 'curl -s http://api:8080/metrics | grep ws_active'
   ```

### Message Backlog

1. **Increase worker threads**:
   ```bash
   kubectl set env deployment/api -n matchbook WS_WORKER_THREADS=8
   ```

2. **Reduce message frequency**:
   ```bash
   kubectl set env deployment/api -n matchbook WS_THROTTLE_MS=100
   ```

### Load Balancer Timeouts

1. **Increase timeouts**:
   ```bash
   kubectl annotate ingress matchbook-ingress -n matchbook \
     nginx.ingress.kubernetes.io/proxy-read-timeout="3600" \
     nginx.ingress.kubernetes.io/proxy-send-timeout="3600"
   ```

## Prevention

1. **Connection limits**: Set per-IP connection limits
2. **Heartbeats**: Implement ping/pong for connection health
3. **Graceful reconnection**: Client-side reconnection logic
4. **Monitoring**: Alert on connection trends

## Related Alerts

- `WebSocketHighConnectionCount` - Warning at > 5000
- `WebSocketMessageBacklog` - Warning at > 10000 messages
