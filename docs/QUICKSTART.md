# Parapet Quick Start

Get Parapet running in under 5 minutes.

## What You're Deploying

A complete security layer for Solana transactions with:
- **RPC Proxy** - Analyzes every transaction before it hits the network
- **Dashboard** - Web UI for human oversight and approvals
- **API** - Manages rules and escalations
- **Redis** - Coordinates everything

## Prerequisites

- Docker and Docker Compose installed
- A Solana RPC endpoint (or use public: `https://api.mainnet-beta.solana.com`). For **multiple RPC endpoints** (failover) on the proxy or API, see [Operations Guide — Multi-upstream RPC](OPERATIONS_GUIDE.md#multi-upstream-rpc-proxy-and-api).
- Your wallet address (for approving risky transactions)

## Start Parapet

```bash
# 1. Navigate to full-stack docker directory
cd deployments/full-stack/docker-compose

# 2. Create environment file
cp .env.example .env

# 3. Edit .env - REQUIRED: Set your wallet address
nano .env  # or vim, or any editor

# Minimum required:
# ESCALATION_APPROVER_WALLET=YourWalletAddressHere

# 4. Start everything
docker-compose up -d

# 5. Check it's running
docker-compose ps
```

## Access Your Services

Open in browser:
- **Dashboard**: http://localhost:8080 (Human approval interface)
- **API**: http://localhost:3001/health (Should return "OK")
- **Proxy**: http://localhost:8899/health (Should return "OK")

## Use It

### Protect Your Application

Point your Solana client to the proxy:

```typescript
import { Connection } from '@solana/web3.js';

// Instead of direct RPC
// const connection = new Connection('https://api.mainnet-beta.solana.com');

// Use Parapet proxy
const connection = new Connection('http://localhost:8899');

// That's it! All transactions are now analyzed
```

### Test It

```bash
# Send a test request
curl http://localhost:8899 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
```

### Monitor It

```bash
# Watch logs
docker-compose logs -f proxy

# See blocked transactions
docker-compose logs proxy | grep BLOCKED
```

## What Happens When a Transaction is Blocked?

1. Risky transaction detected → Shows in **Dashboard** at http://localhost:8080
2. You (human) review transaction details
3. Click "Approve" or "Deny"
4. If approved, agent can retry transaction

## Next Steps

### Add API Keys (Recommended)

Better security analysis with external services:

```bash
# Edit .env
HELIUS_API_KEY=your_key_here    # Get at: helius.dev
JUPITER_API_KEY=your_key_here   # Get at: station.jup.ag/api-keys
```

Restart: `docker-compose restart`

### Adjust Security Threshold

```bash
# .env
DEFAULT_BLOCK_THRESHOLD=70  # Default: blocks risk >= 70
# Lower = stricter (blocks more)
# Higher = more permissive (blocks less)
```

### Use Strict Rules

```bash
# .env
RULES_PATH=/app/rules/presets/strict.json  # Maximum security
```

Restart: `docker-compose restart proxy`

## Troubleshooting

### Services won't start

```bash
# Check logs
docker-compose logs

# Common fixes:
# - Ports already in use: Change ports in docker-compose.yml
# - Missing ESCALATION_APPROVER_WALLET: Add to .env
```

### "Connection refused" from client

```bash
# Make sure proxy is running
curl http://localhost:8899/health

# If not healthy, check logs
docker-compose logs proxy
```

### Dashboard shows "Disconnected"

```bash
# Check API is running
curl http://localhost:3001/health

# Check Redis
docker-compose ps redis
```

## Learn More

- [Complete Documentation](docs/)
- [Agent Integration Guide](docs/AGENT_GUIDE.md) - For AI agents like OpenClaw/Cursor
- [User Guide](docs/USER_GUIDE.md) - For running scanner and proxy
- [Operations Guide](docs/OPERATIONS_GUIDE.md) - For DevOps
- [Use Cases](docs/USE_CASES.md) - Real-world examples

## Stop Parapet

```bash
# Stop services
docker-compose down

# Stop and remove all data
docker-compose down -v
```

## Need Help?

1. Check logs: `docker-compose logs -f`
2. Verify .env configuration
3. Test each service health endpoint
4. See [deployments/full-stack/docker-compose/README.md](deployments/full-stack/docker-compose/README.md) for detailed troubleshooting
