# Basic Fly.io Deployment - Proxy Only

**What you get:** RPC proxy with transaction inspection and blocking using static security rules.

**What's included:**

- RPC proxy (analyzes and blocks malicious transactions)
- No default rules (you must configure your own)
- No external dependencies (no Redis, no API server)

**Best for:** Production use, simple deployments, cost-conscious setups.

## Deploy

```bash
# IMPORTANT: Deploy from the parapet root directory (not from deployments/flyio/basic)
cd /path/to/parapet

# Launch (creates app)
fly launch --config deployments/flyio/basic/fly.toml --dockerfile deployments/flyio/basic/Dockerfile --no-deploy

# Deploy
fly deploy --config deployments/flyio/basic/fly.toml --dockerfile deployments/flyio/basic/Dockerfile -a parapet-rpc-proxy

# Get your URL
fly info -a parapet-rpc-proxy

# Verify
curl https://YOUR-APP.fly.dev/health
```

**Why deploy from root?** The Dockerfile references workspace files (core, rpc-proxy, scanner, api, mcp, tools) that are in the parapet root directory.

## Configuration

### Optional API Keys

```bash
fly secrets set HELIUS_API_KEY=key -a parapet-rpc-proxy
fly secrets set JUPITER_API_KEY=key -a parapet-rpc-proxy
fly secrets set OTTERSEC_API_KEY=key -a parapet-rpc-proxy
```

### Configure Rules (REQUIRED)

**⚠️ Parapet has NO default rules. You must configure security rules or transactions pass through unblocked.**

**Rule Sources:**

- `parapet/rpc-proxy/rules/` → Your custom rules (baked into your deployment)
- `parapet-rules/` → Community rules (separate repo, use via HTTP feeds)

#### Option A: Static Rules (Baked into Image)

**For your own custom rules** - add them to `rpc-proxy/rules/` before deploying:

1. **Create your custom rules file:**

```bash
# In parapet/ directory - create or edit your rules file
vim rpc-proxy/rules/my-custom-rules.json
```

Example (see `rpc-proxy/rules/custom-example.json` for template):

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

The Dockerfile copies `rpc-proxy/rules/` to `/app/rules/` during build. To update rules, edit the file and redeploy.

**Note:** This is for YOUR custom rules. Community rules from `parapet-rules/` are better used via Option B (Rule Feeds).

#### Option B: Rule Feeds (Auto-Update, Recommended)

**⚡ Rules update automatically from HTTP URLs without redeployment!**

Configure in `fly.toml`:

```toml
[env]
  RULES_FEED_ENABLED = 'true'
  RULES_FEED_POLL_INTERVAL = '3600'  # Check every hour (seconds)
  RULES_FEED_URLS = 'https://parapet-rules.securecheck.io/community/core-protection.json'
```

For multiple feeds, use comma-separated URLs:
```toml
[env]
  RULES_FEED_URLS = 'https://example.com/feed1.json,https://example.com/feed2.json'
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

- `parapet-rules/community/core-protection.json` (built-in patterns) + optional `community/helius-protection.json` / `community/jupiter-protection.json` when API keys are configured
- See [github.com/securecheckio/parapet-rules](https://github.com/securecheckio/parapet-rules)

## Common Tasks

**View logs**: `fly logs -a parapet-rpc-proxy`
**Scale**: `fly autoscale set min=1 max=10 -a parapet-rpc-proxy`
**Add regions**: `fly regions add iad ord lhr -a parapet-rpc-proxy`
**Update**: `fly deploy`
**Change rules**: Edit `RULES_PATH` in fly.toml, then `fly deploy`

## Use It

```typescript
import { Connection } from '@solana/web3.js';

// Use your proxy URL from `fly info -a parapet-rpc-proxy`
const connection = new Connection('https://YOUR-APP.fly.dev');
```

## Troubleshooting

### GLIBC Version Error

If you see `GLIBC_2.39' not found` errors:

**Problem:** Rust nightly requires GLIBC 2.39+, but older Debian versions have 2.36.

**Solution:** The Dockerfile uses `debian:trixie-slim` which includes GLIBC 2.39+. If you modify the Dockerfile, ensure you use Debian Trixie or newer.

### Build Context Issues

If the build fails with "file not found" errors for workspace members:

**Problem:** The Dockerfile expects to be built from the parapet root directory with access to all workspace crates (core, api, mcp, tools, etc.).

**Solution:** Always run `fly deploy` from the parapet root directory and specify the full path to the config and Dockerfile:

```bash
cd /path/to/parapet
fly deploy --config deployments/flyio/basic/fly.toml --dockerfile deployments/flyio/basic/Dockerfile -a parapet-rpc-proxy
```

### Rules Not Loading

If you see "Loading 0 rules" in logs:

**Problem:** Rules feed URL is incorrect or RULES_PATH points to non-existent file.

**Solution:** 
- Verify the feed URL is accessible: `curl https://parapet-rules.securecheck.io/community/core-protection.json`
- Check fly.toml has `RULES_FEED_ENABLED = 'true'` and correct `RULES_FEED_URLS`
- For static rules, ensure RULES_PATH points to a file that exists in the Docker image at `/app/rules/`

### Machine Restarting Frequently

Check logs for errors:
```bash
fly logs -a parapet-rpc-proxy -n | tail -50
```

Common causes:
- Invalid rules causing startup failure (check rule validation errors in logs)
- Missing environment variables
- GLIBC version mismatch (see above)

