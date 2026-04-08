# Parapet Proxy

RPC-level Solana transaction security with configurable rules and detailed threat analysis.

## Purpose

Additional security verification layer for users with significant holdings. Provides:

- **Detailed threat information**: Specific warnings (e.g., "Unlimited SPL approval to program X") vs generic errors
- **Configurable policies**: JSON security rules users can audit and customize
- **Multiple data sources**: Helius identity database, OtterSec program verification, pattern analysis
- **Independent layer**: Security verification outside wallet provider control

## Features

- **Rule-Based Protection**: Customizable transaction filtering with pluggable analyzers
- **Fast Analysis**: Sub-100ms pattern matching for common drain attacks
- **Delegation Detection**: Identifies unlimited token approvals (u64::MAX)
- **Blocklist Checking**: Blocks transactions with known malicious programs
- **Authority Detection**: Flags unauthorized authority changes
- **Pattern Analysis**: Detects suspicious instruction sequences
- **Rate Limiting**: Optional per-wallet rate limiting (configurable monthly limits)
- **Custom Rules**: Create your own rules or download from marketplace

## Architecture

**Primary Deployment**: Public server running as standalone Rust binary

The RPC proxy is designed to run on public servers as a high-performance reverse proxy. The core architecture is platform-agnostic with:

- **Axum HTTP server** for JSON-RPC handling
- **Parapet** for transaction analysis (separate crate)
- **Redis** for blocklist caching and rate limiting (optional, falls back to in-memory)

### Platform Support

- **Linux/macOS/Windows**: Primary platform - runs as standalone binary
- **Android**: Optional support via JNI FFI layer (see `../android-app/rpc-proxy-android-ffi/`)

The Android FFI is implemented as a separate crate that depends on `rpc-proxy` as a library, keeping platform-specific code isolated.

## Quick Start

### Development (Local Testing)

```bash
# Copy example config
cp .env.example .env
# Edit .env with your settings

# Run locally
cargo run --release
```

The proxy will listen on `http://localhost:8899`

### Production Deployment

**Choose your deployment method:**

1. **Automated Cloud Deployment** (Recommended)
  - See `../deployment/terraform/README.md`
  - One-command deploy to DigitalOcean
  - Automatic HTTPS, firewall, monitoring
2. **Manual Bare Metal Installation**
  - See `BARE_METAL.md` for complete guide
  - Step-by-step systemd setup
  - Security hardening included
3. **Docker Deployment**
  - See `../deployment/docker/README.md`
  - Container-based deployment
  - Easy updates and portability

## Usage

### Supported Methods

The proxy currently supports **HTTP JSON-RPC methods only**:

**✅ Fully Supported:**

- `sendTransaction` / `sendRawTransaction` (with security analysis)
- `simulateTransaction` (with security analysis)
- All read-only methods (`getAccountInfo`, `getBalance`, `getTransaction`, etc.)
- Block/slot queries
- Program queries

**❌ Not Yet Supported:**

- WebSocket subscriptions (`accountSubscribe`, `signatureSubscribe`, `logsSubscribe`, etc.)
- Real-time notifications

**Workaround for WebSocket needs:** Use this proxy for transaction submission (to get security benefits) and connect directly to a standard RPC endpoint for subscriptions.

### For Wallet Users

Update your RPC endpoint to:

```
http://your-proxy-server.com:8899
```

**Note:** If your wallet requires WebSocket support for real-time balance updates, you may need to configure a separate WebSocket endpoint or use polling-based updates.

### For Developers

Point your application's Solana connection to the proxy:

```typescript
import { Connection } from '@solana/web3.js';

// For transaction submission with security analysis
const connection = new Connection('http://your-proxy-server.com:8899');

// For WebSocket subscriptions (if needed), use a standard RPC
const wsConnection = new Connection('https://api.mainnet-beta.solana.com', 'confirmed');
```

### For dApp Developers

