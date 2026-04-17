# Parapet OSS Rule Presets

**This directory contains ONE minimal starter preset with 3 critical rules.**

Parapet ships with minimal rules by design. You must explicitly configure comprehensive security rules to avoid unexpected behavior.

## What's Included

**`default-protection.json`** - 3 critical rules:
- Block unlimited token delegations (u64::MAX)
- Block known malicious programs
- Block large SOL transfers (>10 SOL)

This provides basic protection but is NOT comprehensive. See below for production-grade rules.

### 2. Community Rules (Recommended)

**[Parapet Rules Repository](https://github.com/securecheckio/parapet-rules)**

**Community Rules:**
- `parapet-rules/community/*.json` - published community category feeds (see `parapet-rules/README.md`)
- Blocks malicious addresses, unlimited delegations, suspicious tokens, active rug pulls
- Good baseline for most use cases
- Additional community feed JSON: `parapet-rules/community/` (separate repo)

**📖 Auto-Update Guide:** [Rule Feeds Documentation](../../../docs/RULE_FEEDS.md) - Learn how to automatically update rules without redeployment.

### 3. Create Your Own

See [../../../docs/RULES_FORMAT.md](../../../docs/RULES_FORMAT.md) for documentation.

## Important

**Parapet will run with NO RULES if you don't configure any.** This means transactions pass through unblocked. You must explicitly:

1. Set `RULES_PATH` to a rules file, OR
2. Enable rule feeds with `RULES_FEED_ENABLED=true`, OR  
3. Use dynamic rules via the API (full deployment only)

Choose and configure your rules intentionally.
