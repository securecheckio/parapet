# Parapet RPC Proxy - Fly.io Optimized Dockerfile
# Multi-stage build for minimal runtime image size

# ============================================
# Builder Stage: Compile Rust binary
# ============================================
FROM rustlang/rust:nightly as builder

WORKDIR /build

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY core ./core
COPY proxy ./proxy
COPY scanner ./scanner

# Build optimized release binary
RUN cargo build --release -p parapet-proxy --bin parapet-proxy

# ============================================
# Runtime Stage: Minimal Debian image
# ============================================
FROM debian:bookworm-slim

# Install minimal runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy compiled binary from builder
COPY --from=builder /build/target/release/parapet-proxy /usr/local/bin/parapet-proxy

# Copy rules directory (baked into image)
COPY proxy/rules /app/rules

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

CMD ["parapet-proxy"]
