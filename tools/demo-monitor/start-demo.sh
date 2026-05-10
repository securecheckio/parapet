#!/bin/bash
# Drift Attack Demo - Quick Start Script

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🛡️  Parapet Security - Drift Attack Demo"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "This demo shows how Parapet could have prevented the \$285M Drift attack"
echo "through pre-signing transaction analysis."
echo ""

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: Must run from demo-monitor directory"
    echo "   cd parapet/tools/demo-monitor"
    exit 1
fi

# Build the demo
echo "📦 Building demo monitor..."
cargo build --release

echo ""
echo "✅ Build complete!"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🚀 Starting Demo Monitor..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Demo UI: http://localhost:3030"
echo ""
echo "Press Ctrl+C to stop"
echo ""

# Set environment variables
export RUST_LOG=info
export PARAPET_RPC_URL=${PARAPET_RPC_URL:-http://localhost:8899}

# Run the demo
cargo run --release
