# Basic Deployments

Use this option when you want the simplest production path: protect transactions through the Parapet RPC proxy only.

## Includes

- Parapet RPC proxy
- Optional Redis integration for caching/rate limiting
- No API server
- No dashboard

## Deployment Methods

- [`docker/`](./docker/) - Basic Docker Compose deployment (proxy only)
- [`terraform/`](./terraform/) - Basic infrastructure provisioning (proxy only)

## Best For

- AI agents that only need transaction guardrails
- Trading bots
- DApps that only need protected RPC
