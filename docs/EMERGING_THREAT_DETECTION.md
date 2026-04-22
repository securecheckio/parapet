# Emerging Threat Detection

Real-time detection of active attacks using Helius Wallet API behavioral analysis.

## Overview

Three new analyzers detect ongoing attacks by analyzing the fee payer's recent transaction history and funding source:

1. **HeliusTransferAnalyzer** - Detects active drains via velocity/pattern analysis
2. **HeliusFundingAnalyzer** - Detects sybil attacks and bot farms
3. **HeliusIdentityAnalyzer** - Existing wallet reputation checking

## Quick Start

### 1. Enable Helius API

```bash
# Set your Helius API key
export HELIUS_API_KEY=your_key_here
```

Get a free API key at [dashboard.helius.dev](https://dashboard.helius.dev)

### 2. Load Emerging Threats Rules

```bash
# Use the emerging-threats preset
export RULES_PATH=parapet/rpc-proxy/rules/presets/emerging-threats.json

# Or combine with other presets
export RULES_PATH=parapet/rpc-proxy/rules/presets/
```

### 3. Run Proxy or Scanner

The analyzers are automatically enabled when `HELIUS_API_KEY` is set.

## Attack Scenarios Detected

### 1. Compromised AI Agent

**Scenario:** AI agent's private key leaked, attacker draining funds

**Detection:**
- High velocity: >10 outgoing transfers/hour
- High concentration: >70% to same address
- **Action:** BLOCK transaction

**Rule:**
```json
{
  "id": "compromised-agent",
  "conditions": {
    "all": [
      {"field": "helius_transfer:is_high_velocity", "operator": "equals", "value": true},
      {"field": "helius_transfer:counterparty_concentration", "operator": "greater_than", "value": 0.7}
    ]
  },
  "action": "block"
}
```

### 2. Active Wallet Drain

**Scenario:** Attacker has access to wallet, rapidly moving funds

**Detection:**
- >10 outgoing transfers/hour to same address
- **Action:** BLOCK transaction

**Rule:**
```json
{
  "id": "active-drain-velocity",
  "conditions": {
    "field": "helius_transfer:is_high_velocity",
    "operator": "equals",
    "value": true
  },
  "action": "block"
}
```

### 3. Phishing Victim

**Scenario:** User repeatedly signing malicious transactions to scammer

**Detection:**
- >80% of transfers go to same address
- **Action:** ALERT user

**Rule:**
```json
{
  "id": "phishing-victim-concentration",
  "conditions": {
    "field": "helius_transfer:counterparty_concentration",
    "operator": "greater_than",
    "value": 0.8
  },
  "action": "alert"
}
```

### 4. Sybil/Bot Wallet

**Scenario:** Coordinated attack using bot wallets

**Detection:**
- Unknown funding source
- Recent funding (<24h)
- Small funding amount (<0.1 SOL)
- **Action:** ALERT

**Rule:**
```json
{
  "id": "sybil-wallet",
  "conditions": {
    "field": "helius_funding:is_likely_sybil",
    "operator": "equals",
    "value": true
  },
  "action": "alert"
}
```

## Available Fields

### HeliusTransferAnalyzer

| Field | Type | Description |
|-------|------|-------------|
| `helius_transfer:outgoing_tx_per_hour` | u32 | Count of outgoing transfers in last hour |
| `helius_transfer:max_transfers_to_same_address` | u32 | Maximum transfers to any single address |
| `helius_transfer:is_high_velocity` | bool | True if >10 tx/hour to same address |
| `helius_transfer:top_counterparty` | string | Most frequent recipient address |
| `helius_transfer:counterparty_concentration` | f32 | Ratio 0.0-1.0 of transfers to top counterparty |

### HeliusFundingAnalyzer

| Field | Type | Description |
|-------|------|-------------|
| `helius_funding:funding_source` | string | Address that originally funded this wallet |
| `helius_funding:funding_source_type` | string | Type: "exchange", "unknown", etc. |
| `helius_funding:funding_risk_score` | u32 | Risk score 0-100 |
| `helius_funding:is_likely_sybil` | bool | Sybil detection flag |
| `helius_funding:funding_age_hours` | u32 | Hours since wallet was funded |

## Performance

- **Latency:** ~150ms per analyzer (single API call each)
- **Caching:** Transfers cached 1 hour, funding cached permanently
- **Rate Limit:** 20 requests/min (shared across all Helius analyzers)
- **Opportunistic:** Only runs if rules reference the analyzer's fields

## Cost Optimization

The analyzers are **opportunistic** - they only run if rules reference their fields:

```json
// This rule triggers HeliusTransferAnalyzer
{
  "conditions": {"field": "helius_transfer:is_high_velocity", "operator": "equals", "value": true}
}

// This rule does NOT trigger any Helius analyzers
{
  "conditions": {"field": "delegation_is_unlimited", "operator": "equals", "value": true}
}
```

**Cost control strategies:**
1. Only enable rules you need
2. Use caching (automatic)
3. Combine multiple Helius fields in single rule (1 API call)
4. Monitor API usage via Helius dashboard

## Example: Protecting AI Agents

Create a rule file for AI agent protection:

```json
[
  {
    "id": "protect-ai-agent",
    "name": "AI Agent Protection",
    "enabled": true,
    "rule": {
      "action": "block",
      "conditions": {
        "any": [
          {
            "field": "helius_transfer:is_high_velocity",
            "operator": "equals",
            "value": true
          },
          {
            "all": [
              {"field": "helius_transfer:counterparty_concentration", "operator": "greater_than", "value": 0.8},
              {"field": "helius_transfer:outgoing_tx_per_hour", "operator": "greater_than", "value": 5}
            ]
          }
        ]
      },
      "message": "🚨 BLOCKED: Suspicious activity pattern detected - possible compromised agent"
    }
  }
]
```

## Testing

Run integration tests (requires HELIUS_API_KEY):

```bash
cd parapet/core
RUN_INTEGRATION_TESTS=1 HELIUS_API_KEY=your_key cargo test --features helius helius_transfer
RUN_INTEGRATION_TESTS=1 HELIUS_API_KEY=your_key cargo test --features helius helius_funding
```

## Limitations

- **Analyzes fee payer only** - Not suitable for enterprise relayer scenarios
- **1-hour window** - May miss slow drains over days/weeks
- **Requires recent activity** - New wallets with no history return empty fields
- **Subject to rate limits** - Helius API has 20 req/min limit
- **Requires HELIUS_API_KEY** - Free tier available, paid tiers for higher volume

## Troubleshooting

### Analyzer not running

Check logs for:
```
💡 HeliusTransferAnalyzer: HELIUS_API_KEY not set - analyzer will be disabled
```

Solution: Set `HELIUS_API_KEY` environment variable

### Rate limit errors

Check logs for:
```
⚠️  Helius API rate limited
```

Solutions:
- Increase `HELIUS_RATE_LIMIT` (default: 20/60)
- Upgrade Helius plan
- Reduce rule usage of Helius fields

### Empty fields returned

This is normal for:
- New wallets with no transaction history
- Wallets that never received SOL (no funding source)
- Wallets with no recent transfers (last 1 hour)

The analyzers gracefully degrade and return empty fields.

## See Also

- [HeliusTransferAnalyzer Documentation](../core/src/rules/analyzers/third_party/helius_transfer.md)
- [HeliusFundingAnalyzer Documentation](../core/src/rules/analyzers/third_party/helius_funding.md)
- [HeliusIdentityAnalyzer Documentation](../core/src/rules/analyzers/third_party/helius_identity.md)
- [Emerging Threats Rules](../rpc-proxy/rules/presets/emerging-threats.json)
