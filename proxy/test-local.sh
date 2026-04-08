#!/bin/bash

# Quick local testing script for Parapet RPC Proxy
# This script runs the proxy with default settings for local testing

set -e  # Exit on error

echo "🚀 Starting Parapet RPC Proxy (Local Testing)"
echo "================================================"
echo ""

# Check if .env exists
if [ ! -f ".env" ]; then
    echo "⚠️  No .env file found. Creating from .env.local..."
    cp .env.local .env
    echo "✅ Created .env file"
    echo ""
    echo "⚠️  Using free public RPC endpoint (slow, rate-limited)"
    echo "   For better performance, edit .env and add your Helius API key:"
    echo "   UPSTREAM_RPC_URL=https://mainnet.helius-rpc.com/?api-key=YOUR_KEY"
    echo ""
fi

# Show current configuration
echo "📋 Current Configuration:"
echo "------------------------"
grep "^UPSTREAM_RPC_URL" .env | sed 's/api-key=[^&]*/<API_KEY_HIDDEN>/'
grep "^PROXY_PORT" .env
grep "^RULES_PATH" .env
echo ""

# Check if rules file exists
RULES_PATH=$(grep "^RULES_PATH" .env | cut -d'=' -f2)
if [ ! -f "$RULES_PATH" ]; then
    echo "❌ Rules file not found: $RULES_PATH"
    echo "   Available rules:"
    ls -1 rules/*.json | sed 's/^/   - /'
    exit 1
fi

echo "✅ Rules file found: $RULES_PATH"
echo ""

# Build and run
echo "🔨 Building (release mode)..."
cargo build --release

echo ""
echo "🎯 Starting proxy server..."
echo "   Access at: http://localhost:8899"
echo "   Press Ctrl+C to stop"
echo ""

cargo run --release
