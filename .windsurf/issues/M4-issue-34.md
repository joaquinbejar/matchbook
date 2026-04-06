# Issue #34: Kubernetes manifests for all services

## Acceptance Criteria

- [ ] Namespace configuration (`matchbook` namespace)
- [ ] Deployments for each service:
  - Indexer (1 replica, stateful considerations)
  - API server (2+ replicas, HPA)
  - Crank service (1 replica per market)
- [ ] Services:
  - ClusterIP for internal communication
  - LoadBalancer or NodePort for API
- [ ] ConfigMaps:
  - Application configuration
  - Environment-specific settings
- [ ] Secrets:
  - Database credentials
  - RPC endpoints
  - API keys
- [ ] Ingress:
  - TLS termination
  - Path-based routing
  - Rate limiting annotations
- [ ] Resource limits and requests
- [ ] Liveness and readiness probes
- [ ] PodDisruptionBudgets for availability
- [ ] Kustomize overlays for dev/staging/prod
- [ ] Documentation for deployment

## Technical Approach

### Directory Structure

```
k8s/
├── base/                    # Base manifests
│   ├── kustomization.yaml
│   ├── namespace.yaml
│   ├── configmap.yaml
│   ├── secrets.yaml         # Template (values in overlays)
│   ├── indexer/
│   │   ├── deployment.yaml
│   │   └── service.yaml
│   ├── api/
│   │   ├── deployment.yaml
│   │   ├── service.yaml
│   │   └── hpa.yaml
│   ├── crank/
│   │   ├── deployment.yaml
│   │   └── service.yaml
│   ├── ingress.yaml
│   └── pdb.yaml
├── overlays/
│   ├── dev/
│   │   ├── kustomization.yaml
│   │   ├── configmap-patch.yaml
│   │   └── secrets.yaml
│   ├── staging/
│   │   ├── kustomization.yaml
│   │   ├── configmap-patch.yaml
│   │   └── secrets.yaml
│   └── prod/
│       ├── kustomization.yaml
│       ├── configmap-patch.yaml
│       ├── secrets.yaml
│       └── replicas-patch.yaml
└── README.md
```

### Key Design Decisions

1. **Kustomize over Helm**: Simpler, native kubectl support, easier to understand
2. **Base + Overlays**: Common config in base, environment-specific in overlays
3. **Secrets as templates**: Real values injected via CI/CD or sealed-secrets
4. **HPA for API**: Auto-scale based on CPU/memory
5. **PDB for availability**: Ensure minimum replicas during updates

### Resource Allocations

| Service | CPU Request | CPU Limit | Memory Request | Memory Limit |
|---------|-------------|-----------|----------------|--------------|
| Indexer | 1000m | 2000m | 2Gi | 4Gi |
| API | 500m | 1000m | 512Mi | 1Gi |
| Crank | 250m | 500m | 256Mi | 512Mi |

### Health Probes

All services expose:
- `/health` - Liveness probe
- `/ready` - Readiness probe

## Implementation Steps

1. Create base Kustomize structure
2. Create namespace manifest
3. Create ConfigMap with common settings
4. Create Secrets template
5. Create Indexer deployment and service
6. Create API deployment, service, and HPA
7. Create Crank deployment and service
8. Create Ingress with TLS
9. Create PodDisruptionBudgets
10. Create dev/staging/prod overlays
11. Add documentation

## Testing

- [ ] `kubectl kustomize k8s/overlays/dev` validates
- [ ] `kubectl kustomize k8s/overlays/staging` validates
- [ ] `kubectl kustomize k8s/overlays/prod` validates
- [ ] Apply to test cluster and verify services start
