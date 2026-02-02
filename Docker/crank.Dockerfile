# syntax=docker/dockerfile:1

# =============================================================================
# Matchbook Crank Service Dockerfile
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
RUN cargo build --release -p matchbook-crank --bin matchbook-crank

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
COPY --from=builder /app/target/release/matchbook-crank /usr/local/bin/

# Set ownership
RUN chown matchbook:matchbook /usr/local/bin/matchbook-crank

# Switch to non-root user
USER matchbook

# Environment variables with defaults
ENV RUST_LOG=info
ENV SOLANA_RPC_URL=https://api.devnet.solana.com
ENV CRANK_KEYPAIR=""
ENV MIN_PROFIT_LAMPORTS=1000
ENV MAX_MATCHES_PER_TX=8

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:9091/health || exit 1

# Expose metrics port
EXPOSE 9091

# Run the crank service
ENTRYPOINT ["matchbook-crank"]
