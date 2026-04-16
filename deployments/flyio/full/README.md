# Full Fly.io Deployment - Proxy + API + Redis + Dashboard

**What you get:** Complete stack with RPC proxy, API server, Redis, and web dashboard.

**What's included:**

- RPC proxy (analyzes and blocks malicious transactions)
- API server (manage rules via authenticated endpoints)
- Redis (shared state and caching)
- Dashboard (web UI to monitor activity feed in real-time)
- No default rules (you must configure your own)

**Best for:**

- 🤖 **AI agent operators** - Monitor agent activity, see what's blocked, manage rules dynamically
- 🏢 Teams needing centralized rule management
- 📊 Users who want real-time monitoring & dashboards
- 🔧 Advanced setups requiring runtime configuration

## Architecture

```
Dashboard (port 80) ──┐
Proxy (port 8899) ────┼──> Redis (Upstash)
API (port 3001) ──────┘
```

**AI Agent Use Case:**

Your AI agent connects to the proxy URL, and you can:

- 📊 Monitor all agent transactions in the dashboard (real-time activity feed)
- 🚫 See what was blocked and why
- 🔧 Manage rules dynamically via API (no redeployment needed)
- 📈 Track agent behavior patterns over time

## Deploy to Fly.io

```bash
cd deployments/flyio/full

# Option 1: Automated (recommended)
cp .env.example .env
# Edit .env with your config
./deploy.sh

# Option 2: Manual
# Create Redis
fly redis create --name parapet-redis

# Deploy API
fly launch --config fly.api.toml --dockerfile Dockerfile.api --no-deploy
fly redis connect parapet-redis -a parapet-api
fly secrets set AUTHORIZED_WALLETS=YourWallet -a parapet-api
fly deploy --config fly.api.toml

# Deploy Proxy
fly launch --config fly.proxy.toml --dockerfile Dockerfile.proxy --no-deploy
fly redis connect parapet-redis -a parapet-proxy
fly deploy --config fly.proxy.toml

# Deploy Dashboard
fly launch --config fly.dashboard.toml --dockerfile Dockerfile.dashboard --no-deploy
fly deploy --config fly.dashboard.toml
```

## Configuration

### Rule Management

The full deployment supports three rule management approaches:

1. **HTTP Rule Feeds** (Recommended): Auto-update rules from URLs
  - Set `RULES_FEED_ENABLED=true` and configure feed URLs
  - See [Rule Feeds Documentation](../../../../parapet-rules/feeds/README.md)
2. **Dynamic Rules via API**: Manage rules at runtime via authenticated API
  - Requires Redis + API + authorized Solana wallet
  - Create/update/delete rules without redeployment
3. **Static Rules**: Baked into Docker image
  - Set `RULES_PATH` to local JSON file
  - Requires rebuild/redeploy to update

### Required Secrets

```bash
fly secrets set AUTHORIZED_WALLETS=YourWallet -a parapet-api
```

### Optional Secrets

```bash
fly secrets set HELIUS_API_KEY=key -a parapet-proxy
fly secrets set JUPITER_API_KEY=key -a parapet-proxy
fly secrets set HELIUS_API_KEY=key -a parapet-api
fly secrets set JUPITER_API_KEY=key -a parapet-api
fly secrets set MCP_API_KEYS=key -a parapet-api
```

### Configure Rules (REQUIRED)

**⚠️ Parapet has NO default rules. You must configure security rules.**

**Option 1** - HTTP Rule Feeds (recommended, auto-update)

```toml
[rule_feeds]
enabled = true
poll_interval = 3600  # Check every hour

[[rule_feeds.sources]]
url = "https://rules.parapet.security/community-base.json"
priority = 1
```

**📖 Full documentation:** [Rule Feeds Guide](../../../docs/RULE_FEEDS.md)

**Option 2** - Use API for dynamic rules (see API docs)

Get community rules at [github.com/securecheckio/parapet-rules](https://github.com/securecheckio/parapet-rules)

## Your URLs

Get your URLs after deployment:

```bash
fly info -a parapet-proxy     # Proxy URL
fly info -a parapet-api       # API URL
fly info -a parapet-dashboard # Dashboard URL
```

Example output: `https://your-app-name.fly.dev`

## Use It

```typescript
import { Connection } from '@solana/web3.js';

// Use your proxy URL from `fly info -a parapet-proxy`
const connection = new Connection('https://YOUR-PROXY-URL.fly.dev');
```

- **Dashboard**: Monitor activity feed in real-time
- **API**: Manage rules via authenticated endpoints

## Common Tasks

**View logs**: `fly logs -a parapet-proxy` / `fly logs -a parapet-api`
**Scale proxy**: `fly autoscale set min=1 max=10 -a parapet-proxy`
**Scale API**: `fly scale count 2 -a parapet-api`
**Update proxy**: `fly deploy --config fly.proxy.toml`
**Update API**: `fly deploy --config fly.api.toml`
**Update dashboard**: `fly deploy --config fly.dashboard.toml`