# OpenClaw + Parapet Integration Guide

Complete guide for protecting OpenClaw AI agents with Parapet security.

## What You Get

- **Automatic transaction analysis** - Every transaction analyzed before sending
- **Protection from drains** - Unlimited delegations, authority changes blocked
- **Human-in-the-loop** - Optional approval workflow for risky transactions
- **Zero code changes** - Point OpenClaw to Parapet RPC, done

---

## Quick Start (5 Minutes)

### 1. Start Parapet Services

```bash
cd parapet

# Terminal 1: RPC Proxy (required)
./parapet proxy

# Terminal 2: API for escalations (optional)
./parapet api
```

### 2. Point OpenClaw to Parapet

```bash
# Set OpenClaw's RPC endpoint
export SOLANA_RPC_URL=http://localhost:8899
```

### 3. Done!

All OpenClaw transactions now protected. Risky transactions blocked automatically.

---

## Architecture

```
┌─────────────────┐
│  OpenClaw Agent │
│  (Your AI Bot)  │
└────────┬────────┘
         │
         │ Transactions
         ↓
┌─────────────────────────┐
│   Parapet RPC Proxy     │
│   (localhost:8899)      │
│                         │
│  • Analyzes every TX    │
│  • Applies rules        │
│  • Blocks threats       │
└────────┬────────────────┘
         │
         ├──→ Safe TX → Solana RPC
         │
         └──→ Risky TX → Escalation
                         (if enabled)
```

---

## Setup Options

### Option A: Basic Protection (No Escalations)

**Best for:** Simple protection without human oversight

```bash
# 1. Start proxy only
./parapet proxy

# 2. Configure OpenClaw
export SOLANA_RPC_URL=http://localhost:8899

# 3. Done!
```

**Result:**
- ✅ Safe transactions pass through
- 🚫 Risky transactions blocked
- ❌ No human approval workflow

---

### Option B: With Human Approval Workflow

**Best for:** High-value operations requiring human oversight

```bash
# 1. Start Redis (required for escalations)
redis-server --daemonize yes

# 2. Start Parapet services
./parapet proxy &
./parapet api &

# 3. Configure proxy for escalations
export ENABLE_ESCALATIONS=true
export ESCALATION_APPROVER_WALLET=YOUR_WALLET_ADDRESS
export REDIS_URL=redis://localhost:6379

# 4. Configure OpenClaw
export SOLANA_RPC_URL=http://localhost:8899
export PARAPET_API_URL=http://localhost:3000
```

**Result:**
- ✅ Safe transactions pass through
- ⏸️  Risky transactions create escalation
- 👤 Human approves/denies via API
- ♻️  Agent automatically retries after approval

---

## Configuration

### Proxy Configuration

Create `rpc-proxy/config.toml`:

```toml
[server]
port = 8899
bind_address = "0.0.0.0"

[upstream]
url = "https://api.mainnet-beta.solana.com"
timeout_secs = 30

[network]
current = "mainnet-beta"

[rules]
path = "./rpc-proxy/rules/presets/balanced.json"

[escalations]
enabled = true
redis_url = "redis://localhost:6379"
approver_wallet = "YOUR_WALLET_ADDRESS_HERE"
timeout_secs = 300  # 5 minutes
```

### API Configuration

**Recommended: Use TOML config**

```bash
cd api
cp config.example.toml config.toml
nano config.toml
```

Edit `api/config.toml`:

```toml
[server]
host = "0.0.0.0"
port = 3000

[redis]
url = "redis://localhost:6379"

[solana]
rpc_url = "https://api.mainnet-beta.solana.com"
network = "mainnet-beta"

[auth]
# Wallets allowed to approve escalations
authorized_wallets = [
    "YOUR_WALLET_ADDRESS_HERE"
]

[rate_limiting]
max_concurrent_scans = 10
scans_per_hour_per_key = 1000
```

**Secrets via environment variables:**

```bash
# API keys (keep these in env, not in config.toml)
export MCP_API_KEYS=your_mcp_key_1,your_mcp_key_2
```

### Environment Variables

**Use for secrets and deployment-specific overrides only:**

```bash
# Secrets (ALWAYS use env vars for these)
export HELIUS_API_KEY=your_helius_key
export JUPITER_API_KEY=your_jupiter_key
export MCP_API_KEYS=your_mcp_key_1,your_mcp_key_2

# Optional overrides (when TOML config isn't sufficient)
export UPSTREAM_RPC_URL=https://custom-rpc.example.com  # Override upstream
export REDIS_URL=redis://production-redis:6379          # Override Redis
```

**Not recommended:** Using env vars for all configuration. Use TOML config files instead.

---

## Security Rules

### Preset Rule Files

```bash
# Strict - Block anything suspicious
export RULES_PATH=./rpc-proxy/rules/presets/strict.json

# Balanced - Good defaults (recommended)
export RULES_PATH=./rpc-proxy/rules/presets/balanced.json

# Permissive - Only critical threats
export RULES_PATH=./rpc-proxy/rules/presets/permissive.json
```

### What Gets Blocked

