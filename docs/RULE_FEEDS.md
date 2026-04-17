# Rule Feeds - Auto-Updating Security Rules

Rule feeds let you automatically update Parapet's security rules without redeployment. Rules are fetched from HTTP URLs and updated in the background with zero downtime.

## Why Use Rule Feeds?

**Without rule feeds:**
- Rules are baked into your Docker image
- Every rule change requires rebuild & redeploy
- Slower response to new threats

**With rule feeds:**
- Rules update automatically from HTTP URLs
- Zero downtime - updates happen in background
- Instant protection against new threats
- Compose rules from multiple sources (community + custom)

## Quick Start

Add to your `config.toml`:

```toml
[rule_feeds]
enabled = true
poll_interval = 3600  # Check every hour

[[rule_feeds.sources]]
url = "https://parapet-rules.securecheck.io/community/core-protection.json"
name = "community"
priority = 1
```

That's it! Rules will now auto-update every hour.

### Community feed URLs (category-based)

Published from [`securecheckio/parapet-rules`](https://github.com/securecheckio/parapet-rules) (GitHub Pages at `https://parapet-rules.securecheck.io/`; raw files also at `raw.githubusercontent.com`); see that repo’s `LICENSE`.

- `https://parapet-rules.securecheck.io/community/core-protection.json` — built-in analyzers only (no third-party API keys)
- `https://parapet-rules.securecheck.io/community/helius-protection.json` — requires `HELIUS_API_KEY`
- `https://parapet-rules.securecheck.io/community/jupiter-protection.json` — requires `JUPITER_API_KEY`
- `https://parapet-rules.securecheck.io/community/rugcheck-protection.json` — requires `RUGCHECK_API_KEY`
- `https://parapet-rules.securecheck.io/community/ai-agent-protection.json` — AI-agent / flowbits patterns (no API keys)
- `https://parapet-rules.securecheck.io/community/advanced-patterns.json` — CPI + instruction-padding patterns (no API keys)
- `https://parapet-rules.securecheck.io/community/trading-bot-alerts.json` — alert-first trading-oriented patterns

There is **no** single-file “all rules” URL: repeating every rule in one JSON would duplicate the same definitions already published in the category feeds. List each feed you want under separate `[[rule_feeds.sources]]` entries.

## Configuration

### Basic Setup

```toml
[rule_feeds]
enabled = true           # Enable auto-updates
poll_interval = 3600     # Check feeds every 3600 seconds (1 hour)

[[rule_feeds.sources]]
url = "https://parapet-rules.securecheck.io/community/core-protection.json"
name = "community"       # Human-readable name for logs
priority = 1             # Lower number = higher priority (0 is highest)
min_interval = 300       # Min 300s (5 min) between requests to this URL
```

### Multiple Feeds (Composable Security)

```toml
[rule_feeds]
enabled = true
poll_interval = 3600

# Community base rules (priority 1)
[[rule_feeds.sources]]
url = "https://parapet-rules.securecheck.io/community/core-protection.json"
name = "community"
priority = 1
min_interval = 300

# Your custom overrides (priority 0 = highest, wins conflicts)
[[rule_feeds.sources]]
url = "https://my-company.com/custom-overrides.json"
name = "custom"
priority = 0
min_interval = 180
```

## Timing Parameters Explained

### `poll_interval` (Global)

How often to check ALL feeds for updates.

**Default:** 3600 seconds (1 hour)

**Recommendations:**
- **Production (balanced):** `3600` (1 hour) - Good balance of freshness & efficiency
- **High-security:** `600` (10 min) - Faster threat response, more bandwidth
- **Cost-conscious:** `7200` (2 hours) - Less frequent checks, lower costs

**Example:**
```toml
poll_interval = 3600  # Check feeds every hour
```

### `min_interval` (Per-Feed)

Minimum seconds between requests to a specific feed URL (rate limiting).

**Default:** 60 seconds

**Recommendations:**
- **Public feeds:** `300` (5 min) - Respectful rate limiting
- **Internal feeds:** `180` (3 min) - Can poll faster
- **External APIs:** `600` (10 min) - Conservative, avoid rate limits

**Why it matters:** Even if `poll_interval` triggers, a feed won't be fetched if it was requested less than `min_interval` seconds ago.

**Example:**
```toml
[[rule_feeds.sources]]
url = "https://parapet-rules.securecheck.io/community/core-protection.json"
min_interval = 300  # Don't hit this URL more than once per 5 min
```

### How They Work Together

```
Time 0:00  → poll_interval triggers → Check all feeds
           → Feed A: last fetch was 6 min ago (> min_interval 5 min) → FETCH ✓
           → Feed B: last fetch was 2 min ago (< min_interval 5 min) → SKIP ⏭️

Time 1:00  → poll_interval triggers → Check all feeds
           → Feed A: last fetch was 1 hr ago → FETCH ✓
           → Feed B: last fetch was 1 hr ago → FETCH ✓
```

## HTTP Caching (Efficient)

Rule feeds use standard HTTP caching:

1. **ETag support**: Server sends ETag header, Parapet includes it in next request
2. **Last-Modified support**: Server sends Last-Modified header, Parapet uses If-Modified-Since
3. **304 Not Modified**: If rules haven't changed, server responds with 304 (no data transfer)

**Result:** Even with frequent polling, bandwidth usage is minimal - only new/changed rules are downloaded.

## Priority System (Conflict Resolution)

When the same rule ID appears in multiple feeds, **lower priority number wins**.

**Example:**

```toml
[[rule_feeds.sources]]
url = "https://parapet-rules.securecheck.io/community/core-protection.json"
priority = 1  # Community defaults

[[rule_feeds.sources]]
url = "https://my-company.com/custom-overrides.json"
priority = 0  # Highest - overrides community rules
```

If both feeds have a rule with ID `max-sol-transfer`:
- Priority 0 (custom) beats priority 1 (community)
- Your custom rule wins

**Priority levels:**
- `0` = Highest (custom overrides)
- `1` = High (main ruleset)
- `2, 3, ...` = Lower priority

## Zero Downtime Updates

Rule updates happen in the background without blocking RPC requests:

1. Background task checks feeds every `poll_interval`
2. New rules are fetched and validated
3. Rules are merged using priority system
4. Active ruleset is atomically swapped
5. Old rules are cleaned up

**RPC requests are never blocked** - transactions continue processing during updates.

## Use Cases

### Pattern 1: Community Rules Only

```toml
[rule_feeds]
enabled = true
poll_interval = 3600

[[rule_feeds.sources]]
url = "https://parapet-rules.securecheck.io/community/core-protection.json"
priority = 1
```

### Pattern 2: Community + Custom Overrides

```toml
[rule_feeds]
enabled = true
poll_interval = 3600

[[rule_feeds.sources]]
url = "https://parapet-rules.securecheck.io/community/core-protection.json"
priority = 1

[[rule_feeds.sources]]
url = "https://my-company.com/custom-rules.json"
priority = 0  # Wins conflicts
```

### Pattern 3: Multiple Specialized Feeds

```toml
[rule_feeds]
enabled = true
poll_interval = 600  # Check every 10 min (high-security)

[[rule_feeds.sources]]
url = "https://parapet-rules.securecheck.io/community/core-protection.json"
priority = 2

[[rule_feeds.sources]]
url = "https://parapet-rules.securecheck.io/community/trading-bot-alerts.json"
priority = 1

[[rule_feeds.sources]]
url = "https://internal.corp/compliance-rules.json"
priority = 0
min_interval = 180  # Internal, can poll faster
```

## Environment Variables (Alternative)

For simple setups, use environment variables instead of `config.toml`:

```bash
# Enable feeds
RULES_FEED_ENABLED=true

# Single feed URL
RULES_FEED_URL_1=https://parapet-rules.securecheck.io/community/core-protection.json

# Multiple feeds (higher number = lower priority)
RULES_FEED_URL_1=https://parapet-rules.securecheck.io/community/core-protection.json
RULES_FEED_URL_2=https://my-company.com/custom-rules.json
```

**Note:** Environment variables don't support `min_interval` - use `config.toml` for full control.

## Monitoring

Check logs for feed activity:

```bash
# Fly.io
fly logs -a parapet-proxy | grep "feed"

# Docker
docker logs parapet-proxy | grep "feed"
```

**Example logs:**
```
📡 Starting multi-source rule feed updater
   2 feed sources configured
   Polling every 3600 seconds
   [1] community (priority: 1, rate limit: 300s)
   [2] custom (priority: 0, rate limit: 180s)
📊 Merged 15 rules from 2 sources
```

## Troubleshooting

### Rules not updating

1. Check `enabled = true` in config
2. Verify feed URLs are accessible: `curl -I <feed-url>`
3. Check logs for fetch errors
4. Verify `poll_interval` and `min_interval` are reasonable

### Feed fetch errors

```
Failed to fetch from https://...: connection timeout
```

**Solutions:**
- Verify network connectivity
- Check firewall rules (outbound HTTPS)
- Verify feed URL is correct and accessible
- Try increasing `poll_interval` if server is rate-limiting

### Priority conflicts not working

- Lower priority NUMBER = higher priority
- Priority 0 > Priority 1 > Priority 2
- Check logs for "Overriding rule X from higher priority source"

## Security Considerations

1. **HTTPS only**: Always use HTTPS URLs for feeds
2. **Trusted sources**: Only add feeds from sources you trust
3. **Review rules**: Monitor feed changes (check repo/changelog)
4. **Test first**: Validate feeds on devnet before mainnet
5. **Fallback**: Keep a static `RULES_PATH` as fallback if feeds fail

## Rule Feed Format

Each feed is a JSON file with this structure:

```json
{
  "version": "1.0",
  "published_at": "2026-04-15T00:00:00Z",
  "source": "my-rules",
  "rules": [
    {
      "version": "1.0",
      "id": "unique-rule-id",
      "name": "Rule Name",
      "enabled": true,
      "rule": {
        "action": "block",
        "conditions": {...},
        "message": "Block message"
      }
    }
  ],
  "deprecated_rule_ids": ["old-rule-to-remove"]
}
```

## Next Steps

- See [parapet-rules repository](https://github.com/securecheckio/parapet-rules) for community feeds
- Read [parapet-rules README](../../parapet-rules/README.md) for composable feed examples
- Check [Deployment Guides](../deployments/README.md) for platform-specific setup

## FAQ

**Q: What happens if a feed is unreachable?**  
A: Parapet continues using the last successfully fetched rules. Logs will show the fetch error.

**Q: Can I disable a feed temporarily?**  
A: Yes, comment out the `[[rule_feeds.sources]]` block or set a very high `min_interval`.

**Q: Do rule updates affect in-flight requests?**  
A: No, in-flight requests complete with the rules they started with. Only new requests use updated rules.

**Q: Can I use GitHub raw URLs?**  
A: Yes! Example: `https://raw.githubusercontent.com/your-org/rules/main/rules.json`

**Q: What's the bandwidth usage?**  
A: Minimal due to HTTP caching. Typically <1KB per feed per hour (only if rules changed).

**Q: Can I mix static rules and feeds?**  
A: Yes! Set both `RULES_PATH` (static fallback) and enable `rule_feeds` (dynamic updates).
