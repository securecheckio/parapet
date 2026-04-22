# Proxy-only Docker Deployment

Deploy only the Parapet RPC proxy with a single Docker Compose service.

## What You Get

- RPC proxy with transaction analysis and blocking
- Static local rules or HTTP rule feeds
- No API service, dashboard, or required Redis service

## Quick Start

```bash
cd deployments/proxy-only/docker
docker-compose up -d --build
curl http://localhost:8899/health
```

## Configuration

Set values in your shell or `.env` file:

- `UPSTREAM_RPC_URL` (recommended)
- `DEFAULT_BLOCK_THRESHOLD` (default: `70`)
- `RULES_PATH` (default: `/app/rules/presets/default-protection.json`)
- `ENABLE_ESCALATIONS` (default: `false`)
- `REDIS_URL` (optional, only if you run Redis externally)

Optional analyzer keys:

- `HELIUS_API_KEY`
- `JUPITER_API_KEY`
- `OTTERSEC_API_KEY`

## Notes

- Rules are mounted from `rpc-proxy/rules` into `/app/rules`.
- No default rules are enforced unless you set a valid `RULES_PATH` or rule feed configuration.