**Default rules block:**
- Unlimited token delegations
- Authority changes without limits
- Suspicious program interactions
- Known malicious programs
- Drain patterns (multiple transfers)

**Configurable via rules:**
- Transaction size limits
- Transfer frequency limits
- Program allowlists/blocklists
- Risk score thresholds

---

## MCP Server Integration (Advanced)

For OpenClaw users wanting deeper integration with escalation support.

### 1. Setup MCP Server

```bash
cd parapet/examples

# Install dependencies
npm install @solana/web3.js
npm install @modelcontextprotocol/sdk
```

### 2. Configure OpenClaw

Add to OpenClaw's MCP configuration:

```json
{
  "mcpServers": {
    "parapet-solana": {
      "command": "node",
      "args": ["/absolute/path/to/parapet/mcp/parapet-mcp-server.ts"],
      "env": {
        "PARAPET_RPC_URL": "http://localhost:8899",
        "PARAPET_API_URL": "http://localhost:3000"
      }
    }
  }
}
```

### 3. Available Tools

**`solana_simulate_safe`**
- Simulates transaction with Parapet analysis
- Returns security verdict before sending
- No blockchain state changes

**`solana_send_safe`**
- Sends transaction through Parapet
- Handles escalations automatically
- Polls for human approval if blocked
- Retries with fresh blockhash after approval

---

## Escalation Workflow

### How It Works

```
1. OpenClaw creates transaction
   ↓
2. Sends to Parapet RPC (localhost:8899)
   ↓
3. Parapet analyzes transaction
   ↓
4. If CRITICAL risk:
   • Create escalation in API
   • Return error with escalation_id
   ↓
5. Human reviews in dashboard/API
   ↓
6. Approve or Deny
   ↓
7. If approved:
   • OpenClaw polls for status
   • Rebuilds TX with fresh blockhash
   • Retries send
   ↓
8. Transaction completes
```

### Approving Escalations

**Option 1: Via API**

```bash
# Get pending escalations
curl http://localhost:3000/api/v1/escalations/pending

# Approve
curl -X POST http://localhost:3000/api/v1/escalations/{escalation_id}/approve \
  -H "Content-Type: application/json" \
  -d '{
    "canonical_hash": "transaction_hash_from_escalation",
    "signature": "wallet_signature",
    "message": "signed_message"
  }'

# Deny
curl -X POST http://localhost:3000/api/v1/escalations/{escalation_id}/deny \
  -H "Content-Type: application/json" \
  -d '{
    "signature": "wallet_signature",
    "message": "signed_message"
  }'
```

**Option 2: Via Dashboard**

```bash
# Start dashboard
cd parapet/dashboard
npm install
npm run dev

# Open http://localhost:5173
# Connect wallet
# View and approve/deny escalations
```

**Option 3: Via WebSocket**

```javascript
const ws = new WebSocket('ws://localhost:3000/ws/escalations');

ws.on('message', (data) => {
  const event = JSON.parse(data);
  if (event.type === 'escalation_created') {
    console.log('New escalation:', event.data);
    // Show notification, etc.
  }
});
```

---

## Testing

### Test Safe Transaction

```bash
# Should pass through
curl -X POST http://localhost:8899 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getLatestBlockhash"
  }'
```

### Test Transaction Analysis

```bash
# Simulate a transaction
curl -X POST http://localhost:8899 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "simulateTransaction",
    "params": ["BASE58_ENCODED_TRANSACTION"]
  }'

# Response includes Parapet analysis:
# {
#   "result": {
#     "value": {
#       "parapet": {
#         "action": "block",
#         "risk_level": "critical",
#         "findings": [...]
#       }
#     }
#   }
# }
```

---

## Monitoring

### Health Checks

```bash
# Proxy health
curl http://localhost:8899/health

# API health
curl http://localhost:3000/health
```

### Logs

```bash
# Proxy logs
tail -f logs/proxy.log

# API logs
tail -f logs/api.log

# Or enable debug logging
export RUST_LOG=debug
./parapet proxy
```

### Metrics

```bash
# Enable performance tracking
export RULE_ENGINE_PERFORMANCE_TRACKING=true

# View metrics via API
curl http://localhost:3000/api/v1/metrics
```

---

## Troubleshooting

### OpenClaw can't connect to RPC

**Check proxy is running:**
```bash
curl http://localhost:8899/health
```

**Check OpenClaw RPC config:**
```bash
echo $SOLANA_RPC_URL
# Should show: http://localhost:8899
```

### All transactions blocked

**Check rules configuration:**
```bash
# Use more permissive rules
export RULES_PATH=./rpc-proxy/rules/presets/permissive.json
```

**Check upstream RPC:**
```bash
curl -X POST $UPSTREAM_RPC_URL \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
```

### Escalations not working

**Check Redis:**
```bash
redis-cli ping
# Should return: PONG
```

**Check escalations enabled:**
```bash
grep "enabled = true" rpc-proxy/config.toml
# Or
echo $ENABLE_ESCALATIONS
```

**Check API is running:**
```bash
curl http://localhost:3000/health
```

