# Parapet - Unified Multi-Binary Docker Image
# Builds core Parapet binaries in a single image for flexible deployment
# Docker Compose can use this single image with different CMD values
#
# Build from the `parapet/` directory (this file lives next to Cargo.toml):
#   docker build -t parapet:latest .

# ============================================
# Builder Stage: Compile core Rust binaries
# ============================================
FROM rustlang/rust:nightly AS builder

WORKDIR /build

# Copy workspace configuration and all members
COPY Cargo.toml Cargo.lock ./
COPY core ./core
COPY upstream ./upstream
COPY rpc-proxy ./rpc-proxy
COPY scanner ./scanner
COPY api ./api
COPY mcp ./mcp
COPY tools ./tools

# Build only the binaries we need (exclude demo-monitor, rpc-perf, flowstate-perf)
RUN cargo build --release \
    -p parapet-rpc-proxy \
    -p parapet-api \
    -p parapet-mcp-server \
    -p parapet-scanner

# ============================================
# Runtime Stage: Minimal Debian image with all binaries
# ============================================
FROM debian:trixie-slim

# Install minimal runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy compiled binaries from builder
# Core services
COPY --from=builder /build/target/release/parapet-rpc-proxy /usr/local/bin/
COPY --from=builder /build/target/release/parapet-api /usr/local/bin/
COPY --from=builder /build/target/release/parapet-mcp /usr/local/bin/

# CLI scanner tools
COPY --from=builder /build/target/release/wallet-scanner /usr/local/bin/
COPY --from=builder /build/target/release/program-analyzer /usr/local/bin/
COPY --from=builder /build/target/release/tx-check /usr/local/bin/
COPY --from=builder /build/target/release/update-safe-lists /usr/local/bin/

# Copy rules (config files should be mounted at runtime or use env vars)
COPY rules ./rules

# Create non-root user for security
RUN useradd -m -u 1000 parapet && \
    chown -R parapet:parapet /app

USER parapet

# Expose common ports (override in docker-compose as needed)
EXPOSE 8899 3001

# Default to RPC proxy, but docker-compose can override with different CMD
CMD ["parapet-rpc-proxy"]
