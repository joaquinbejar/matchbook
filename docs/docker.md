# Docker Guide

This guide explains how to build and run Matchbook services using Docker.

## Prerequisites

- Docker 24.0+
- Docker Compose 2.20+
- At least 8GB RAM available for Docker

## Quick Start (Local Development)

### 1. Start Infrastructure Only

Start PostgreSQL and Redis without building the Rust services:

```bash
docker-compose -f Docker/docker-compose.yml up -d postgres redis
```

### 2. Start All Services

Build and start all services:

```bash
docker-compose -f Docker/docker-compose.yml up -d
```

### 3. View Logs

```bash
# All services
docker-compose -f Docker/docker-compose.yml logs -f

# Specific service
docker-compose -f Docker/docker-compose.yml logs -f api
```

### 4. Stop Services

```bash
# Stop but keep volumes
docker-compose -f Docker/docker-compose.yml down

# Stop and remove volumes (clean slate)
docker-compose -f Docker/docker-compose.yml down -v
```

## Services

| Service | Port | Description |
|---------|------|-------------|
| `postgres` | 5432 | PostgreSQL with TimescaleDB |
| `redis` | 6379 | Redis for caching and pub/sub |
| `indexer` | 9090 | Geyser indexer (metrics) |
| `api` | 8080, 8081 | REST API and WebSocket |
| `crank` | 9091 | Crank service (metrics) |

## Environment Variables

Create a `.env` file in the project root:

```bash
# Logging
RUST_LOG=info

# Solana
SOLANA_RPC_URL=https://api.devnet.solana.com

# Geyser (for indexer)
GEYSER_ENDPOINT=http://your-geyser-endpoint:10000
GEYSER_X_TOKEN=your-auth-token

# Crank
CRANK_KEYPAIR=your-base58-encoded-keypair
MIN_PROFIT_LAMPORTS=1000
MAX_MATCHES_PER_TX=8
```

## Building Images

### Build All Services

```bash
docker-compose -f Docker/docker-compose.yml build
```

### Build Specific Service

```bash
docker-compose -f Docker/docker-compose.yml build api
```

### Build with No Cache

```bash
docker-compose -f Docker/docker-compose.yml build --no-cache
```

### Build for Multiple Architectures

## Docker File Structure

All Docker-related files are located in the `Docker/` directory:

```
Docker/
├── .dockerignore              # Files to exclude from build context
├── api.Dockerfile             # API server image
├── crank.Dockerfile           # Crank service image
├── docker-compose.yml         # Local development compose
├── docker-compose.prod.yml    # Production compose
└── indexer.Dockerfile         # Indexer service image
```

```bash
# Create builder
docker buildx create --name matchbook-builder --use

# Build and push multi-arch images
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t ghcr.io/joaquinbejar/matchbook-api:latest \
  -f Docker/api.Dockerfile \
  --push .
```

## Production Deployment

### Using docker-compose.prod.yml

```bash
# Set required environment variables
export DATABASE_URL=postgres://user:pass@your-postgres:5432/matchbook
export REDIS_URL=redis://your-redis:6379
export GEYSER_ENDPOINT=http://your-geyser:10000
export GEYSER_X_TOKEN=your-token
export SOLANA_RPC_URL=https://api.mainnet-beta.solana.com
export CRANK_KEYPAIR=your-keypair

# Start services
docker-compose -f Docker/docker-compose.prod.yml up -d
```

### Using Pre-built Images

```bash
# Pull latest images
docker-compose -f Docker/docker-compose.prod.yml pull

# Start with specific version
VERSION=v1.0.0 docker-compose -f Docker/docker-compose.prod.yml up -d
```

## Health Checks

All services expose health endpoints:

```bash
# API health
curl http://localhost:8080/health

# Indexer health
curl http://localhost:9090/health

# Crank health
curl http://localhost:9091/health
```

## Troubleshooting

### Container Won't Start

Check logs:
```bash
docker-compose -f Docker/docker-compose.yml logs <service-name>
```

### Database Connection Issues

Verify PostgreSQL is healthy:
```bash
docker-compose -f Docker/docker-compose.yml exec postgres pg_isready -U matchbook
```

### Out of Memory

Increase Docker memory limit or reduce service resources in docker-compose.yml.

### Build Failures

Clear build cache:
```bash
docker builder prune -a
docker-compose -f Docker/docker-compose.yml build --no-cache
```

## Image Sizes

Target sizes for production images:

| Image | Target Size |
|-------|-------------|
| matchbook-indexer | < 100MB |
| matchbook-api | < 100MB |
| matchbook-crank | < 80MB |

Check actual sizes:
```bash
docker images | grep matchbook
```

## Security Notes

- All services run as non-root user (UID 1000)
- Sensitive data should be passed via environment variables or secrets
- Never commit `.env` files with real credentials
- Use Docker secrets in production for sensitive values

## CI/CD Integration

### GitHub Actions Example

```yaml
- name: Build and push
  uses: docker/build-push-action@v5
  with:
    context: .
    file: ./api/Dockerfile
    push: true
    tags: ghcr.io/${{ github.repository }}/matchbook-api:${{ github.sha }}
    cache-from: type=gha
    cache-to: type=gha,mode=max
```

## Related Documentation

- [Deployment Guide](../.internalDoc/07-deployment.md)
- [Operations Guide](../.internalDoc/08-operations.md)
