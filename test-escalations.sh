#!/bin/bash
set -e

echo "🧪 Testing Parapet Escalation Flow"
echo "=================================="

# Check Redis
echo "✓ Checking Redis..."
redis-cli ping || { echo "❌ Redis not running on 6379"; exit 1; }

# Create test configs
echo "✓ Creating test configs..."
cat > /tmp/parapet-proxy-test.toml << 'PROXYCONF'
[server]
bind_address = "0.0.0.0:8899"
upstream_rpc_url = "https://api.mainnet-beta.solana.com"

[redis]
url = "redis://127.0.0.1:6379"

[escalations]
enabled = true
approver_wallet = "vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg"
api_url = "http://localhost:3001"

[rules]
path = "proxy/rules/presets/ai-agent-protection.json"

[security]
default_block_threshold = 10  # Very strict to trigger escalations easily
PROXYCONF

cat > /tmp/parapet-api-test.toml << 'APICONF'
[server]
port = 3001

[redis]
url = "redis://127.0.0.1:6379"

[solana]
rpc_url = "https://api.mainnet-beta.solana.com"

[auth]
authorized_wallets = ["vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg"]
APICONF

echo "✓ Configs created"
echo ""
echo "Next steps:"
echo "1. Terminal 1: cargo run --release -p parapet-api-core -- --config /tmp/parapet-api-test.toml"
echo "2. Terminal 2: cargo run --release -p parapet-proxy -- --config /tmp/parapet-proxy-test.toml"
echo "3. Terminal 3: Run this test script"
echo ""
echo "Or use the binaries:"
echo "1. ./target/release/parapet-api-core (needs config in api-core/config.toml)"
echo "2. ./target/release/parapet-proxy (needs config in proxy/config.toml)"
