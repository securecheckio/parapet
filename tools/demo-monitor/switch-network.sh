#!/bin/bash
# Switch Parapet RPC Proxy between devnet (Samui testing) and mainnet (Drift replay)

set -e

NETWORK=$1

if [ -z "$NETWORK" ]; then
    echo "Usage: ./switch-network.sh [devnet|mainnet]"
    echo ""
    echo "Current configuration:"
    docker exec demo-parapet-proxy env | grep UPSTREAM_RPC_URL || echo "  Container not running"
    exit 1
fi

case $NETWORK in
    devnet)
        echo "🔄 Switching to DEVNET (Samui wallet testing)..."
        UPSTREAM="https://devnet.helius-rpc.com/?api-key=a1dcb99c-304a-458f-b4d6-a9fcdb8eb7e3"
        ;;
    mainnet)
        echo "🔄 Switching to MAINNET (Drift attack replay)..."
        UPSTREAM="https://mainnet.helius-rpc.com/?api-key=a1dcb99c-304a-458f-b4d6-a9fcdb8eb7e3"
        ;;
    *)
        echo "❌ Invalid network: $NETWORK"
        echo "   Use 'devnet' or 'mainnet'"
        exit 1
        ;;
esac

# Update docker-compose.yml
sed -i "s|UPSTREAM_RPC_URL=.*|UPSTREAM_RPC_URL=\${UPSTREAM_RPC_URL:-$UPSTREAM}|" docker-compose.yml

# Recreate container (restart doesn't pick up env changes)
echo "🔄 Recreating Parapet RPC Proxy..."
docker-compose down
docker-compose up -d

# Wait for health
echo "⏳ Waiting for proxy to be ready..."
sleep 3

# Verify
if curl -s http://localhost:8899/health > /dev/null 2>&1; then
    echo "✅ Proxy is healthy on $NETWORK!"
    echo ""
    echo "📊 RPC Proxy: http://localhost:8899"
    echo "   Network: $NETWORK"
    echo "   Helius API: enabled"
else
    echo "❌ Proxy failed to start"
    exit 1
fi

echo ""
case $NETWORK in
    devnet)
        echo "🎯 Ready for: Samui wallet testing"
        echo "   Wallet URL: http://localhost:5174"
        ;;
    mainnet)
        echo "🎯 Ready for: Drift attack replay"
        echo "   Demo Monitor: http://localhost:3030"
        ;;
esac
