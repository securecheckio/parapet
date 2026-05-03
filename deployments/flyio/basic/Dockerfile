# Parapet RPC Proxy - Fly.io Optimized Dockerfile
# Multi-stage build for minimal runtime image size

# ============================================
# Builder Stage: Compile Rust binary
# ============================================
FROM rustlang/rust:nightly as builder

WORKDIR /build

# Copy workspace files
# NOTE: This Dockerfile must be run from the parapet root directory
# All workspace members are required even though we only build rpc-proxy
COPY Cargo.toml Cargo.lock ./
COPY core ./core
COPY upstream ./upstream
COPY rpc-proxy ./rpc-proxy
COPY scanner ./scanner
COPY api ./api
COPY mcp ./mcp
COPY tools ./tools

# Build optimized release binary
RUN cargo build --release -p parapet-rpc-proxy --bin parapet-rpc-proxy

# ============================================
# Runtime Stage: Minimal Debian image with GLIBC 2.39+
# ============================================
# IMPORTANT: Uses Debian Trixie (testing) for GLIBC 2.39+ support
# Rust nightly binaries require GLIBC 2.39+. Debian Bookworm only has 2.36.
# If you see "GLIBC_2.39 not found" errors, ensure you're using Trixie or newer.
FROM debian:trixie-slim

# Install minimal runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy compiled binary from builder
COPY --from=builder /build/target/release/parapet-rpc-proxy /usr/local/bin/parapet-rpc-proxy

# Copy rules directory (baked into image)
COPY rpc-proxy/rules /app/rules

# Expose RPC port
EXPOSE 8899

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8899/health || exit 1

# Run as non-root user for security
RUN useradd -m -u 1000 parapet && \
    chown -R parapet:parapet /app
USER parapet

# Set working directory where config.toml would be (if used)
WORKDIR /app

CMD ["parapet-rpc-proxy"]