### High latency

**Check upstream latency:**
```bash
time curl -X POST $UPSTREAM_RPC_URL -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
```

**Optimize rules:**
- Use fewer complex rules
- Disable expensive analyzers
- Enable caching

---

## Production Deployment

### Using Docker

```bash
cd parapet/deployments/full-stack/docker-compose
cp .env.example .env
# Edit .env with your settings
docker-compose up -d
```

### Using Terraform

```bash
cd parapet/deployments/proxy-only/terraform
cp terraform.tfvars.example terraform.tfvars
# Edit terraform.tfvars
terraform init
terraform apply
```

### Security Checklist

- [ ] Use HTTPS in production (not HTTP)
- [ ] Secure Redis with password
- [ ] Whitelist authorized wallets
- [ ] Enable rate limiting
- [ ] Monitor logs for attacks
- [ ] Keep rules updated
- [ ] Regular security audits

---

## Performance

**Typical latency overhead:**
- Simple rules: <5ms
- Complex rules: 10-50ms
- Third-party analyzers: 100-300ms (cached)

**Recommended for OpenClaw:**
- Use `balanced.json` rules (good speed/security)
- Enable Redis caching
- Deploy proxy locally (minimize network latency)

---

## Advanced Configuration

### Custom Rules

Create custom rules for OpenClaw-specific patterns:

```json
{
  "id": "openclaw-high-value-transfer",
  "rule": {
    "action": "escalate",
    "conditions": {
      "all": [
        {"field": "system:total_sol_transfer", "operator": "greater_than", "value": 1000000000},
        {"field": "basic:instruction_count", "operator": "less_than", "value": 3}
      ]
    },
    "message": "High-value transfer >1 SOL"
  }
}
```

### Rate Limiting

```toml
[rate_limiting]
enabled = true
requests_per_minute = 60
burst_size = 10
```

### Webhook Notifications

```toml
[notifications]
webhook_url = "https://your-webhook.com/parapet"
events = ["escalation_created", "transaction_blocked"]
```

---

## Getting Help

- **Documentation**: `parapet/docs/`
- **Integration examples**: `parapet/integrations/`
- **Issues**: GitHub Issues

---

## Example: Complete Setup Script

Save as `setup-parapet-openclaw.sh`:

```bash
#!/bin/bash
set -e

echo "🚀 Setting up Parapet for OpenClaw..."

# Check dependencies
command -v redis-cli >/dev/null 2>&1 || { echo "❌ Redis required. Install: apt install redis-server"; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo "❌ Rust required. Install: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"; exit 1; }

# Start Redis if not running
if ! redis-cli ping > /dev/null 2>&1; then
    echo "Starting Redis..."
    redis-server --daemonize yes
fi

# Build Parapet
echo "Building Parapet..."
cd "$(dirname "$0")"
cargo build --release -p parapet-rpc-proxy -p parapet-api

# Create config directory
mkdir -p logs config

# Create proxy config
cat > config/proxy.toml <<EOF
[server]
port = 8899

[upstream]
url = "https://api.mainnet-beta.solana.com"

[rules]
path = "./rpc-proxy/rules/presets/balanced.json"

[escalations]
enabled = true
redis_url = "redis://localhost:6379"
timeout_secs = 300
EOF

# Start services
echo "Starting Parapet services..."
./parapet proxy --config config/proxy.toml > logs/proxy.log 2>&1 &
PROXY_PID=$!

./parapet api --config config/api.toml > logs/api.log 2>&1 &
API_PID=$!

# Wait for services to start
sleep 3

# Test health
if curl -s http://localhost:8899/health | grep -q "ok"; then
    echo "✅ Parapet RPC Proxy running on http://localhost:8899"
else
    echo "❌ Proxy failed to start. Check logs/proxy.log"
    exit 1
fi

if curl -s http://localhost:3000/health | grep -q "healthy"; then
    echo "✅ Parapet API running on http://localhost:3000"
else
    echo "❌ API failed to start. Check logs/api.log"
fi

# Save PIDs
echo "$PROXY_PID $API_PID" > .parapet.pids

echo ""
echo "✅ Setup complete!"
echo ""
echo "Configure OpenClaw:"
echo "  export SOLANA_RPC_URL=http://localhost:8899"
echo "  export PARAPET_API_URL=http://localhost:3000"
echo ""
echo "To stop Parapet:"
echo "  kill \$(cat .parapet.pids)"
echo ""
echo "Logs:"
echo "  tail -f logs/proxy.log"
echo "  tail -f logs/api.log"
```

Make it executable:
```bash
chmod +x setup-parapet-openclaw.sh
./setup-parapet-openclaw.sh
```

---

## Summary

**For basic protection:**
1. `./parapet proxy`
2. `export SOLANA_RPC_URL=http://localhost:8899`
3. Done!

**For human approval workflow:**
1. Start Redis
2. `./parapet proxy` + `./parapet api`
3. Configure escalations
4. Use MCP server for auto-retry

**That's it!** Your OpenClaw agent is now protected. 🛡️
