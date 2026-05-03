# Parapet RPC Proxy - Basic Docker Build
# Multi-stage build for a small runtime image

FROM rustlang/rust:nightly AS builder

WORKDIR /build

# Copy all workspace members (Cargo workspace requires them all)
# Multi-stage build means only the final binary ends up in runtime image
COPY Cargo.toml Cargo.lock ./
COPY core ./core
COPY rpc-proxy ./rpc-proxy
COPY scanner ./scanner
COPY api ./api
COPY mcp ./mcp
COPY tools ./tools

# Build release binaries for proxy and wallet scanner (see `parapet-scanner` crate bin `wallet-scanner`)
RUN cargo build --release -p parapet-rpc-proxy --bin parapet-rpc-proxy && \
    cargo build --release -p parapet-scanner --bin wallet-scanner

FROM debian:trixie-slim

RUN apt-get update && \
    apt-get install -y ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/target/release/parapet-rpc-proxy /usr/local/bin/parapet-rpc-proxy
COPY --from=builder /build/target/release/wallet-scanner /usr/local/bin/wallet-scanner
COPY rpc-proxy/rules /app/rules

# No config.toml in image - use environment variables
# Users can volume-mount their own config.toml if needed

EXPOSE 8899

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8899/health || exit 1

RUN useradd -m -u 1000 parapet && \
    chown -R parapet:parapet /app
USER parapet

CMD ["parapet-rpc-proxy"]
