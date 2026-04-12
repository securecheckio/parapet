<p align="center">
  <img src="parapet.png" alt="Parapet Logo" width="400"/>
</p>

# Parapet

### Fast, portable Solana transaction security

**Open-source security platform for the Solana ecosystem**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Solana](https://img.shields.io/badge/Built%20for-Solana-14F195?logo=solana)](https://solana.com/)

  [Quick Start](#quick-start) • [Documentation](#documentation) • [Contributing](#contributing) • [Community](#community--support)

---

## What is Parapet?

Parapet is an open-source security platform for Solana that provides:

- **Transaction analysis and threat detection** - Real-time rule-based security
- **Wallet security scanning** - Comprehensive threat assessment for Solana wallets
- **Secure RPC proxy** - Configurable security rules at the RPC level

## Components

- **parapet-core** - Security analysis library
- **parapet-proxy** - RPC proxy with rule engine
- **parapet-scanner** - Wallet security scanner
- **parapet-mcp** - Model Context Protocol server integration
- **parapet-platform** - Multi-tenant API platform
- **parapet** - Unified CLI for all commands

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
cd deployments/proxy/docker
cp .env.example .env      # Docker Compose uses .env files
nano .env                 # Edit configuration
docker-compose up -d
```

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

Security rules for Parapet are maintained in a separate repository: [parapet-rules](https://github.com/securecheckio/parapet-rules)

## 📚 Reference Implementations

The `reference/` directory contains example implementations for building a multi-tenant RPC user interface:

- **auth-api** - Multi-tenant authentication with PostgreSQL
- **gateway** - RPC gateway wrapper
- **dashboard** - Web UI for users and operators

These are provided as educational examples showing how to build on top of Parapet.

## 🛠️ Development

### Prerequisites

- Rust 1.70+
- Redis 7+ (recommended for production)
- Node.js 18+ (for reference dashboard)
- PostgreSQL 15+ (for reference implementations only)

### Configuration

**Use TOML config files** (recommended):

```bash
# Proxy
cp proxy/config.toml.example proxy/config.toml
nano proxy/config.toml

# API
cp api-core/config.example.toml api-core/config.toml
nano api-core/config.toml
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
├── api/                     # Rule management API
├── reference/              # Reference implementations
│   ├── auth-api/           # Multi-tenant auth
│   ├── gateway/            # RPC gateway wrapper
│   └── dashboard/          # Web UI
├── tools/
│   ├── rpc-perf/          # Proxy + rule-engine latency harness
│   ├── flowbits-perf/     # Flowbits Criterion benchmarks
│   └── risk-register/     # Risk database & analysis
├── docs/                   # Documentation
├── examples/               # Example configurations
└── deployments/           # Deployment configurations
    ├── proxy/             # Standalone proxy deployment
    └── reference/         # Reference stack (optional)
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