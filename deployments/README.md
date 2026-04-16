# Parapet Deployments

Production deployment configurations for Parapet components.

## Available Deployments

### Two Primary Deployment Modes

- **[proxy-only/](./proxy-only/)** - Simple protected RPC (proxy only)
- **[full-stack/](./full-stack/)** - Complete stack (proxy + API + Redis + dashboard)

### Cloud Deployments (Fly.io)

- **[flyio/basic/](./flyio/basic/)** - Proxy-only deployment (maps to `proxy-only`)
- **[flyio/full/](./flyio/full/)** - Full stack deployment (maps to `full-stack`)
  - Ideal for AI agent operators: monitor activity, manage rules dynamically, and use the dashboard

### Infrastructure

- **[https/](./https/)** - HTTPS reverse proxy with Caddy/nginx (required for wallet connections)

## Rule Management

**🔄 Auto-Updating Rules (Recommended):** Use [Rule Feeds](../docs/RULE_FEEDS.md) to automatically update security rules from HTTP URLs without redeployment.

**Static Rules:** Bake rules into Docker images for fully offline deployments.

## Quick Start

Each subdirectory contains its own README with specific deployment instructions.

### Full Stack (Monitoring + Management)

```bash
cd full-stack/docker-compose/
docker-compose up -d
```

Access at: `http://localhost:8080`

### Proxy-only (RPC Security Layer)

```bash
cd proxy-only/docker/
docker-compose up -d
```

Proxy available at: `http://localhost:8899`

## Full-stack Architecture

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
