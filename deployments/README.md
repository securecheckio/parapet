# Parapet Deployments

Production deployment configurations for Parapet components.

## Available Deployments

### Core Services

- **[proxy/](./proxy/)** - RPC proxy deployment (Docker, Terraform, cloud-init)
- **[dashboard/](./dashboard/)** - Activity feed dashboard (Docker Compose)

### Infrastructure

- **[caddy/](./caddy/)** - Caddy reverse proxy with auto-HTTPS
- **[https/](./https/)** - HTTPS/TLS configuration
- **[reference/](./reference/)** - Reference platform deployment

## Quick Start

Each subdirectory contains its own README with specific deployment instructions.

### Dashboard (AI Agent Monitoring)

```bash
cd dashboard/
docker-compose up -d
```

Access at: `http://localhost:8080`

### Proxy (RPC Security Layer)

```bash
cd proxy/docker/
docker-compose up -d
```

Proxy available at: `http://localhost:8899`

## Architecture

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│  Dashboard  │────▶│  API Server  │────▶│    Redis    │
│  (port 80)  │     │  (port 3001) │     │ (port 6379) │
└─────────────┘     └──────────────┘     └─────────────┘
                           │
                           ▼
                    ┌──────────────┐     ┌─────────────┐
                    │  RPC Proxy   │────▶│   Solana    │
                    │ (port 8899)  │     │     RPC     │
                    └──────────────┘     └─────────────┘
```

## Environment Variables

Key secrets (set in `.env` or docker-compose):
- `HELIUS_API_KEY` - Helius API key (optional)
- `JUPITER_API_KEY` - Jupiter API key (optional)
- `REDIS_URL` - Redis connection string
- `APPROVER_WALLET` - Wallet for transaction approval

## Production Considerations

1. **TLS/HTTPS**: Use Caddy or nginx with Let's Encrypt
2. **Redis Persistence**: Enable AOF or RDB snapshots
3. **Monitoring**: Set up health checks and alerting
4. **Rate Limiting**: Configure per-IP limits on proxy
5. **Log Aggregation**: Ship logs to centralized system

## Support

See main [README.md](../README.md) and [docs/](../docs/) for more information.
