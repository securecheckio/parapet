#!/bin/bash
# Demo Monitor + Parapet RPC Proxy - Single Command Startup
# Configured for Samui Wallet testing with 0.01 SOL alert rule

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🛡️  Parapet Security - Demo Monitor with RPC Proxy"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Starting with:"
echo "  • Samui Wallet rule: Alert on SOL transfers > 0.01 SOL"
echo "  • Drift Attack Detection: 7 critical rules (block pre-signed exploits)"
echo ""

# Check if we're in the right directory
if [ ! -f "docker-compose.yml" ]; then
    echo "❌ Error: Must run from demo-monitor directory"
    echo "   cd parapet/tools/demo-monitor"
    exit 1
fi

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "❌ Error: Docker is not running"
    echo "   Please start Docker and try again"
    exit 1
fi

# Check if parapet:latest image exists
if ! docker images | grep -q "parapet.*latest"; then
    echo "❌ Error: parapet:latest Docker image not found"
    echo "   Please build it first:"
    echo "   cd ../../.. && docker build -t parapet:latest -f Dockerfile ."
    exit 1
fi

# Stop any existing containers
echo "🧹 Cleaning up existing containers..."
docker-compose down 2>/dev/null || true

# Start the RPC proxy
echo ""
echo "🚀 Starting Parapet RPC Proxy with Docker..."
docker-compose up -d

# Wait for proxy to be healthy
echo ""
echo "⏳ Waiting for RPC proxy to be ready..."
sleep 3

# Check health
MAX_RETRIES=10
RETRY=0
while [ $RETRY -lt $MAX_RETRIES ]; do
    if curl -s http://localhost:8899/health > /dev/null 2>&1; then
        echo "✅ RPC Proxy is healthy!"
        break
    fi
    RETRY=$((RETRY + 1))
    if [ $RETRY -eq $MAX_RETRIES ]; then
        echo "❌ Error: RPC Proxy failed to start"
        echo "Check logs: docker logs demo-parapet-proxy"
        exit 1
    fi
    echo "   Waiting... ($RETRY/$MAX_RETRIES)"
    sleep 2
done

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✨ Setup Complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "📊 Parapet RPC Proxy: http://localhost:8899"
echo "   Rules: 8 total (1 Samui wallet + 7 Drift attack detection)"
echo ""
echo "🎯 To start the Demo Monitor:"
echo "   cargo run --release"
echo "   Then open: http://localhost:3030"
echo ""
echo "📋 Useful commands:"
echo "   docker logs demo-parapet-proxy -f     # View proxy logs"
echo "   docker-compose down                    # Stop proxy"
echo ""
echo "🔧 Samui Wallet Testing:"
echo "   Configure Samui wallet to use: http://localhost:8899"
echo "   • SOL transfers > 0.01 → Alert"
echo "   • Pre-signed durable nonce transactions → Block/Alert (Drift protection)"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
