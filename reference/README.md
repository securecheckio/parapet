# Parapet Reference Implementations

Reference implementations and example integrations for Parapet.

## Purpose

This directory contains **reference code** for building integrations and platforms on top of Parapet. These are examples and starting points, not production-ready components.

## Components

### [dashboard/](./dashboard/)

Full-featured marketing and learning platform with:

- Multi-page React application
- Transaction simulation
- Educational content
- Wallet integration examples
- Rules visualization

**Note**: For AI agent activity monitoring, use `../dashboard/` instead.

### [auth-api/](./auth-api/)

Reference authentication service with:

- API key management
- Wallet-based authentication
- Session handling
- RBAC examples

### [gateway/](./gateway/)

Reference API gateway with:

- Rate limiting
- Request routing
- Authentication middleware
- Monitoring hooks

## Usage

These implementations demonstrate Parapet patterns but should be adapted for your use case:

1. **Learning**: Study the code to understand Parapet integration
2. **Prototyping**: Fork and modify for quick prototypes
3. **Production**: Use as reference when building production systems

## Development vs Production


| Directory              | Purpose                      | Status           |
| ---------------------- | ---------------------------- | ---------------- |
| `reference/dashboard/` | Marketing/learning platform  | Reference only   |
| `../dashboard/`        | AI agent activity monitoring | Production-ready |
| `reference/auth-api/`  | Auth examples                | Reference only   |
| `../api-core/`         | Production API               | Production-ready |


## Contributing

Improvements to reference implementations are welcome! See [../CONTRIBUTING.md](../CONTRIBUTING.md).

## Support

For questions about reference implementations:

- Open an issue: [https://github.com/securecheckio/parapet/issues](https://github.com/securecheckio/parapet/issues)

