# Security Rules

Rule-based transaction filtering for Parapet RPC Proxy.

## Quick Start

**Most users:** Use a preset from `presets/`

```bash
# Recommended default
RULES_PATH=./rules/presets/bot-essentials.json

# Or comprehensive protection
RULES_PATH=./rules/presets/comprehensive-protection.json
```

## Directory Structure

### `presets/` - Production-Ready Complete Sets

Grab-and-go complete rule configurations:

- **`bot-essentials.json`** ⭐ **Recommended** - Balanced protection for most users (7 rules)
  - Blocks unlimited delegations
  - Detects dangerous operation combos
  - Monitors large transfers
  - Fast, no external API calls

- **`comprehensive-protection.json`** - Maximum protection using all analyzers (19 rules)
  - Everything in bot-essentials
  - Token mint checks
  - System program monitoring
  - Complexity detection

- **`trading-bot-protection.json`** - Ultra-strict for automated bots (9 rules)
  - Only allow core programs
  - Block ALL delegations
  - Limit SOL transfers
  - No account creation

- **`enhanced-security.json`** - Advanced protection with external analyzers (12 rules)
  - Requires HELIUS_API_KEY or similar
  - Wallet reputation checks
  - Program verification

- **`anti-scam.json`** - Focus on known scammer protection (8 rules)
  - Requires HELIUS_API_KEY
  - Blocks known scammer wallets
  - Monitors suspicious patterns

### `policies/` - Mix-and-Match Policy Types

Combine multiple policies for custom configurations:

**`allowlists/`** - Whitelist-based policies
- `delegate-allowlist.json` - Only allow delegations to specific DEXes
- `program-allowlist.json` - Only allow specific programs
- `recipient-allowlist.json` - Only allow transfers to specific wallets
- `token-mint-allowlist.json` - Only allow specific token mints

**`blocklists/`** - Blacklist-based policies
- `program-blacklist.json` - Block known malicious programs

**`protections/`** - Specific security features
- `freeze-protection.json` - Block freeze operations
- `ownership-protection.json` - Detect non-owned account operations
- `spending-limits.json` - Enforce transaction size limits
- `ottersec-verified-only.json` - Require cryptographic verification

### `examples/` - Testing and Learning

Example rules for development and testing:

- `custom-example.json` - Template for creating custom rules
- `invalid-example.json` - Example of invalid rules (for testing validation)
- `test-balanced.json` - Moderate test configuration
- `test-complete.json` - Full feature test
- `test-permissive.json` - Minimal restrictions (debugging)
- `test-strict.json` - Maximum restrictions (stress testing)

## Rules Location Best Practices

**Version controlled (ship with binary):**
- `presets/` - Default configurations
- `policies/` - Reusable policy components
- `examples/` - Documentation and testing

**NOT version controlled (per-deployment):**
- `custom/` - Instance-specific rules (add to .gitignore)
- Customer-specific configurations
- Private blocklists/allowlists

**Override via environment:**
```bash
# Single file
RULES_PATH=./rules/presets/default-protection.json

# Directory (loads all .json files)
RULES_PATH=./rules/presets

# Custom rules
RULES_PATH=/etc/parapet/rules/custom
```

## Creating Custom Rules

1. Copy `examples/custom-example.json`
2. Modify conditions and actions
3. Point `RULES_PATH` to your file
4. Restart proxy

See `../RULES.md` for rule format documentation.

## Field Reference

All available fields are documented in the analyzer docs:
- Core analyzers: `../parapet/src/rules/analyzers/core/*.md`
- Third-party analyzers: `../parapet/src/rules/analyzers/third_party/*.md`

## Validation

Rules are validated at startup. If validation fails:
- Server logs the error
- Falls back to legacy security checks
- Check logs for missing fields or syntax errors
