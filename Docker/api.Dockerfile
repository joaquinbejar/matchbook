# syntax=docker/dockerfile:1

# =============================================================================
# Matchbook API Server Dockerfile
# Multi-stage build for efficient caching and minimal runtime image
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Chef - Install cargo-chef for dependency caching
# -----------------------------------------------------------------------------
FROM rust:1.85-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

# -----------------------------------------------------------------------------
# Stage 2: Planner - Generate dependency recipe
# -----------------------------------------------------------------------------
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# -----------------------------------------------------------------------------
# Stage 3: Builder - Build dependencies and application
# -----------------------------------------------------------------------------
FROM chef AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    libssl-dev \
    pkg-config \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# Copy recipe and build dependencies first (cached layer)
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source code and build application
COPY . .
RUN cargo build --release -p matchbook-api --bin matchbook-api

# -----------------------------------------------------------------------------
# Stage 4: Runtime - Minimal production image
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN groupadd --gid 1000 matchbook \
    && useradd --uid 1000 --gid matchbook --shell /bin/false matchbook

# Copy binary from builder
COPY --from=builder /app/target/release/matchbook-api /usr/local/bin/

# Set ownership
RUN chown matchbook:matchbook /usr/local/bin/matchbook-api

# Switch to non-root user
USER matchbook

# Environment variables with defaults
ENV RUST_LOG=info
ENV DATABASE_URL=postgres://matchbook:matchbook@postgres/matchbook
ENV REDIS_URL=redis://redis:6379
ENV API_HOST=0.0.0.0
ENV API_PORT=8080
ENV WS_PORT=8081

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Expose HTTP and WebSocket ports
EXPOSE 8080 8081

# Run the API server
ENTRYPOINT ["matchbook-api"]
