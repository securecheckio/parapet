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
# Path when developing in the Parapet workspace; use a crates.io version when published.
parapet-core = { path = "../core", features = ["helius", "ottersec"] }
```

## Available features

| Feature | Description | Requires |
|---------|-------------|----------|
| `helius` | Helius Identity API (wallet reputation) | `HELIUS_API_KEY` |
| `ottersec` | OtterSec cryptographic verification | network |
| `all-analyzers` | Umbrella for optional analyzers | per-analyzer keys where applicable |

## Quick start

### Rules engine (recommended)

```rust
use parapet_core::rules::analyzers::{BasicAnalyzer, CoreSecurityAnalyzer};
use parapet_core::rules::{AnalyzerRegistry, RuleEngine};
use std::collections::HashSet;
use std::sync::Arc;

let mut registry = AnalyzerRegistry::new();
registry.register(Arc::new(BasicAnalyzer::new()));
registry.register(Arc::new(CoreSecurityAnalyzer::new(HashSet::new())));

let mut engine = RuleEngine::new(registry);
engine.load_rules_from_file("./rules/default.json").expect("rules");

// let decision = engine.evaluate(&transaction).await.expect("eval");
```

### With optional analyzers

Enable Cargo features (e.g. `helius`, `ottersec`) and register the corresponding analyzers from
`parapet_core::rules::analyzers::third_party` — see crate source and `docs/RULES_*.md` for field names.

## Available Analyzers

### Built-in (Always Available)

#### BasicAnalyzer
Transaction metrics:
- `instruction_count`, `account_keys_count`, `amount`, `program_ids`

#### CoreSecurityAnalyzer
Pattern detection and structural risk signals (see `core_security` analyzer fields in source).

### Optional (Require Features)

#### HeliusIdentityAnalyzer (feature="helius")
Wallet reputation via Helius Identity API:
- `signer_classifications`, `other_classifications`
- Detects: Scammers, Hackers, Ruggers, Exploiters
- Coverage: 5100+ tagged accounts

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

### With external analyzers
- Helius Identity: ~150ms (cached: <1ms when applicable)
- OtterSec: ~150ms (cached: <1ms when applicable)

All analyzers cache results to minimize latency.

## Use Cases

### Additional Security Layer for Solana Users

Parapet provides RPC-level transaction verification for users requiring enhanced security.

#### Mobile wallet developers
```toml
parapet-core = { path = "../core", default-features = true }
```
**Provides:**
- Pattern detection for unlimited approvals and authority changes
- Sub-50ms analysis overhead
- No network dependencies for core security checks

#### Trading bot operators
```toml
parapet-core = { path = "../core", features = ["ottersec"] }
```
**Enables:**
- Program allowlist enforcement (verified programs only)
- Configurable JSON security policies
- Programmatic access to structured risk data

#### RPC providers
```toml
parapet-core = { path = "../core", features = ["all-analyzers"] }
```
**Includes:** optional analyzers such as Helius, OtterSec, Jupiter, Rugcheck (see `Cargo.toml` features).

## Documentation

- [Wallet Integration Guide](../WALLET_INTEGRATION.md) — custom RPC in wallets
- [Proxy](../rpc-proxy/README.md) — rules path, config, run
- Analyzer code: `core/src/rules/analyzers/`

## Examples

See the [integrations/](../integrations/) directory for integration examples and the [docs/](../docs/) directory for usage guides.

## Development

```bash
# From the `parapet/` workspace root:
cargo test -p parapet-core
cargo build -p parapet-core --features all-analyzers
cargo clippy -p parapet-core --all-targets --all-features
```

## License

MIT

## Support

- Issues: https://github.com/securecheckio/parapet/issues
- Docs: See documentation files above
