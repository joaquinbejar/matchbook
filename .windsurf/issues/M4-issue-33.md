# Issue #33: Docker configuration for all services

## Acceptance Criteria

- [ ] Dockerfiles for each service:
  - `indexer/Dockerfile` — Indexer service
  - `api/Dockerfile` — API server (REST + WebSocket)
  - `crank/Dockerfile` — Crank service
- [ ] Multi-stage builds for smaller images:
  - Build stage with Rust toolchain
  - Runtime stage with minimal base (distroless or alpine)
- [ ] `docker-compose.yml` for local development:
  - All services
  - PostgreSQL with TimescaleDB
  - Redis
  - Proper networking and volumes
- [ ] `docker-compose.prod.yml` for production-like setup
- [ ] Environment variable configuration
- [ ] Health check endpoints
- [ ] Non-root user for security
- [ ] `.dockerignore` files
- [ ] Documentation for building and running

## Technical Approach

### Dockerfile Strategy

Use multi-stage builds with cargo-chef for efficient layer caching:

1. **Chef stage**: Install cargo-chef
2. **Planner stage**: Generate recipe.json for dependencies
3. **Builder stage**: Build dependencies first (cached), then build app
4. **Runtime stage**: Minimal image with just the binary

### Base Images

- **Build**: `rust:1.75-bookworm` (Debian for compatibility)
- **Runtime**: `debian:bookworm-slim` (small but has glibc for Rust binaries)

### Architecture Support

- Build for both `amd64` and `arm64` using Docker buildx

### Security

- Run as non-root user (UID 1000)
- No shell in production images where possible
- CA certificates for HTTPS

## Files to Create

```
matchbook/
├── indexer/
│   └── Dockerfile
├── api/
│   └── Dockerfile
├── crank/
│   └── Dockerfile
├── docker-compose.yml
├── docker-compose.prod.yml
├── .dockerignore
└── docs/
    └── docker.md
```

## Implementation Steps

1. Create `.dockerignore` at root
2. Create `indexer/Dockerfile`
3. Create `api/Dockerfile`
4. Create `crank/Dockerfile`
5. Create `docker-compose.yml` for local dev
6. Create `docker-compose.prod.yml` for production
7. Create `docs/docker.md` with usage instructions
8. Test builds locally

## Environment Variables

### Common
- `RUST_LOG` - Logging level
- `DATABASE_URL` - PostgreSQL connection string
- `REDIS_URL` - Redis connection string

### Indexer
- `GEYSER_ENDPOINT` - Geyser gRPC endpoint
- `GEYSER_X_TOKEN` - Geyser auth token
- `SOLANA_RPC_URL` - Solana RPC URL

### API
- `API_HOST` - Bind address (default: 0.0.0.0)
- `API_PORT` - HTTP port (default: 8080)
- `WS_PORT` - WebSocket port (default: 8081)

### Crank
- `SOLANA_RPC_URL` - Solana RPC URL
- `CRANK_KEYPAIR` - Base58 encoded keypair

## Testing

- [ ] Build each Dockerfile locally
- [ ] Run docker-compose up and verify services start
- [ ] Verify health endpoints respond
- [ ] Verify non-root user is used
- [ ] Check image sizes are reasonable (<100MB runtime)
