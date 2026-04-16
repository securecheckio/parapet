Parapet Logo

# Parapet

### Stop attacks before the transaction lands

**Perimeter security for Solana wallets, AI agents 🦞, and trading firms**

[License: MIT](LICENSE)
[Rust](https://www.rust-lang.org/)
[Solana](https://solana.com/)

  [Quick Start](#quick-start) • [Documentation](#documentation) • [Contributing](#contributing) • [Community](#community--support)

---

## What is Parapet?

Parapet is a **drop-in RPC that automatically inspects and blocks malicious transactions before they land**. It supports any upstream RPC from Helius, Triton, Quicknode, and others.

Open-source security for Solana with:

- **Real-time transaction interception** - Analyzes every transaction at the RPC layer
- **Wallet security scanning** - Comprehensive threat assessment for any Solana wallet
- **Embeddable library** - Integrate security analysis into any application

### Performance

**Proven sub-millisecond analysis** ([full benchmarks](docs/benchmarks/rpc-perf-2026-04-15.md)):

- **0.25ms median latency** - 70x faster than 50ms target
- **~2,900 tx/s throughput** - 29x faster than requirement
- **0.456ms p95 latency** - 22x better than target
- **Zero failures** - Stable under load

## Components

- **parapet-core** - Security analysis library (embeddable)
- **parapet-proxy** - RPC proxy with rule engine
- **parapet-scanner** - Wallet security scanner
- **parapet-api** - Rule management API & MCP server
- **parapet-mcp** - Model Context Protocol integration for AI agents
- **parapet** - Unified CLI for all commands

### Unique Features

- **AI Agent Ready** - Drop-in RPC protection with real-time monitoring dashboard
- **Embeddable Library** - Integrate security analysis into any Solana app
- **MCP Server** - First-class AI agent integration (Cursor, Claude Desktop)
- **Dual Deployment** - Client-side (fast local analysis) or server-side (centralized policies)
- **Enterprise Rule Engine** - JSON-based, auditable security rules

## 🚀 Quick Start

### CLI Usage

```bash
# Scan a wallet for threats
./parapet wallet <WALLET_ADDRESS>

# Check a transaction
./parapet tx <SIGNATURE>

# Analyze a program
./parapet program <PROGRAM_ID>

# Start RPC proxy
./parapet proxy

# See all commands
./parapet --help
```

### Local Development

```bash
# 1. Configure using TOML files (recommended for local dev)
cp proxy/config.toml.example proxy/config.toml
nano proxy/config.toml

# 2. Set secrets via environment
export HELIUS_API_KEY=your_key
export JUPITER_API_KEY=your_key

# 3. Run proxy
./parapet proxy
```

### Docker Deployment

```bash
cd deployments/proxy-only/docker
docker-compose up -d
```

### Production Deployment

```bash
# Fly.io - Recommended (Global edge, auto-scaling)
cd deployments/flyio/basic
fly launch
fly deploy

# Terraform (DigitalOcean, proxy-only)
cd deployments/proxy-only/terraform/digitalocean
cp terraform.tfvars.example terraform.tfvars
nano terraform.tfvars
terraform init
terraform apply
```

See [deployments/](deployments/) for all deployment options.

### 🔄 Auto-Updating Rules

Parapet supports **rule feeds** - automatically update security rules from HTTP URLs without redeployment:

```toml
[rule_feeds]
enabled = true
poll_interval = 3600  # Check every hour

[[rule_feeds.sources]]
url = "https://rules.parapet.security/community-base.json"
priority = 1
```

**Benefits:**
- ✅ Zero downtime updates (background polling)
- ✅ Instant protection from new threats
- ✅ Compose multiple rule sources
- ✅ Community + custom rules

**📖 Read the full guide:** [Rule Feeds Documentation](docs/RULE_FEEDS.md)

## 🛡️ Security Rules

Parapet uses **JSON-based declarative rules** with condition trees, pluggable analyzers, and stateful detection. Rules can:

- **Block, alert, or pass** transactions based on complex conditions
- **Track state across transactions** with flowbits (counters, flags, TTL)
- **Detect vulnerability patterns** at the bytecode level (missing checks, arbitrary CPI)
- **Compose third-party signals** (Helius, OtterSec, Jupiter, Rugcheck)
- **Use variable interpolation** for dynamic per-wallet/program/mint tracking

**Documentation:**

- [Rule Format Reference](docs/RULES_FORMAT.md) - JSON structure, operators, analyzers
- [Rule Development Guide](docs/RULES_DEVELOPMENT.md) - Hub for rule authoring
- [Flowbits Guide](docs/RULES_FLOWBITS.md) - Stateful detection patterns

**Community Rules:** [parapet-rules](https://github.com/securecheckio/parapet-rules) (separate repository)

## 🛠️ Development

### Prerequisites

**Required:**

- Rust 1.85+ (required by Solana SDK 4.0)

**Optional:**

- Redis 7+ (only needed for escalations, activity feed, or multi-instance caching)
- Node.js 18+ (only if you want to run the monitoring dashboard)

### Configuration

**Use TOML config files** (recommended):

```bash
# Proxy
cp proxy/config.toml.example proxy/config.toml
nano proxy/config.toml

# API
cp api/config.example.toml api/config.toml
nano api/config.toml
```

**Environment variables** for secrets only:

```bash
export HELIUS_API_KEY=your_key
export JUPITER_API_KEY=your_key
export MCP_API_KEYS=your_key
```

### Build

```bash
# Build all components
./parapet build

# Or use cargo directly
cargo build --workspace --release

# Run tests
./parapet test

# Run benchmarks
./parapet bench
```

### Project Structure

```
parapet/
├── core/                    # Security analysis library
├── proxy/                   # RPC proxy with rule engine
├── scanner/                 # Wallet security scanner
├── mcp/                     # MCP server
├── api/               # Rule management API
├── tools/
│   ├── rpc-perf/          # Proxy + rule-engine latency harness
│   ├── flowbits-perf/     # Flowbits Criterion benchmarks
│   └── risk-register/     # Risk database & analysis
├── docs/                   # Documentation
└── deployments/           # Deployment configurations
    ├── proxy-only/        # Simple proxy-only deployment
    ├── full-stack/        # Proxy + API + Redis + dashboard
    └── flyio/             # Fly.io basic/full variants
```

## 📖 Documentation

### Getting Started

- [OpenClaw Integration Guide](docs/OPENCLAW_SETUP.md) - Complete guide for AI agents
- [Quick Start](docs/QUICKSTART.md) - Get running in 5 minutes

### Deployment

- [Deployment Guide](docs/operators/deployment-guide.md)
- [Configuration Reference](docs/operators/configuration.md)

## 🤝 Community & Support

- **Issues**: [GitHub Issues](https://github.com/securecheckio/parapet/issues)
- **Website**: [securecheck.io/parapet](https://securecheck.io/parapet)

## 🌟 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 💖 Acknowledgments

Built with ❤️ for the Solana ecosystem by SecureCheck.

---

**⚠️ Security Notice**: This software is provided as-is. While we strive for high quality and security, always perform your own security audits before using in production.