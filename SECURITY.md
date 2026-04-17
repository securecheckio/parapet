# Security Policy

## Supported Versions

We provide security updates for the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1   | :x:                |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

### Private Disclosure

Send vulnerability reports to: **security@securecheck.io**

Include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if available)

### What to Expect

- **Acknowledgment**: Within 48 hours
- **Initial assessment**: Within 1 week
- **Status updates**: Every 7 days until resolved
- **Disclosure timeline**: 90 days (or sooner if mutually agreed)

### Coordinated Disclosure

We follow coordinated disclosure practices:
1. Private report received
2. Vulnerability confirmed and assessed
3. Fix developed and tested
4. Security advisory published
5. Credit given to reporter (unless anonymous requested)

## Security Measures

### Dependency Security

- **Daily CVE scanning** via `cargo-audit`
- **Supply chain validation** via `cargo-deny`
- **Automated updates** via Dependabot
- **SBOM generation** for all releases

### Code Security

- **Security test suite** runs on every PR
- **Static analysis** via Clippy with security lints
- **Fuzzing** for parser and analyzer components (planned)
- **Code signing** via GitHub attestations

### Infrastructure Security

- **Rate limiting** on all public endpoints
- **Authentication** required for write operations
- **TLS 1.3** for all network communications
- **Redis auth** required in production
- **Read-only filesystem** in Docker containers

## Security Best Practices for Users

### Proxy Deployment

- ✅ Enable API key authentication
- ✅ Use TLS/HTTPS in production
- ✅ Configure rate limits
- ✅ Run as non-root user
- ✅ Use firewall to restrict access
- ✅ Enable Redis authentication
- ✅ Regularly update to latest version

### API Keys

- ✅ Rotate API keys quarterly
- ✅ Use different keys per environment
- ✅ Store keys in secrets management (not env files)
- ✅ Monitor key usage for anomalies
- ✅ Revoke compromised keys immediately

### Rule Management

- ✅ Review rules before deployment
- ✅ Test rules in staging first
- ✅ Use version control for custom rules
- ✅ Audit rule changes
- ✅ Subscribe to security rule updates

## Known Security Considerations

### Transaction Simulation

Transaction simulation uses upstream RPC providers. Simulation results should be treated as advisory, not authoritative. Always validate critical transactions.

### FlowState State

FlowState use in-memory or Redis state. In high-availability deployments, ensure Redis is properly secured and replicated.

### Third-Party Analyzers

Optional analyzers (Helius, OtterSec, Rugcheck, Jupiter) make external API calls. Review their privacy policies and rate limits.

## Security Audits

- **Last audit**: Not yet audited
- **Planned audit**: Q2 2026
- **Audit reports**: Will be published at `/docs/audits/`

## Bug Bounty Program

Coming soon. Contact security@securecheck.io for inquiries.

## Security Champions

- **Security Lead**: TBD
- **Vulnerability Response**: security@securecheck.io
- **Community Security**: GitHub Security Advisories

## Acknowledgments

We thank the security researchers who have helped improve Parapet's security:
- (None yet - be the first!)
