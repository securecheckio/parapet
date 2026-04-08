# Parapet Wallet Scanner

Scan Solana wallets for security threats and compromises.

## What It Does

The wallet scanner analyzes a Solana wallet address for:

### Fast Mode (Default)

By default, scans **only active threats** - what can hurt you RIGHT NOW:
- ✅ Unlimited token delegations (u64::MAX)
- ✅ Active malicious program interactions
- ✅ Compromised authority controls
- ✅ Fast (< 1 second)
- ✅ Minimal RPC calls

**Use this for:** Quick security checks, monitoring, CI/CD

### Deep Mode (--enable-history)

With `--enable-history`, also scans **historical threats**:
- ✅ All fast mode checks PLUS
- ✅ Previously granted delegations that may have been exploited
- ✅ Interactions with blacklisted programs
- ✅ Unusual transaction patterns
- ⚠️ Slower (depends on transaction count)
- ⚠️ More RPC calls (may hit rate limits)

**Use this for:** Thorough audits, investigating suspicious activity

### Emerging Threat Detection (with HELIUS_API_KEY)

When `HELIUS_API_KEY` is set, scanner also detects **active attacks in progress**:
- ✅ **Active wallet drains** - Rapid outgoing transfers (>10 tx/hour)
- ✅ **Compromised AI agents** - High velocity + concentration patterns
- ✅ **Phishing victims** - Repeated transfers to same scammer
- ✅ **Sybil/bot wallets** - Suspicious funding sources
- ⚠️ Requires Helius API key (free tier available)

**Use this for:** Detecting ongoing attacks, protecting AI agents, real-time monitoring

### Security Scoring

Overall wallet health (0-100):
- **SAFE (91-100):** No threats detected ✅
- **LOW RISK (76-90):** Minor concerns
- **MEDIUM RISK (51-75):** Some issues found ⚠️
- **HIGH RISK (31-50):** Multiple security concerns ⚠️
- **CRITICAL (0-30):** Signs of compromise 🚨

## Installation

### CLI Tool (Rust)

Build the command-line tool:

```bash
cd parapet/scanner
cargo build --release --bin wallet-scanner
```

The binary will be at `target/release/wallet-scanner`

### Bash Script (Quick Use)

The bash script is ready to use immediately:

```bash
cd saas
./check-wallet.sh <wallet_address> -k <api_key>
```

## Usage

### Option 1: Rust CLI Tool (Standalone)

The CLI tool connects directly to Solana RPC - no API key needed!

```bash
# Basic scan (mainnet, last 100 transactions)
./wallet-scanner 9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin

# Scan devnet wallet
./wallet-scanner YOUR_WALLET --network devnet --rpc-url https://api.devnet.solana.com

# Enable historical analysis (checks past transactions)
./wallet-scanner YOUR_WALLET --enable-history

# Deep historical scan (200 transactions, 60 days)
./wallet-scanner YOUR_WALLET --enable-history -t 200 -d 60

# JSON output (for scripts)
./wallet-scanner YOUR_WALLET --format json

# Brief output (for CI/CD)
./wallet-scanner YOUR_WALLET --format brief
```

#### CLI Options

```
OPTIONS:
  -r, --rpc-url URL            Solana RPC endpoint (default: mainnet)
  -t, --max-transactions NUM   Max transactions to analyze (default: 100)
  -d, --time-window-days NUM   Time window in days (default: 30)
  -n, --network NAME           Network: mainnet-beta, devnet, testnet
  -f, --format FORMAT          Output: pretty, json, brief (default: pretty)
      --enable-history         Enable historical transaction analysis
  -h, --help                   Show help
  
Note: By default, the scanner only checks active threats (current delegations,
authorities) for speed. Use --enable-history to analyze past transactions.
```

### Option 2: Bash Script (Via API)

Uses the SecureCheck API endpoint - requires API key:

```bash
# Local development
./check-wallet.sh 9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin \
  -k your_api_key

# Production API
./check-wallet.sh YOUR_WALLET \
  -k your_api_key \
  -u https://api.securecheck.io \
  -t 200 \
  -d 60

# JSON output
./check-wallet.sh YOUR_WALLET -k your_api_key --json
```