```javascript
// Use proxy for transaction submission
const wallet = new SolanaWalletAdapter({
  rpcUrl: 'https://your-proxy-server.com:8899'
});

// Note: If your dApp uses WebSocket subscriptions for real-time updates,
// you'll need to configure a separate connection for those features
```

## Rules Engine

The proxy uses a powerful rule-based system for transaction filtering. Instead of hardcoded logic, you can customize protection with JSON rules.

### Quick Start

**Use default rules (recommended for bots):**

```bash
RULES_PATH=./rules/bot-essentials.json
```

**Or create custom rules:**

```bash
cp rules/custom-example.json rules/my-rules.json
# Edit my-rules.json
RULES_PATH=./rules/my-rules.json
```

### Available Rule Sets

- `bot-essentials.json` - Essential security for trading bots (blocks unlimited delegations, blocklisted programs, critical risks)
- `spending-limits.json` - Additional spending controls (max trade size, complexity warnings)
- `custom-example.json` - Template for creating your own rules

### Example Rule

```json
{
  "id": "unlimited-delegation",
  "name": "Block Unlimited Delegations",
  "enabled": true,
  "rule": {
    "action": "block",
    "conditions": {
      "field": "delegation_is_unlimited",
      "operator": "equals",
      "value": true
    },
    "message": "🚨 BLOCKED: Unlimited token delegation detected"
  }
}
```

**For complete documentation**, see [RULES.md](RULES.md)

## How It Works

1. **Intercept**: Proxy intercepts `sendTransaction` and `sendRawTransaction` calls
2. **Analyze**: Rule engine evaluates transaction in <5ms:
  - Extract fields using analyzers (basic, security, helius_identity, ottersec)
  - Evaluate rules in order
  - Block, alert, or pass based on matched rules
3. **Respond**: Either blocks transaction with error or forwards to upstream
4. **Forward**: Safe transactions forwarded to upstream RPC

### Available Analyzers

#### Core Analyzers (Always Active)

- **Basic Analyzer**: Transaction metrics (instruction count, accounts, etc.)
- **Security Analyzer**: Pattern analysis, delegations, risk scores
- **Token Instructions**: SPL token operations (approve, transfer, etc.)
- **System Program**: SOL transfers and account operations

#### Third-Party Analyzers (Require API Keys)

- **Helius Identity Analyzer**: Wallet reputation (scammers, hackers, exchanges)
- **Helius Transfer Analyzer**: Active drain detection, velocity patterns (NEW)
- **Helius Funding Analyzer**: Sybil detection, bot farm identification (NEW)
- **OtterSec Verified Analyzer**: Cryptographic source code verification
- **Jupiter Token Analyzer**: Token safety and metadata
- **Rugcheck Analyzer**: Scam/rugpull detection (FREE)

## Detection Patterns

### Unlimited Delegation (CRITICAL)

```rust
// SPL Token Approve instruction with u64::MAX amount
if instruction.data[0] == 4 && amount == u64::MAX {
    return Risk::Critical("Unlimited delegation");
}
```

### Multiple Delegations (HIGH)

```rust
// 3+ approve instructions in single transaction
if approve_count >= 3 {
    return Risk::High("Multiple delegations");
}
```

### Authority Changes (MEDIUM-HIGH)

```rust
// SetAuthority instruction detected
if instruction.data[0] == 6 {
    return Risk::MediumHigh("Authority change");
}
```

## Blocklist Management

Add programs to blocklist via Redis:

```bash
# Add to blocklist
redis-cli SET "blocklist:PROGRAM_ID" "1"

# Add to allowlist
redis-cli SET "allowlist:PROGRAM_ID" "1"

# Check if blocklisted
redis-cli EXISTS "blocklist:PROGRAM_ID"
```

## Performance

- **Response Time**: p50 < 50ms, p99 < 100ms
- **Memory**: ~10MB idle
- **Throughput**: 1000+ req/sec on single core
- **Startup**: <1 second

## Security

- **No transaction modification**: Proxy only reads and forwards
- **No key storage**: Never handles private keys
- **Fail-open**: On error, forwards transaction (configurable)
- **Audit logs**: All blocked transactions logged

