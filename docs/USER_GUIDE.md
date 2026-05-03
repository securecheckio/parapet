# Parapet User Guide

**For:** End users running Parapet tools (scanner, proxy)

## Workflow Overview

```mermaid
graph LR
    A[Choose Tool] --> B{Scanner or Proxy?}
    B -->|Historical Analysis| C[Scanner]
    B -->|Live Protection| D[Proxy]
    
    C --> E[Scan wallet/token]
    E --> F[Review risk report]
    F --> G[Take action based on findings]
    
    D --> H[Start proxy]
    H --> I[Point client to proxy]
    I --> J[Transactions auto-protected]
    J --> K{Risk detected?}
    K -->|Low| L[✅ Transaction passes]
    K -->|High| M[🚫 Transaction blocked]
```

## Quick Start

### Scanner - Analyze Wallet Transactions

Scan a wallet's transaction history for security risks:

```bash
# Scan wallet
cargo run -p parapet-scanner -- \
  --wallet 7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU \
  --rpc https://api.mainnet-beta.solana.com

# Save results
cargo run -p parapet-scanner -- \
  --wallet YOUR_WALLET \
  --output scan-results.json
```

**Output shows:**
- Risk score for each transaction
- Security warnings and matched rules
- Summary statistics

### Proxy - Protect Live Transactions

Run the RPC proxy to protect transactions in real-time:

```bash
# Recommended: TOML (see rpc-proxy/config.toml.example)
cp rpc-proxy/config.toml.example rpc-proxy/config.toml
# Edit [upstream] then:
cargo run -p parapet-rpc-proxy --bin parapet-rpc-proxy

# CLI override (single URL)
cargo run -p parapet-rpc-proxy -- \
  --upstream-rpc https://api.mainnet-beta.solana.com \
  --port 8899

# Or environment variables (overrides TOML when set)
export UPSTREAM_RPC_URL=https://api.mainnet-beta.solana.com
# Multi-endpoint failover (comma-separated):
# export UPSTREAM_RPC_URLS=https://primary.example.com,https://backup.example.com
# Optional: UPSTREAM_STRATEGY=smart  UPSTREAM_SMART_MAX_SLOT_LAG=20
cargo run -p parapet-rpc-proxy --bin parapet-rpc-proxy
```

Then point your Solana client to `http://localhost:8899`.

For **multi-upstream**, method allow/block lists, and production notes, see [Operations Guide — Multi-upstream RPC](OPERATIONS_GUIDE.md#multi-upstream-rpc-proxy-and-api) and [rpc-proxy/README.md](../rpc-proxy/README.md#upstream-rpc-multi-url-and-method-policy).

## Configuration

Prefer **`rpc-proxy/config.toml`** (copy from `config.toml.example`) so upstream URLs, retries, circuit breakers, and optional **`[security].allowed_methods` / `blocked_methods`** live in one place. Use environment variables for secrets and for **overrides** in containers.

### Environment Variables (proxy)

```bash
# Upstream: set ONE of these (not both)
UPSTREAM_RPC_URL=https://api.mainnet-beta.solana.com
# UPSTREAM_RPC_URLS=https://a.example.com,https://b.example.com

# Optional
PROXY_PORT=8899
DEFAULT_BLOCKING_THRESHOLD=70     # 0–255 risk threshold (see rpc-proxy config reference)
REDIS_URL=redis://localhost:6379  # For caching / usage / escalations when enabled
ALLOWED_RPC_METHODS=getHealth,getAccountInfo   # Optional allowlist (comma-separated)
BLOCKED_RPC_METHODS=sendTransaction            # Optional blocklist
RUST_LOG=info
```

### Rules Configuration

Customize which security rules are active:

```bash
# Use preset (specify path to preset file)
cargo run -p parapet-rpc-proxy -- --rules rpc-proxy/rules/presets/default-protection.json

# Or custom rules file
cargo run -p parapet-rpc-proxy -- --rules rpc-proxy/rules/custom-rules.json
```

**Available Presets:**
- `default-protection.json` - Balanced security and usability
- `bot-essentials.json` - Essential protection for automated bots
- `wallet-scan-enhanced.json` - Enhanced scanning for wallet analysis

### Custom Rules

Copy and edit a preset:

```bash
cp rpc-proxy/rules/presets/default-protection.json rpc-proxy/rules/my-rules.json
# Edit my-rules.json to adjust weights and thresholds
cargo run -p parapet-rpc-proxy -- --rules rpc-proxy/rules/my-rules.json
```

## Understanding Risk Scores

Risk scores range from 0-100:
- **0-30**: Low risk (safe)
- **31-60**: Medium risk (warnings)
- **61-100**: High risk (may be blocked)

Transactions are blocked when `risk_score >= threshold` (default: 70).

## Common Use Cases

### Protecting a Bot Wallet

```bash
# Run proxy with bot-essentials rules
export UPSTREAM_RPC_URL=https://api.mainnet-beta.solana.com
export DEFAULT_BLOCKING_THRESHOLD=60  # Block more aggressively
cargo run -p parapet-rpc-proxy -- --rules rpc-proxy/rules/presets/bot-essentials.json

# Configure bot to use http://localhost:8899
```

### Scanning Before Trading

```bash
# Scan a token's recent transactions
cargo run -p parapet-scanner -- \
  --token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --limit 100
```

### Monitoring Wallet Activity

```bash
# Continuous monitoring (checks every 60s)
cargo run -p parapet-scanner -- \
  --wallet YOUR_WALLET \
  --watch \
  --interval 60
```

## Checking Logs

```bash
# View proxy logs
RUST_LOG=info cargo run -p parapet-rpc-proxy

# Debug mode (verbose)
RUST_LOG=debug cargo run -p parapet-rpc-proxy

# Save logs to file
cargo run -p parapet-rpc-proxy 2>&1 | tee parapet.log
```

**Look for:**
- `✅ Transaction PASSED` - Safe transaction
- `⚠️  Transaction has warnings` - Medium risk
- `🚫 Transaction BLOCKED` - High risk, blocked

## Troubleshooting

### Proxy won't start
- Check **`UPSTREAM_RPC_URL`** or **`UPSTREAM_RPC_URLS`** is set (env-only), or `[upstream]` is filled in `config.toml`
- Verify port 8899 is not in use: `lsof -i :8899`

### Too many false positives
- Increase threshold: `DEFAULT_BLOCKING_THRESHOLD=80`
- Use a less restrictive preset or create custom rules

### Missing Redis errors
- Redis is optional for caching
- Set `REDIS_URL` or ignore warnings

## Getting Help

- Check logs with `RUST_LOG=debug`
- See `rpc-proxy/README.md` for more proxy options
- See `scanner/README.md` for more scanner options
