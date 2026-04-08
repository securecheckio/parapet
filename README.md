# Parapet

> Fast, portable Solana transaction security

**By SecureCheck**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

## What is Parapet?

Parapet is an open-source security platform for Solana that provides:
- **Transaction analysis and threat detection** - Real-time rule-based security
- **Wallet security scanning** - Comprehensive threat assessment for Solana wallets
- **Phishing site analysis** - Detection and analysis of malicious sites
- **Secure RPC proxy** - Configurable security rules at the RPC level

## Components

- **parapet-core** - Security analysis library
- **parapet-proxy** - RPC proxy with rule engine
- **parapet-scanner** - Wallet security scanner
- **parapet-sentinel** - Phishing site analyzer
- **parapet-mcp** - Model Context Protocol server integration
- **parapet-api** - Rule management API

## For Community Operators

Parapet enables you to run your own secure RPC server for your community, DAO, or organization.

### Quick Start

```bash
# Clone the repository
git clone https://github.com/securecheckio/parapet
cd parapet

# Deploy with Docker (easiest)
cd deployments/proxy/docker
cp .env.example .env
# Edit .env with your settings
docker-compose up -d

# Or deploy with Terraform (production)
cd deployments/proxy/terraform/digitalocean
cp terraform.tfvars.example terraform.tfvars
# Edit terraform.tfvars
terraform init
terraform apply
```

See [`docs/operators/`](docs/operators/) for complete deployment guides.

## Security Rules

Parapet includes comprehensive security rules from SecureCheck:
- All rules are free and open source
- Licensed under CC BY-NC-SA 4.0 (non-commercial use)
- Updated regularly via GitHub
- Custom rule development available (consulting)

Rules are maintained in a separate repository: [parapet-rules](https://github.com/securecheckio/parapet-rules)

## Reference Implementations

The `reference/` directory contains example implementations for building a multi-tenant SaaS service:
- **auth-api** - Multi-tenant authentication with PostgreSQL
- **gateway** - SaaS RPC gateway wrapper
- **dashboard** - Web UI for users and operators

These are provided as educational examples showing how to build on top of Parapet.

## Development

### Prerequisites

- Rust 1.70+
- Node.js 18+ (for Sentinel and dashboard)
- PostgreSQL 15+ (for SaaS reference implementations)
- Redis 7+ (optional, for caching and rate limiting)

### Build

```bash
# Build all Rust components
cargo build --workspace --release

# Run tests
cargo test --workspace

# Run performance benchmarks
cargo run --release -p rpc-perf -- --iterations 500
```

### Project Structure

```
parapet/
├── core/                    # Security analysis library
├── proxy/                   # RPC proxy with rule engine
├── scanner/                 # Wallet security scanner
├── sentinel/                # Phishing site analyzer (TypeScript)
├── mcp/                     # MCP server
├── api/                     # Rule management API
├── integrations/
│   └── agent-kit/          # Solana Agent Kit plugin
├── reference/              # SaaS reference implementations
│   ├── auth-api/           # Multi-tenant auth
│   ├── gateway/            # SaaS gateway
│   └── dashboard/          # Web UI
├── tools/
│   ├── rpc-perf/          # Performance benchmarking
│   └── risk-register/     # Risk database & analysis
├── docs/                   # Documentation
├── examples/               # Example configurations
└── deployments/           # Deployment configurations
    ├── proxy/             # Standalone proxy deployment
    └── reference/         # Full SaaS stack (optional)
```

## Documentation

- [Deployment Guide](docs/operators/deployment-guide.md)
- [Configuration Reference](docs/operators/configuration.md)
- [Rule Format](https://github.com/securecheckio/parapet-rules/blob/main/docs/rule-format.md)
- [API Documentation](docs/api/)

## Community & Support

- **Issues**: [GitHub Issues](https://github.com/securecheckio/parapet/issues)
- **Discussions**: [GitHub Discussions](https://github.com/securecheckio/parapet/discussions)
- **Discord**: [SecureCheck Community](https://discord.gg/securecheck)
- **Website**: [securecheck.io/parapet](https://securecheck.io/parapet)

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Business Model

- **Software**: 100% open source (MIT license)
- **Rules**: 100% free (CC BY-NC-SA 4.0 license)
- **Revenue**: Custom rule development and security consulting

### Services

- Custom rule development: $5k-$20k per project
- Security consulting: $2k-$10k per engagement
- Threat analysis: $2k-$10k per analysis
- Training & workshops: $1k-$5k per session
- Enterprise support: $5k-$20k/month retainer

Contact: [security@securecheck.io](mailto:security@securecheck.io)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

Security rules are licensed under CC BY-NC-SA 4.0 - see the [parapet-rules](https://github.com/securecheckio/parapet-rules) repository.

## Acknowledgments

Built with ❤️ for the Solana ecosystem by SecureCheck.

---

**⚠️ Security Notice**: This software is provided as-is. While we strive for high quality and security, always perform your own security audits before using in production.