## Monitoring

Health check endpoint:

```bash
curl http://localhost:8899/health
```

Metrics (if enabled):

```bash
curl http://localhost:8899/metrics
```

## Testing

### Integration Tests

```bash
cargo test
```

### Devnet Testing

Test with real Solana transactions on devnet:

```bash
./test-devnet.sh
```

## Development

### Run with debug logging

```bash
RUST_LOG=debug cargo run
```

### Build for production (Linux server)

```bash
cargo build --release --target x86_64-unknown-linux-gnu
```

### Build for Android (optional)

```bash
# Use the Android build script (builds the FFI wrapper)
cd ../android-app
./build-rust.sh
```

Android FFI code lives in `../android-app/rpc-proxy-android-ffi/`

## Deployment

### Docker

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/rpc-proxy /usr/local/bin/
CMD ["rpc-proxy"]
```

### Systemd Service

```ini
[Unit]
Description=SecureCheck RPC Proxy
After=network.target

[Service]
Type=simple
User=securecheck
WorkingDirectory=/opt/securecheck
EnvironmentFile=/opt/securecheck/.env
ExecStart=/opt/securecheck/rpc-proxy
Restart=always

[Install]
WantedBy=multi-user.target
```

## Roadmap

- WebSocket support (for real-time subscriptions and notifications)
- Token-2022 extensions (permanent delegate, transfer hooks, metadata pointer)
- Permanent delegate detection
- Transfer hook analysis
- Historical pattern learning
- Browser extension
- Wallet SDK

**Current Focus:** HTTP JSON-RPC transaction security. WebSocket support will be added based on user demand.

## License

MIT

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md)

## Rate Limiting (Optional)

Enable per-wallet rate limiting by setting in `.env`:

```bash
ENABLE_RATE_LIMITING=true
DEFAULT_REQUESTS_PER_MONTH=10000  # Default limit per wallet
```

Rate limiting requires Redis. When enabled:

- Each wallet address gets a monthly request counter
- Counters reset automatically after 30 days
- You can set custom limits for specific wallets via Redis:

```bash
# Set custom limit for a specific wallet (e.g., 100k/month)
redis-cli SET "limit:WALLET_ADDRESS" 100000

# Remove custom limit (reverts to default)
redis-cli DEL "limit:WALLET_ADDRESS"
```

## Wallet Allowlisting (Optional)

Restrict access to specific wallet addresses by setting in `.env`:

```bash
# Comma-separated list of allowed wallet addresses
ALLOWLISTED_WALLETS=wallet1,wallet2,wallet3
```

**Behavior:**

- **No allowlist (default)**: All wallets are allowed ✅ (inclusive by default)
- **Allowlist set**: Only listed wallets can use the RPC endpoint
- **Non-allowlisted wallet**: Returns `403 Forbidden` with error code `-32003`

**Use cases:**

- Private RPC for your own wallets only
- Team/organization-specific endpoint
- Testing environment with controlled access
- Beta access for specific users

**Example:**

```bash
# Allow only these 3 wallets
ALLOWLISTED_WALLETS=7a8b9c...,3d4e5f...,9g8h7i...

# Empty or not set = all wallets allowed
# ALLOWLISTED_WALLETS=
```

## Documentation

- [Protocol Support](PROTOCOL_SUPPORT.md) - Supported RPC methods and WebSocket status
- [Wallet Integration Guide](../WALLET_INTEGRATION.md) - Support custom RPC in wallets
- [Rules Engine](RULES.md) - Security rule configuration
- [Analyzer Guide](ANALYZER_GUIDE.md) - Available analyzers and fields
- [Quick Start](QUICK_START.md) - Getting started guide

## Support

- GitHub: [https://github.com/securecheckio/parapet](https://github.com/securecheckio/parapet)
- Issues: [https://github.com/securecheckio/parapet/issues](https://github.com/securecheckio/parapet/issues)

