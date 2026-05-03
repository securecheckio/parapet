![Parapet Logo](parapet.png)

# Parapet

### Stop attacks before the transaction lands

**Perimeter security for Solana wallets, AI agents 🦞, and trading firms**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.89+-orange.svg?logo=rust)](https://www.rust-lang.org/)
[![Solana](https://img.shields.io/badge/Solana-9945FF?logo=solana&logoColor=white)](https://solana.com/)

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

- **[parapet-core](core/)** - Security analysis library (embeddable)
- **[parapet-rpc-proxy](rpc-proxy/)** - RPC proxy with rule engine
- **[parapet-scanner](scanner/)** - Wallet security scanner CLI
- **[parapet-api](api/)** - HTTP API with rule management and MCP-over-HTTP
- **[parapet-mcp](mcp/)** - STDIO MCP server for local AI assistants (Cursor, Claude)

### Unique Features

- **Embeddable Library** - Integrate security analysis into any Solana app
- **MCP Server** - First-class AI agent integration (Cursor, Claude Desktop)
- **Dual Deployment** - Client-side (fast local analysis) or server-side (centralized policies)
- **Enterprise Rule Engine** - JSON-based, auditable security rules

## 🚀 Quick Start

### CLI usage (from workspace root)

Binaries are built with Cargo from this repository:

```bash
# Wallet scanner (mainnet RPC default)
cargo run -p parapet-scanner --bin wallet-scanner -- <WALLET_ADDRESS>

# Transaction check (see scanner for other bins: tx-check, program-analyzer, …)
cargo run -p parapet-scanner --bin tx-check -- --help

# RPC proxy
cargo run -p parapet-rpc-proxy --bin parapet-rpc-proxy

# Rule / MCP API server
cargo run -p parapet-api --bin parapet-api

# MCP (stdio)
cargo run -p parapet-mcp-server --bin parapet-mcp -- --help
```

### Local development

```bash
# 1. Configure using TOML files (recommended for local dev)
cp rpc-proxy/config.toml.example rpc-proxy/config.toml
nano rpc-proxy/config.toml

# 2. Set secrets via environment
export HELIUS_API_KEY=your_key
export JUPITER_API_KEY=your_key

# 3. Run proxy
cargo run -p parapet-rpc-proxy --bin parapet-rpc-proxy
```

### Docker Deployment (Recommended)

Pull and run from GitHub Container Registry with community security rules:

```bash
# Single upstream URL (or use UPSTREAM_RPC_URLS=comma,separated,list for failover)
docker run -d -p 8899:8899 \
  -e UPSTREAM_RPC_URL=https://api.mainnet-beta.solana.com \
  -e RULES_FEED_URLS=https://parapet-rules.securecheck.io/community/core-protection.json \
  ghcr.io/securecheckio/parapet-rpc-proxy:latest
```

Or use docker-compose:

```bash
cd deployments/quickstart
docker-compose up -d
```

See [deployments/quickstart/README.md](deployments/quickstart/README.md) for full deployment guide.

### Production Deployment

```bash
# Terraform (DigitalOcean)
cd deployments/proxy/terraform/digitalocean
cp terraform.tfvars.example terraform.tfvars
nano terraform.tfvars
terraform init
terraform apply
```

See [docs/operators/](docs/operators/) for complete deployment guides.

## 🛡️ Security Rules

Parapet uses **JSON-based declarative rules** with condition trees, pluggable analyzers, and stateful detection. Rules can:

- **Block, alert, or pass** transactions based on complex conditions
- **Track state across transactions** with flowstate (counters, flags, TTL)
- **Detect vulnerability patterns** at the bytecode level (missing checks, arbitrary CPI)
- **Compose third-party signals** (Helius, OtterSec, Jupiter, Rugcheck)
- **Use variable interpolation** for dynamic per-wallet/program/mint tracking

**Documentation:**
- [Rule Format Reference](docs/RULES_FORMAT.md) - JSON structure, operators, analyzers
- [Rule Development Guide](docs/RULES_DEVELOPMENT.md) - Hub for rule authoring
- [FlowState Guide](docs/RULES_FLOWSTATE.md) - Stateful detection patterns

**Community Rules:** [parapet-rules](https://github.com/securecheckio/parapet-rules) (separate repository)

## 🛠️ Development

### Prerequisites

**Required:**
- Rust 1.89+ (workspace `rust-version`; Solana SDK 4.x)

**Optional:**
- Redis 7+ (only needed for escalations, activity feed, or multi-instance caching)
- Node.js 18+ (only if you want to run the monitoring dashboard)

### Configuration

**Use TOML config files** (recommended):

```bash
# Proxy
cp rpc-proxy/config.toml.example rpc-proxy/config.toml
nano rpc-proxy/config.toml

# API
cp api/config.example.toml api/config.toml
nano api/config.toml
```

Upstream RPC (single URL, multi-endpoint failover, optional smart routing, method allow/block) is documented in **[docs/OPERATIONS_GUIDE.md](docs/OPERATIONS_GUIDE.md#multi-upstream-rpc-proxy-and-api)** and **[rpc-proxy/README.md](rpc-proxy/README.md)**.

**Environment variables** for secrets only:

```bash
export HELIUS_API_KEY=your_key
export JUPITER_API_KEY=your_key
export MCP_API_KEYS=your_key
```

Use **`UPSTREAM_RPC_URL`** / **`UPSTREAM_RPC_URLS`** (proxy) or **`SOLANA_RPC_URL`** / **`SOLANA_RPC_URLS`** (API) when you need container overrides instead of baking URLs into TOML.

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
│   ├── flowstate-perf/     # FlowState Criterion benchmarks
│   └── risk-register/     # Risk database & analysis
├── docs/                   # Documentation
└── deployments/           # Deployment configurations
    └── proxy/             # Standalone proxy deployment
```

## 📖 Documentation

### Getting Started

- [OpenClaw Integration Guide](docs/OPENCLAW_SETUP.md) - Complete guide for AI agents
- [Quick Start](docs/QUICKSTART.md) - Get running in 5 minutes
- [Operations Guide](docs/OPERATIONS_GUIDE.md) - Production operations, including **multi-upstream RPC**

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