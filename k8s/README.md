# Kubernetes Deployment Guide

This directory contains Kubernetes manifests for deploying Matchbook services using Kustomize.

## Directory Structure

```
k8s/
├── base/                    # Base manifests (shared across environments)
│   ├── kustomization.yaml   # Kustomize configuration
│   ├── namespace.yaml       # Namespace definition
│   ├── serviceaccount.yaml  # Service account
│   ├── configmap.yaml       # Application configuration
│   ├── secrets.yaml         # Secrets template
│   ├── ingress.yaml         # Ingress configuration
│   ├── pdb.yaml             # PodDisruptionBudgets
│   ├── indexer/             # Indexer service
│   ├── api/                 # API server
│   └── crank/               # Crank service
├── overlays/
│   ├── dev/                 # Development environment
│   ├── staging/             # Staging environment
│   └── prod/                # Production environment
└── README.md
```

## Prerequisites

- Kubernetes cluster (1.25+)
- kubectl configured
- Kustomize (built into kubectl 1.14+)
- Container images pushed to registry

## Quick Start

### Preview Manifests

```bash
# Preview dev environment
kubectl kustomize k8s/overlays/dev

# Preview staging environment
kubectl kustomize k8s/overlays/staging

# Preview production environment
kubectl kustomize k8s/overlays/prod
```

### Deploy to Development

```bash
# Create namespace and deploy
kubectl apply -k k8s/overlays/dev

# Check status
kubectl get pods -n matchbook-dev
```

### Deploy to Staging

```bash
kubectl apply -k k8s/overlays/staging
kubectl get pods -n matchbook-staging
```

### Deploy to Production

```bash
kubectl apply -k k8s/overlays/prod
kubectl get pods -n matchbook
```

## Configuration

### Environment Variables

Configuration is managed through ConfigMaps and Secrets:

| Variable | Source | Description |
|----------|--------|-------------|
| `RUST_LOG` | ConfigMap | Logging level |
| `API_HOST` | ConfigMap | API bind address |
| `API_PORT` | ConfigMap | API HTTP port |
| `WS_PORT` | ConfigMap | WebSocket port |
| `DATABASE_URL` | Secret | PostgreSQL connection |
| `REDIS_URL` | Secret | Redis connection |
| `SOLANA_RPC_URL` | Secret | Solana RPC endpoint |
| `GEYSER_ENDPOINT` | Secret | Geyser gRPC endpoint |
| `GEYSER_X_TOKEN` | Secret | Geyser auth token |
| `CRANK_KEYPAIR` | Secret | Crank wallet keypair |

### Secrets Management

**Development/Staging**: Edit `secrets.env` in the overlay directory.

**Production**: Use one of:
- [Sealed Secrets](https://github.com/bitnami-labs/sealed-secrets)
- [External Secrets Operator](https://external-secrets.io/)
- Kubernetes native secrets with RBAC

```bash
# Create secret from file
kubectl create secret generic matchbook-secrets \
  --from-env-file=secrets.env \
  -n matchbook \
  --dry-run=client -o yaml | kubectl apply -f -
```

## Services

| Service | Replicas | Ports | Description |
|---------|----------|-------|-------------|
| Indexer | 1 | 9090 (metrics) | Geyser indexer |
| API | 2-10 (HPA) | 8080 (HTTP), 8081 (WS) | REST + WebSocket |
| Crank | 1 | 9091 (metrics) | Order matching |

## Resource Limits

### Development
- Reduced replicas (1 API instance)
- Lower resource limits

### Staging
- Production-like replicas
- Moderate resource limits

### Production
- Full replicas with HPA
- Higher resource limits
- PodDisruptionBudgets enforced

| Service | Environment | CPU Request | Memory Request |
|---------|-------------|-------------|----------------|
| Indexer | dev/staging | 1000m | 2Gi |
| Indexer | prod | 2000m | 4Gi |
| API | dev/staging | 500m | 512Mi |
| API | prod | 1000m | 1Gi |
| Crank | dev/staging | 250m | 256Mi |
| Crank | prod | 500m | 512Mi |

## Health Checks

All services expose health endpoints:

```bash
# Check API health
kubectl exec -it deploy/api -n matchbook -- curl localhost:8080/health

# Check indexer health
kubectl exec -it deploy/indexer -n matchbook -- curl localhost:9090/health
```

## Scaling

### Manual Scaling

```bash
kubectl scale deployment/api --replicas=5 -n matchbook
```

### HPA Configuration

The API service uses HorizontalPodAutoscaler:

```bash
kubectl get hpa -n matchbook
```

## Troubleshooting

### Check Pod Status

```bash
kubectl get pods -n matchbook
kubectl describe pod <pod-name> -n matchbook
```

### View Logs

```bash
kubectl logs -f deploy/api -n matchbook
kubectl logs -f deploy/indexer -n matchbook
```

### Check Events

```bash
kubectl get events -n matchbook --sort-by='.lastTimestamp'
```

### Restart Deployment

```bash
kubectl rollout restart deployment/api -n matchbook
```

### Rollback

```bash
kubectl rollout undo deployment/api -n matchbook
kubectl rollout undo deployment/api --to-revision=2 -n matchbook
```

## Ingress

The ingress is configured for:
- `api.matchbook.example` → API service (HTTP)
- `ws.matchbook.example` → API service (WebSocket)

Update the hostnames in `base/ingress.yaml` or create an overlay patch.

### TLS

TLS is configured with cert-manager. Ensure you have:
1. cert-manager installed
2. ClusterIssuer named `letsencrypt-prod`

## Monitoring

Services expose Prometheus metrics:

| Service | Endpoint |
|---------|----------|
| Indexer | `:9090/metrics` |
| API | `:8080/metrics` |
| Crank | `:9091/metrics` |

## Related Documentation

- [Docker Guide](../docs/docker.md)
- [Deployment Guide](../.internalDoc/07-deployment.md)
- [Operations Guide](../.internalDoc/08-operations.md)
