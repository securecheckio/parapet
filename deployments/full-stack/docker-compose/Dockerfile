# Parapet RPC Proxy - Optimized Docker Build
# Multi-stage build for minimal runtime image size

# ============================================
# Builder Stage: Compile Rust binary
# ============================================
FROM rustlang/rust:nightly as builder

WORKDIR /build

# Copy all workspace members
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
# Runtime Stage: Minimal Debian image
# ============================================
FROM debian:trixie-slim

# Install minimal runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*

# Copy compiled binary from builder
COPY --from=builder /build/target/release/parapet-rpc-proxy /usr/local/bin/parapet-rpc-proxy
COPY rpc-proxy/rules /app/rules

# Set working directory
WORKDIR /app

# No config.toml in image - use environment variables
# Users can volume-mount their own config.toml if needed

# Expose RPC port
EXPOSE 8899

# Run as non-root user for security (created by deployment script)
# Note: When deployed with --network host, this eliminates NAT overhead (~2-5% latency improvement)

CMD ["parapet-rpc-proxy"]
