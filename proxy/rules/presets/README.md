# Parapet OSS Rule Presets

**This directory intentionally contains no default rules.**

Parapet requires you to explicitly configure security rules - we don't include any by default to avoid unexpected transaction blocking.

## Getting Rules

### 1. Example Rules (Demo/Testing Only)

See [../examples/demo-basic-rules.json](../examples/demo-basic-rules.json) for a minimal example with:
- Block unlimited token delegations
- Block known malicious programs  
- Block large SOL transfers

**⚠️ Demo only - not for production**

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
