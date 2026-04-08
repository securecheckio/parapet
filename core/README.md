# Parapet

**Fast, portable Solana transaction security library with flexible rules engine**

## Overview

Parapet is a Rust library for analyzing Solana transactions in real-time. It combines fast pattern detection (<50ms) with a flexible rules engine and pluggable analyzers.

## Features

- ⚡ **Fast Pattern Detection** - Sub-50ms analysis, no RPC calls
- 🔌 **Pluggable Analyzers** - Extend with custom verification layers
- 📋 **Rules Engine** - JSON-configurable security policies
- 🌐 **Optional External APIs** - Helius Identity, OtterSec verification
- 🦀 **Pure Rust** - Safe, fast, portable
- 📦 **Library-First** - Use anywhere (mobile, CLI, server)

## Installation

```toml
[dependencies]
# Basic security (fast, local only)
parapet = "0.1"

# With external verification (Helius, OtterSec, security.txt)
parapet = { version = "0.1", features = ["all-analyzers"] }

# Or pick specific analyzers
parapet = { version = "0.1", features = ["helius", "ottersec"] }
```

## Available Features

| Feature | Description | Requires |
|---------|-------------|----------|
| `helius` | Helius Identity API (wallet reputation) | HELIUS_API_KEY |
| `security-txt` | RFC 9116 security disclosure | None |
| `ottersec` | OtterSec cryptographic verification | None |
| `all-analyzers` | Enable all external analyzers | HELIUS_API_KEY |

## Quick Start

### Basic Usage (Fast, Local)

```rust
use sol_shield::{SecurityEngine, RiskLevel, EngineConfig};
use std::collections::HashSet;

// Create engine with blocklist
let blocklist = HashSet::new(); // Add known malicious programs
let engine = SecurityEngine::new(blocklist, EngineConfig::default());

// Analyze transaction
let result = engine.analyze(&transaction);

if result.risk_level == RiskLevel::Critical {
    println!("🚨 Blocked: {:?}", result.issues);
}
```

### Using Rules Engine

```rust
use sol_shield::rules::{RuleEngine, analyzers::*};

// Create rule engine
let mut engine = RuleEngine::new();

// Register analyzers
engine.register_analyzer(Arc::new(BasicAnalyzer::new()));
engine.register_analyzer(Arc::new(SecurityAnalyzer::new(security_engine)));

// Load rules from JSON
engine.load_rules_from_file("./rules.json")?;

// Evaluate transaction
let decision = engine.evaluate(&transaction)?;
```

### With External Analyzers

```rust
use sol_shield::rules::{RuleEngine, analyzers::*};

let mut engine = RuleEngine::new();

// Built-in analyzers
engine.register_analyzer(Arc::new(BasicAnalyzer::new()));

// External analyzers (requires features)
#[cfg(feature = "helius")]
engine.register_analyzer(Arc::new(HeliusIdentityAnalyzer::new()));

#[cfg(feature = "ottersec")]
engine.register_analyzer(Arc::new(OtterSecVerifiedAnalyzer::new()));

engine.load_rules_from_file("./comprehensive-rules.json")?;
```

## Available Analyzers

### Built-in (Always Available)

#### BasicAnalyzer
Transaction metrics:
- `instruction_count`, `account_keys_count`, `amount`, `program_ids`

#### SecurityAnalyzer
Pattern detection (wraps SecurityEngine):
- `risk_score`, `risk_level`, `delegation_detected`, `delegation_is_unlimited`
- `delegation_count`, `authority_changes`, `blocklisted_program_detected`

### Optional (Require Features)

#### HeliusIdentityAnalyzer (feature="helius")
Wallet reputation via Helius Identity API:
- `signer_classifications`, `other_classifications`
- Detects: Scammers, Hackers, Ruggers, Exploiters
- Coverage: 5100+ tagged accounts

#### SecurityTxtAnalyzer (feature="security-txt")
RFC 9116 security disclosure verification:
- `programs_with_security_txt`, `all_programs_verified`
- Validates programs have /.well-known/security.txt

#### OtterSecVerifiedAnalyzer (feature="ottersec")
Cryptographic source code verification:
- `programs_verified`, `all_programs_verified`, `repo_urls`
- Reproducible builds, source attribution
- API: https://verify.osec.io

## Rules Engine

Define security policies in JSON:

```json
{
  "version": "1.0",
  "id": "block-scammers",
  "enabled": true,
  "rule": {
    "action": "block",
    "conditions": {
      "field": "helius_identity:other_classifications",
      "operator": "in",
      "value": ["Scammer", "Hacker"]
    },
    "message": "🚨 Transaction involves known malicious address"
  }
}
```

## Performance

### Pattern Detection (Built-in)
- Analysis: <50ms
- No network calls
- Zero external dependencies

### With External Analyzers
- Helius Identity: ~150ms (cached: <1ms)
- Security.txt: ~100ms (cached: <1ms)
- OtterSec: ~150ms (cached: <1ms)

All analyzers cache results to minimize latency.

## Use Cases

### Additional Security Layer for Solana Users

Parapet provides RPC-level transaction verification for users requiring enhanced security.

#### Mobile Wallet Developers
```toml
parapet = "0.1"  # Fast, local analysis (no external APIs)
```
**Provides:**
- Pattern detection for unlimited approvals and authority changes
- Sub-50ms analysis overhead
- No network dependencies for core security checks

#### Trading Bot Operators
```toml
parapet = { version = "0.1", features = ["ottersec"] }
```
**Enables:**
- Program allowlist enforcement (verified programs only)
- Configurable JSON security policies
- Programmatic access to structured risk data

#### RPC Providers
```toml
parapet = { version = "0.1", features = ["all-analyzers"] }
```
**Includes:**
- Helius address reputation checking
- OtterSec program verification
- security.txt validation per RFC 9116

## Documentation

- [Wallet Integration Guide](../WALLET_INTEGRATION.md) — custom RPC in wallets
- [Proxy](../proxy/README.md) — rules path, config, run
- Analyzer code: `core/src/rules/analyzers/`

## Examples

See the [integrations/](../integrations/) directory for integration examples and the [docs/](../docs/) directory for usage guides.

## Development

```bash
# Run tests
cargo test

# Build with all features
cargo build --features all-analyzers

# Check code
cargo clippy
```

## License

MIT

## Support

- Issues: https://github.com/securecheckio/parapet/issues
- Docs: See documentation files above
