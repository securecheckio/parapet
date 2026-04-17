# Parapet Security Rules

**Minimal starter rules included with the engine.**

## What's Included

The Parapet engine ships with minimal rules by design:

| File | Description | Size |
|------|-------------|------|
| **`presets/default-protection.json`** | 3 critical rules (unlimited delegation, malicious programs, large SOL transfers) | 68 lines |
| **`examples/demo-basic-rules.json`** | Identical to default-protection (for documentation) | 68 lines |
| **`examples/invalid-example.json`** | Shows invalid rule syntax (testing only) | 34 lines |

**That's it!** No comprehensive rules ship with the engine.

## Why So Minimal?

Following the **Snort IDS model**: the engine is separate from the rules. This ensures:

1. **No unexpected blocking** - You explicitly choose your security posture
2. **Engine stays lean** - Fast downloads, minimal dependencies
3. **Rules update independently** - No redeployment needed for new threats
4. **Clear separation** - Open-source engine, commercial rule feeds

## Getting Comprehensive Rules

### 1. Community Rules (Free, Open Source)

**[Parapet Rules Repository](https://github.com/securecheckio/parapet-rules)**

```bash
# Clone community rules
git clone https://github.com/securecheckio/parapet-rules.git

# Use via file path
RULES_PATH=./parapet-rules/community/core-protection.json

# Or enable rule feeds for automatic updates
RULES_FEED_ENABLED=true
RULES_FEED_URLS=https://parapet-rules.securecheck.io/community/core-protection.json
```

### 2. Premium Rules (Commercial)

Advanced rules with lower false positives, specialized use cases, and faster updates.

Contact: [SecureCheck Team](https://securecheck.io)

## Rule Feeds (Recommended)

Automatically update rules without redeployment:

```toml
# proxy/config.toml
[rules.feed]
enabled = true

[[rules.feed.sources]]
url = "https://parapet-rules.securecheck.io/community/core-protection.json"
name = "SecureCheck Community"
priority = 0
```

See [RULE_FEEDS.md](../../docs/RULE_FEEDS.md) for full documentation.

## Creating Custom Rules

See [RULES_FORMAT.md](../../docs/RULES_FORMAT.md) for the JSON specification.

## Test Fixtures

Additional test rules live in `proxy/tests/fixtures/rules/` - these are for testing only, not production use.