#### Bash Script Options

```
OPTIONS:
  -k, --api-key KEY    API key (required)
  -u, --url URL        API endpoint (default: http://localhost:3001)
  -t, --max-tx NUM     Max transactions (default: 100)
  -d, --days NUM       Time window days (default: 30)
  -j, --json           Output JSON
  -h, --help           Show help
```

## Output Examples

### Pretty Output (Default)

```
═══════════════════════════════════════════════════════════
            Parapet Wallet Security Scanner
═══════════════════════════════════════════════════════════

🔍 Scanning wallet: 9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin
🌐 Network: mainnet-beta
📊 Analyzing last 30 days (100 transactions max)

⏳ Scanning ✓ (1234ms)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  SECURITY ASSESSMENT
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  ✅ Security Score: 95 / 100
  Risk Level: SAFE

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  SCAN STATISTICS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  📅 Time Range: 30 days
  📝 Transactions Analyzed: 87
  ⚠️  Total Threats Found: 0

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  RECOMMENDATION
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  ✅ SAFE: No security threats detected

  Your wallet appears secure. Continue best practices:
    • Only connect to trusted dApps
    • Review transactions before signing
    • Monitor for unexpected activity
```

### Compromised Wallet Example

```
  🚨 Security Score: 25 / 100
  Risk Level: CRITICAL

  ⚠️  Total Threats Found: 3

  Threat Breakdown:
    2 Critical
    1 High

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  DETECTED THREATS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  [1] 🚨 Critical
     Type: Active Unlimited Delegation
     Token: 7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU
     Delegate: AbcD...xyz
     📌 Action: REVOKE THIS DELEGATION IMMEDIATELY

  [2] 🚨 Critical
     Type: Possibly Exploited Delegation
     Token: 8yYt...123
     Delegate: Def9...456
     📌 Action: Check transaction history for unauthorized transfers

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  RECOMMENDATION
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  🚨 CRITICAL: This wallet shows signs of compromise!

  Immediate Actions:
    1. Stop using this wallet immediately
    2. Create a new wallet with a new seed phrase
    3. Transfer remaining funds to the new wallet
    4. Revoke all token delegations
    5. Review how the compromise occurred
```

### JSON Output

```json
{
  "wallet": "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin",
  "scanned_at": "2026-03-30T15:30:00Z",
  "security_score": 95,
  "risk_level": "SAFE",
  "threats": [],
  "suspicious_programs": [],
  "stats": {
    "transactions_analyzed": 87,
    "time_range_days": 30,
    "threats_found": 0,
    "critical_count": 0,
    "high_count": 0,
    "medium_count": 0,
    "low_count": 0,
    "scan_duration_ms": 1234
  }
}
```

### Brief Output

```
✅ 9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin - Security Score: 95/100 - Risk: SAFE
```

## Common Use Cases

### 1. Check Your Own Wallet

```bash
# Quick check
./wallet-scanner YOUR_WALLET_ADDRESS

# Thorough check (more history)
./wallet-scanner YOUR_WALLET_ADDRESS -t 500 -d 90
```

### 2. Audit a New Wallet Before Transfer

```bash
# Check wallet before sending funds
./wallet-scanner RECIPIENT_WALLET --format brief
```

### 3. Monitor Wallets (CI/CD)

```bash
#!/bin/bash
# monitor-wallets.sh

WALLETS=(
  "wallet1..."
  "wallet2..."
  "wallet3..."
)

for wallet in "${WALLETS[@]}"; do
  ./wallet-scanner "$wallet" --format brief
  if [ $? -ne 0 ]; then
    echo "⚠️  Wallet $wallet has security issues!"
    # Send alert (email, Slack, etc.)
  fi
done
```

### 4. Integration with Scripts

