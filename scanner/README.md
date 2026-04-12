# Parapet Wallet Scanner

CLI tool for comprehensive security analysis of Solana wallets.

## Quick Start

```bash
# Basic scan
cargo run --release -p parapet-scanner --bin wallet-scanner -- WALLET_ADDRESS

# With options
cargo run --release -p parapet-scanner --bin wallet-scanner -- WALLET_ADDRESS \
  -t 50 -d 7 --format json

# See all options
cargo run --release -p parapet-scanner --bin wallet-scanner -- --help
```

## What It Scans

1. **Transaction History** - Analyzes recent transactions for suspicious patterns
2. **Token Holdings** - Identifies risky or malicious tokens
3. **On-Chain Behavior** - Detects unusual activity, delegation attacks, drain attempts
4. **Smart Contract Interactions** - Flags unverified or suspicious programs
5. **Risk Scoring** - Provides overall assessment (Safe, Low, Medium, High, Critical)

## CLI Parameters

```bash
wallet-scanner WALLET_ADDRESS [OPTIONS]
```

**Required:**

- `WALLET_ADDRESS` - Base58 Solana public key

**Optional:**

- `-t, --transactions <COUNT>` - Number of transactions to analyze (default: 100, max: 1000)
- `-d, --days <DAYS>` - Days of history to scan (default: 30)
- `--format <FORMAT>` - Output format: `human` (default), `json`, `brief`
- `--network <NETWORK>` - Network: `mainnet-beta` (default), `devnet`, `testnet`
- `--rpc-url <URL>` - Custom RPC endpoint
- `--safe-programs-file <FILE>` - JSON file with trusted program IDs

**Environment:**

- `RULES_PATH` - Custom rule file path (default: auto-discovered from `proxy/rules/presets/`)

## Third-Party Analyzers (Optional)

**Full docs:** `[../core/src/rules/analyzers/third_party/README.md](../core/src/rules/analyzers/third_party/README.md)`

**Quick setup:**

```bash
# Build with analyzers
cargo build --release -p parapet-scanner --features "rugcheck,jupiter,helius,ottersec"

# Set API keys
export HELIUS_API_KEY="your_key"
export OTTERSEC_API_KEY="your_key"

# Run
cargo run --release -p parapet-scanner --bin wallet-scanner -- WALLET_ADDRESS
```

**Rule presets:** Auto-loads from `proxy/rules/presets/` (`wallet-scan-enhanced.json` → `bot-essentials.json` → `default-protection.json`).

## Output Formats

### Human (default)

Detailed report with findings, explanations, and recommendations.

### JSON

```bash
cargo run --release -p parapet-scanner --bin wallet-scanner -- WALLET \
  --format json | jq
```

Structured output for automation. Fields:

- `risk_level` - "safe", "low", "medium", "high", "critical"
- `findings` - Array of detected issues
- `transaction_count` - Number analyzed
- `token_count` - Tokens in wallet
- `summary` - Risk assessment summary

### Brief

```bash
cargo run --release -p parapet-scanner --bin wallet-scanner -- WALLET --format brief
```

Single-line summary: `[RISK_LEVEL] WALLET: X findings in Y transactions`

## Examples

```bash
# Basic scan
cargo run --release -p parapet-scanner --bin wallet-scanner -- \
  9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM

# Quick check (last 7 days, 50 txs)
cargo run --release -p parapet-scanner --bin wallet-scanner -- WALLET -t 50 -d 7

# Deep scan (1000 txs, 90 days)
cargo run --release -p parapet-scanner --bin wallet-scanner -- WALLET -t 1000 -d 90

# JSON output for automation
cargo run --release -p parapet-scanner --bin wallet-scanner -- WALLET \
  --format json | jq '.risk_level, .findings'

# Use custom RPC (recommended for production)
cargo run --release -p parapet-scanner --bin wallet-scanner -- WALLET \
  --rpc-url https://your-rpc-provider.com

# Custom rules
RULES_PATH=./custom-rules.json cargo run --release -p parapet-scanner \
  --bin wallet-scanner -- WALLET
```

## Performance

**Scan time:** 5-30 seconds depending on:

- Transaction count (`-t`)
- RPC endpoint speed
- Enabled analyzers (third-party APIs add latency)

**RPC considerations:**

- Public endpoints: Rate-limited, slower
- Paid RPC (QuickNode, Helius, Triton): Faster, more reliable

**Optimization:**

- Use `-t 50 -d 7` for quick checks
- Use paid RPC for production
- Cache results to avoid repeated scans

## Troubleshooting

**"Rate limit exceeded"**

- Use paid RPC provider
- Reduce scan depth: `-t 50 -d 7`

**"API_KEY not set"**

- Set required keys: `export HELIUS_API_KEY="key"`
- Or build without that feature

**"No rules found"**

- Run from `parapet/` directory
- Or set `RULES_PATH=/absolute/path/to/rules.json`

**"Rules not triggering"**

- Check rule requires analyzers that are enabled
- See `[../core/src/rules/analyzers/third_party/README.md](../core/src/rules/analyzers/third_party/README.md)` for field references

## Exit Codes

- `0` - Scan completed successfully
- `1` - Invalid arguments or configuration error
- `2` - RPC connection failed
- `3` - Rule loading failed
- `4` - Scan failed (transaction fetch errors, etc.)

## Library Usage

```rust
use parapet_scanner::WalletScanner;

let scanner = WalletScanner::new(rpc_client, rule_engine);
let report = scanner.scan_wallet(wallet_pubkey, max_transactions, days).await?;
println!("Risk: {:?}", report.risk_level);
```

See `scanner/src/lib.rs` for full API.