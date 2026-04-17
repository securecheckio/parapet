# FlowState Configuration Guide

## Overview

This guide explains how to configure Parapet's flowstate system for different deployment scenarios: AI agent protection and enterprise RPC protection.

## Table of Contents

- [Environment Variables](#environment-variables)
- [AI Agent Deployment](#ai-agent-deployment)
- [Enterprise Deployment](#enterprise-deployment)
- [Rule Presets](#rule-presets)
- [Performance Tuning](#performance-tuning)
- [Troubleshooting](#troubleshooting)

## Environment Variables

### Core FlowState Settings

```bash
# Enable/disable flowstate (default: enabled if not set)
SOLSHIELD_FLOWBITS_ENABLED=true

# Maximum number of wallets to track (default: unlimited)
# For AI agents: Set to 1
# For enterprise: Set based on number of internal wallets (e.g., 100, 1000)
SOLSHIELD_FLOWBITS_MAX_WALLETS=unlimited

# Default TTL for flowstate in seconds (default: 3600 = 1 hour)
# Can be overridden per-rule
SOLSHIELD_FLOWBITS_DEFAULT_TTL=3600

# Maximum global flowstate keys (default: 10000)
# For enterprise cross-wallet detection
SOLSHIELD_FLOWBITS_MAX_GLOBAL_KEYS=10000
```

## AI Agent Deployment

### Use Case

Protect autonomous AI agents from:
- Runaway behavior (transaction loops)
- Compromise detection (repeated blocks)
- Gradual exfiltration (slow fund draining)
- Suspicious program interaction

### Configuration

```bash
# AI Agent specific settings
SOLSHIELD_FLOWBITS_ENABLED=true
SOLSHIELD_FLOWBITS_MAX_WALLETS=1  # Single wallet per agent
SOLSHIELD_FLOWBITS_DEFAULT_TTL=3600  # 1 hour default
```

### Recommended Rules

**Phase 1 (Simple Counters)**:
- `ai-agent-protection.json` - Velocity limits, account spam, repeated blocks

**Phase 2 (Advanced)**:
- `ai-agent-advanced.json` - Exfiltration detection, program interaction tracking

### Rule Loading

```bash
# Load AI agent protection rules
parapet-proxy \
  --rules proxy/rules/presets/ai-agent-protection.json \
  --rules proxy/rules/presets/ai-agent-advanced.json \
  --rpc-url https://api.mainnet-beta.solana.com
```

### Tuning for AI Agents

| Rule | Parameter | Default | Tuning Guidance |
|------|-----------|---------|-----------------|
| Velocity Limit | Threshold | 10 tx/10min | Increase for high-frequency trading agents |
| Account Spam | Threshold | 5 accounts/5min | Decrease for agents that never create accounts |
| Exfiltration | Threshold | 4 transfers/24h | Allowlist legitimate recipients (exchanges) |
| Program Interaction | Threshold | 3 interactions/7d | Maintain allowlist of known safe programs |

### Example: High-Frequency Trading Agent

```bash
# Higher velocity limit for HFT agents
SOLSHIELD_FLOWBITS_DEFAULT_TTL=600  # 10 minutes (shorter window)
```

Edit `ai-agent-protection.json`:
```json
{
  "id": "ai-agent-velocity-limit",
  "rule": {
    "conditions": {
      "field": "flowstate:transaction_count",
      "operator": "greater_than_or_equal",
      "value": 50  // Increased from 10
    },
    "flowstate": {
      "ttl_seconds": 600  // 10 minutes instead of 10 minutes
    }
  }
}
```

## Enterprise Deployment

### Use Case

Protect organization's internal wallets from:
- Lateral movement (coordinated breach)
- Mass compromise (token drain velocity)
- Stale durable nonces (defense-in-depth for Drift-style attacks - note: core Drift patterns already covered by `drift-exploit-protection.json`)

### Configuration

```bash
# Enterprise specific settings
SOLSHIELD_FLOWBITS_ENABLED=true
SOLSHIELD_FLOWBITS_MAX_WALLETS=1000  # Number of internal wallets
SOLSHIELD_FLOWBITS_DEFAULT_TTL=3600  # 1 hour default
SOLSHIELD_FLOWBITS_MAX_GLOBAL_KEYS=50000  # For cross-wallet tracking
```

### Recommended Rules

**Phase 1 (Basic)**:
- `enterprise-basic-protection.json` - Nonce usage tracking

**Phase 2 (Cross-Wallet)**:
- `enterprise-cross-wallet.json` - Lateral movement, token drain velocity, nonce staleness

### Rule Loading

```bash
# Load enterprise protection rules
parapet-proxy \
  --rules proxy/rules/presets/enterprise-basic-protection.json \
  --rules proxy/rules/presets/enterprise-cross-wallet.json \
  --rpc-url https://internal-rpc.company.com
```

### Tuning for Enterprise

| Rule | Parameter | Default | Tuning Guidance |
|------|-----------|---------|-----------------|
| Lateral Movement | Threshold | 3 wallets/1h | Allowlist shared recipients (treasury, exchanges) |
| Token Drain Velocity | Threshold | 10 transfers/15min | Adjust based on normal trading volume |
| Nonce Staleness | TTL | 30 minutes | Match organization's multisig workflow |
| Nonce Staleness | Transfer Threshold | 1 SOL | Adjust based on typical transfer sizes |

### Example: Allowlisting Shared Recipients

Create a custom rule that skips lateral movement detection for known recipients:

```json
{
  "id": "enterprise-lateral-movement-allowlist",
  "name": "Enterprise Lateral Movement (with Allowlist)",
  "rule": {
    "action": "block",
    "conditions": {
      "all": [
        {"field": "system:has_sol_transfer", "operator": "equals", "value": true},
        {"field": "system:sol_recipients", "operator": "not_in", "value": [
          "CompanyTreasury1111111111111111111111111",
          "ExchangeHotWallet1111111111111111111111",
          "PayrollWallet11111111111111111111111111"
        ]},
        {"field": "flowstate_global:suspicious_recipient:{recipient}", "operator": "greater_than", "value": 2}
      ]
    },
    "flowstate": {
      "scope": "global",
      "increment": ["suspicious_recipient:{recipient}"],
      "ttl_seconds": 3600
    }
  }
}
```

### Example: Adjusting Nonce Staleness for Multisig

If your organization uses multisigs with longer approval times:

```json
{
  "id": "track-nonce-advancement",
  "rule": {
    "flowstate": {
      "ttl_seconds": 7200  // 2 hours instead of 30 minutes
    }
  }
}
```

## Rule Presets

### Available Presets

| Preset | Scope | Phase | Description |
|--------|-------|-------|-------------|
| `ai-agent-protection.json` | Per-Wallet | 1 | Basic AI agent protection (velocity, spam, blocks) |
| `enterprise-basic-protection.json` | Per-Wallet | 1 | Basic enterprise protection (nonce tracking) |
| `ai-agent-advanced.json` | Per-Wallet | 2 | Advanced AI agent detection (exfiltration, programs) |
| `enterprise-cross-wallet.json` | Global | 2 | Cross-wallet detection (lateral movement, drains, nonce staleness) |

### Combining Presets

You can load multiple presets simultaneously:

```bash
# AI Agent + Enterprise (for organizations with AI agents)
parapet-proxy \
  --rules proxy/rules/presets/ai-agent-protection.json \
  --rules proxy/rules/presets/ai-agent-advanced.json \
  --rules proxy/rules/presets/enterprise-cross-wallet.json
```

## Performance Tuning

### Memory Usage

FlowState memory usage scales with:
- Number of tracked wallets (`SOLSHIELD_FLOWBITS_MAX_WALLETS`)
- Number of global flowstate keys (`SOLSHIELD_FLOWBITS_MAX_GLOBAL_KEYS`)
- Number of unique recipients/mints/programs tracked

**Estimated Memory Usage**:
- Per-wallet state: ~1KB per wallet
- Global state: ~100 bytes per unique key
- Example: 1000 wallets + 10000 global keys ≈ 2MB

### Latency Impact

FlowState add minimal latency:
- Per-wallet lookup: <0.1ms
- Global lookup: <0.2ms
- Variable interpolation: <0.5ms
- **Total overhead**: <1ms per transaction (p50), <2ms (p99)

### Cleanup Intervals

FlowState automatically clean up expired entries every 60 seconds. To adjust:

```rust
// In FlowbitStateManager::new()
self.cleanup_interval = Duration::from_secs(120); // 2 minutes
```

## Troubleshooting

### Issue: FlowState not working

**Symptoms**: Rules with flowstate don't trigger
**Causes**:
1. FlowState disabled: Check `SOLSHIELD_FLOWBITS_ENABLED=true`
2. Rule loading warning: Check logs for "flowstate-dependent rules loaded but flowstate disabled"

**Solution**:
```bash
# Verify flowstate are enabled
grep "FlowState enabled" logs/parapet.log

# Check rule loading
grep "flowstate" logs/parapet.log
```

### Issue: False positives

**Symptoms**: Legitimate transactions blocked
**Causes**:
1. Thresholds too low
2. TTLs too long (accumulating old activity)
3. Missing allowlists

**Solution**:
1. Start with `action: alert` instead of `action: block`
2. Tune thresholds based on observed activity
3. Add allowlists for known recipients/programs

### Issue: High memory usage

**Symptoms**: Memory grows unbounded
**Causes**:
1. `SOLSHIELD_FLOWBITS_MAX_WALLETS` set too high or unlimited
2. `SOLSHIELD_FLOWBITS_MAX_GLOBAL_KEYS` set too high
3. Variable interpolation creating too many unique keys

**Solution**:
```bash
# Set reasonable limits
SOLSHIELD_FLOWBITS_MAX_WALLETS=1000
SOLSHIELD_FLOWBITS_MAX_GLOBAL_KEYS=10000

# Monitor memory usage
ps aux | grep parapet-proxy
```

### Issue: Variable interpolation not working

**Symptoms**: Flowbit names contain literal `{recipient}` instead of addresses
**Causes**:
1. Field not available (analyzer not enabled)
2. Array empty (no recipients in transaction)
3. Unknown variable name

**Solution**:
```bash
# Check logs for interpolation warnings
grep "Unknown variable" logs/parapet.log
grep "Field.*not found" logs/parapet.log

# Verify analyzers are enabled
grep "Registered analyzer" logs/parapet.log
```

## Best Practices

1. **Start with Alert Mode**: Use `action: alert` initially, upgrade to `block` after tuning
2. **Monitor Metrics**: Track false positive/negative rates
3. **Gradual Rollout**: Enable rules incrementally, starting with Phase 1
4. **Maintain Allowlists**: Keep allowlists up-to-date for recipients, programs, tokens
5. **Regular Review**: Review blocked transactions weekly, adjust thresholds as needed
6. **Test Scenarios**: Use integration tests to validate rule behavior before production
7. **Document Customizations**: Keep track of threshold adjustments and allowlist changes

## Support

For issues or questions:
- GitHub Issues: https://github.com/securecheck/parapet/issues
- Documentation: https://github.com/securecheck/parapet/docs
- Integration examples: `parapet/integrations/`