```bash
# Get security score programmatically
SCORE=$(./wallet-scanner YOUR_WALLET --format json | jq -r '.security_score')

if [ "$SCORE" -lt 50 ]; then
  echo "⚠️  Wallet compromised! Score: $SCORE"
  exit 1
fi
```

## Understanding the Results

### Security Score Ranges

| Score   | Risk Level | Meaning                           |
|---------|-----------|-----------------------------------|
| 91-100  | SAFE      | No threats, wallet is secure      |
| 76-90   | LOW       | Minor concerns, monitor closely   |
| 51-75   | MEDIUM    | Some issues, review & address     |
| 31-50   | HIGH      | Multiple concerns, take action    |
| 0-30    | CRITICAL  | Compromised, migrate immediately  |

### Threat Types

1. **Active Unlimited Delegation**
   - Token approval set to maximum amount (u64::MAX)
   - Can be exploited right now to drain tokens
   - ACTION: Revoke immediately

2. **Possible Exploited Delegation**
   - Delegation was granted but is now missing
   - May indicate funds were already stolen
   - ACTION: Review transaction history

3. **Compromised Authority**
   - Account ownership/authority has changed
   - Wallet may no longer control assets
   - ACTION: Investigate and potentially migrate

4. **Suspicious Transaction**
   - Transaction flagged by security rules
   - May be malicious program interaction
   - ACTION: Review transaction details

5. **Unusual Pattern**
   - Atypical activity detected
   - Could be normal or suspicious
   - ACTION: Verify activity is legitimate

## How It Works

### 1. Active State Scan

Checks current wallet state for:
- Token account delegations
- Authority controls
- Active vulnerabilities

### 2. Historical Transaction Scan (Optional)

Analyzes past transactions for:
- Suspicious program interactions
- Blacklisted programs
- Rule violations
- Unusual patterns

### 3. Threat Correlation

Combines active + historical data to identify:
- Exploited delegations (granted but now missing)
- Recurring suspicious programs
- Risk patterns

### 4. Security Scoring

Calculates score based on:
- Critical threats: -50 points each
- High threats: -20 points each
- Medium threats: -5 points each
- Low threats: -1 point each

## Limitations

1. **RPC Rate Limits**: Scanning many transactions may hit rate limits
2. **Historical Depth**: Only scans recent history (configurable)
3. **False Positives**: Some legitimate transactions may be flagged
4. **Active State Only**: Fast mode only checks current state, not history

## Troubleshooting

### Error: RPC rate limit exceeded

```bash
# Use a private RPC endpoint
./wallet-scanner YOUR_WALLET --rpc-url https://your-private-rpc.com

# Or reduce transaction count
./wallet-scanner YOUR_WALLET -t 50
```

### Error: Wallet not found

```bash
# Check network (devnet vs mainnet)
./wallet-scanner YOUR_WALLET --network devnet

# Verify wallet address is correct
```

### Slow scanning

```bash
# Skip historical analysis for faster scan
./wallet-scanner YOUR_WALLET --skip-history

# Reduce transaction count
./wallet-scanner YOUR_WALLET -t 50 -d 7
```

## API Integration

### Using in Your App

```rust
use sol_shield_scanner::{WalletScanner, ScanConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let scanner = WalletScanner::new(
        "https://api.mainnet-beta.solana.com".to_string()
    )?;
    
    let config = ScanConfig {
        max_transactions: Some(100),
        time_window_days: Some(30),
        check_active_threats: true,
        check_historical: false,  // Fast mode
        commitment: Default::default(),
    };
    
    let report = scanner.scan(
        "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin",
        config
    ).await?;
    
    println!("Security Score: {}/100", report.security_score);
    println!("Threats Found: {}", report.threats.len());
    
    Ok(())
}
```

## Contributing

Found an issue or want to improve the scanner? See [CONTRIBUTING.md](../../CONTRIBUTING.md)

## Support

- Issues: [GitHub Issues](https://github.com/securecheckio/parapet/issues)
- Docs: [Parapet Documentation](../../docs/)
- Security: security@securecheck.io

---

**Built with Rust 🦀 for the Solana ecosystem**
