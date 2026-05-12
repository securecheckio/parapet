#!/bin/bash
# Run Parapet RPC Proxy in console mode (foreground, with live logs)

set -e

NETWORK=${1:-devnet}

# Determine upstream URL
case $NETWORK in
    devnet)
        UPSTREAM="https://devnet.helius-rpc.com/?api-key=a1dcb99c-304a-458f-b4d6-a9fcdb8eb7e3"
        ;;
    mainnet)
        UPSTREAM="https://mainnet.helius-rpc.com/?api-key=a1dcb99c-304a-458f-b4d6-a9fcdb8eb7e3"
        ;;
    *)
        echo "❌ Invalid network: $NETWORK"
        echo "   Usage: ./run-proxy-console.sh [devnet|mainnet]"
        exit 1
        ;;
esac

echo "🚀 Starting Parapet RPC Proxy in console mode"
echo "═══════════════════════════════════════════════"
echo "Network: $NETWORK"
echo "Port: 8899"
echo "Rules: ./rules/demo-combined-rules.json"
echo "Press Ctrl+C to stop"
echo "═══════════════════════════════════════════════"
echo ""

# Stop any existing container
docker-compose down 2>/dev/null || true

# Update docker-compose.yml with the network
sed -i "s|UPSTREAM_RPC_URL=.*|UPSTREAM_RPC_URL=\${UPSTREAM_RPC_URL:-$UPSTREAM}|" docker-compose.yml

# Run in background then follow logs (avoids docker-compose bug)
docker-compose up -d
docker logs demo-parapet-proxy -f
