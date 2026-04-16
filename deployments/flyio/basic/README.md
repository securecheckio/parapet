# Basic Fly.io Deployment - Proxy Only

**What you get:** RPC proxy with transaction inspection and blocking using static security rules.

**What's included:**

- RPC proxy (analyzes and blocks malicious transactions)
- No default rules (you must configure your own)
- No external dependencies (no Redis, no API server)

**Best for:** Production use, simple deployments, cost-conscious setups.

## Deploy

```bash
cd deployments/flyio/basic

# Launch (creates app)
fly launch --config fly.toml --dockerfile Dockerfile --no-deploy

# Deploy
fly deploy

# Get your URL
fly info -a parapet-proxy

# Verify
curl https://YOUR-APP.fly.dev/health
```

## Configuration

### Optional API Keys

```bash
fly secrets set HELIUS_API_KEY=key -a parapet-proxy
fly secrets set JUPITER_API_KEY=key -a parapet-proxy
fly secrets set OTTERSEC_API_KEY=key -a parapet-proxy
```

### Configure Rules (REQUIRED)

**⚠️ Parapet has NO default rules. You must configure security rules or transactions pass through unblocked.**

**Rule Sources:**

- `parapet/proxy/rules/` → Your custom rules (baked into your deployment)
- `parapet-rules/` → Community rules (separate repo, use via HTTP feeds)

#### Option A: Static Rules (Baked into Image)

**For your own custom rules** - add them to `proxy/rules/` before deploying:

1. **Create your custom rules file:**

```bash
# In parapet/ directory - create or edit your rules file
vim proxy/rules/my-custom-rules.json
```

Example (see `proxy/rules/custom-example.json` for template):

```json
{
  "version": "1.0",
  "published_at": "2026-04-15T00:00:00Z",
  "rules": [
    {
      "id": "my-sol-limit",
      "name": "Block Large SOL Transfers",
      "enabled": true,
      "rule": {
        "action": "block",
        "conditions": {"field": "system:max_sol_transfer", "operator": "greater_than", "value": 50000000000},
        "message": "🚨 Exceeds 50 SOL limit"
      }
    }
  ]
}
```

1. **Configure** `fly.toml`:

```toml
[env]
  UPSTREAM_RPC_URL = "https://api.mainnet-beta.solana.com"
  RULES_PATH = "/app/rules/my-custom-rules.json"
```

1. **Deploy** (rules are baked into the image):

```bash
fly deploy
```

The Dockerfile copies `proxy/rules/` to `/app/rules/` during build. To update rules, edit the file and redeploy.

**Note:** This is for YOUR custom rules. Community rules from `parapet-rules/` are better used via Option B (Rule Feeds).

#### Option B: Rule Feeds (Auto-Update, Recommended)

**⚡ Rules update automatically from HTTP URLs without redeployment!**

```toml
[rule_feeds]
enabled = true
poll_interval = 3600  # Check for updates every hour

[[rule_feeds.sources]]
url = "https://rules.parapet.security/community-base.json"
priority = 1
```

**Key benefits:**

- ✅ Zero downtime updates (background polling)
- ✅ Instant protection from new threats
- ✅ Compose multiple rule sources with priority system
- ✅ Efficient HTTP caching (minimal bandwidth)

**📖 Full documentation:** [Rule Feeds Guide](../../../docs/RULE_FEEDS.md)

**Quick config:**

- `poll_interval` - How often to check feeds (default: 3600s = 1 hour)
- `priority` - Lower number = higher priority for conflict resolution
- `min_interval` - Per-feed rate limit (default: 60s)

**Community Rules:**

- `parapet-rules/feeds/community-base.json` (7 universal rules)
- See [github.com/securecheckio/parapet-rules](https://github.com/securecheckio/parapet-rules)

## Common Tasks

**View logs**: `fly logs -a parapet-proxy`
**Scale**: `fly autoscale set min=1 max=10 -a parapet-proxy`
**Add regions**: `fly regions add iad ord lhr -a parapet-proxy`
**Update**: `fly deploy`
**Change rules**: Edit `RULES_PATH` in fly.toml, then `fly deploy`

## Use It

```typescript
import { Connection } from '@solana/web3.js';

// Use your proxy URL from `fly info -a parapet-proxy`
const connection = new Connection('https://YOUR-APP.fly.dev');
```

