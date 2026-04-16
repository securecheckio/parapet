# Parapet RPC Proxy - Proxy-only Docker Build
# Multi-stage build for a small runtime image

FROM rustlang/rust:nightly AS builder

WORKDIR /build

# Copy workspace files required for proxy build
COPY Cargo.toml Cargo.lock ./
COPY core ./core
COPY proxy ./proxy
COPY scanner ./scanner

# Build release binary
RUN cargo build --release -p parapet-proxy --bin parapet-proxy

FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/target/release/parapet-proxy /usr/local/bin/parapet-proxy
COPY proxy/rules /app/rules

EXPOSE 8899

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8899/health || exit 1

RUN useradd -m -u 1000 parapet && \
    chown -R parapet:parapet /app
USER parapet

CMD ["parapet-proxy"]
