# Sol-Shield RPC Proxy - Optimized Docker Build
# Multi-stage build for minimal runtime image size

# ============================================
# Builder Stage: Compile Rust binary
# ============================================
FROM rustlang/rust:nightly as builder

WORKDIR /build

# Copy everything
COPY . .

# Build optimized release binary
RUN cargo build --release -p parapet-proxy --bin parapet-proxy

# ============================================
# Runtime Stage: Minimal Debian image
# ============================================
FROM debian:bookworm-slim

# Install minimal runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*

# Copy compiled binary from builder
COPY --from=builder /build/target/release/parapet-proxy /usr/local/bin/parapet-proxy

# Expose RPC port
EXPOSE 8899

# Run as non-root user for security (created by deployment script)
# Note: When deployed with --network host, this eliminates NAT overhead (~2-5% latency improvement)

CMD ["parapet-proxy"]
