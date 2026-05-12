#!/bin/bash
# Check current network configuration

echo "🔍 Parapet RPC Proxy Status"
echo "═══════════════════════════════════════"

# Check if container is running
if ! docker ps | grep -q demo-parapet-proxy; then
    echo "❌ Proxy container not running"
    echo ""
    echo "Start with: ./start-with-docker.sh"
    exit 1
fi

# Get network from logs
NETWORK=$(docker logs demo-parapet-proxy 2>&1 | grep "Network:" | tail -1 | awk -F'Network: ' '{print $2}' | awk '{print $1}')

# Get upstream URL
UPSTREAM=$(docker logs demo-parapet-proxy 2>&1 | grep "Upstream RPC:" | tail -1 | grep -oP 'https://[^?]+' || echo "unknown")

# Health check
if curl -s http://localhost:8899/health > /dev/null 2>&1; then
    HEALTH="✅ healthy"
else
    HEALTH="❌ unhealthy"
fi

echo "Status: $HEALTH"
echo "Network: $NETWORK"
echo "Upstream: $UPSTREAM"
echo "Proxy: http://localhost:8899"
echo ""

case $NETWORK in
    *devnet*)
        echo "🎯 Ready for: Samui wallet testing"
        echo "   Wallet URL: http://localhost:5174"
        ;;
    *mainnet*)
        echo "🎯 Ready for: Drift attack replay"
        echo "   Demo Monitor: http://localhost:3030"
        ;;
esac

echo ""
echo "Switch network: ./switch-network.sh [devnet|mainnet]"
