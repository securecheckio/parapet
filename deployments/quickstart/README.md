# Parapet Quick Start with Docker

Deploy Parapet RPC proxy with community security rules in under 2 minutes.

## Prerequisites

- Docker installed
- A Solana RPC endpoint (or use public mainnet)

## Quick Start

1. **Pull the latest image:**
   ```bash
   docker pull ghcr.io/securecheckio/parapet-rpc-proxy:latest
   ```

2. **Run with docker-compose:**
   ```bash
   curl -O https://raw.githubusercontent.com/securecheckio/parapet/main/deployments/quickstart/docker-compose.yml
   docker-compose up -d
   ```

3. **Or run directly with docker:**
   ```bash
   docker run -d \
     --name parapet-rpc-proxy \
     -p 8899:8899 \
     -e UPSTREAM_RPC_URL=https://api.mainnet-beta.solana.com \
     -e RULES_FEED_URLS=https://parapet-rules.securecheck.io/community/core-protection.json \
     --restart unless-stopped \
     ghcr.io/securecheckio/parapet-rpc-proxy:latest
   ```

4. **Test your proxy:**
   ```bash
   curl http://localhost:8899 -X POST -H "Content-Type: application/json" -d '
   {
     "jsonrpc": "2.0",
     "id": 1,
     "method": "getHealth"
   }'
   ```

## Configuration

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `UPSTREAM_RPC_URL` | Yes | - | Your Solana RPC endpoint |
| `RULES_FEED_URLS` | No | - | Comma-separated list of rule feed URLs (auto-enables feeds) |
| `HELIUS_API_KEY` | No | - | Enable Helius-powered rules |
| `JUPITER_API_KEY` | No | - | Enable Jupiter-powered rules |
| `RUST_LOG` | No | `info` | Log level (debug, info, warn, error) |

### Community Rule Feeds

Choose which rule feeds to enable:

- **Core Protection** (recommended, no API keys needed):
  ```
  https://parapet-rules.securecheck.io/community/core-protection.json
  ```

- **Helius Protection** (requires `HELIUS_API_KEY`):
  ```
  https://parapet-rules.securecheck.io/community/helius-protection.json
  ```

- **Jupiter Protection** (requires `JUPITER_API_KEY`):
  ```
  https://parapet-rules.securecheck.io/community/jupiter-protection.json
  ```

- **AI Agent Protection** (velocity limits):
  ```
  https://parapet-rules.securecheck.io/community/ai-agent-protection.json
  ```

### Using Multiple Feeds

Combine feeds with comma-separated URLs:

```yaml
environment:
  - RULES_FEED_URLS=https://parapet-rules.securecheck.io/community/core-protection.json,https://parapet-rules.securecheck.io/community/helius-protection.json
```

## Enhanced Protection with API Keys

For advanced threat detection, add your API keys:

```bash
docker run -d \
  --name parapet-rpc-proxy \
  -p 8899:8899 \
  -e UPSTREAM_RPC_URL=https://api.mainnet-beta.solana.com \
  -e HELIUS_API_KEY=your_key_here \
  -e JUPITER_API_KEY=your_key_here \
  -e RULES_FEED_URLS=https://parapet-rules.securecheck.io/community/core-protection.json,https://parapet-rules.securecheck.io/community/helius-protection.json,https://parapet-rules.securecheck.io/community/jupiter-protection.json \
  --restart unless-stopped \
  ghcr.io/securecheckio/parapet-rpc-proxy:latest
```

## Production Deployment

For production, see:
- [Full Stack Deployment](../full-stack/README.md) - With Redis and API
- [HTTPS Setup](../https/README.md) - With Caddy or Nginx
- [Terraform](../proxy-only/terraform/README.md) - Cloud deployment

## Monitoring

Check proxy status:
```bash
curl http://localhost:8899/health
```

View logs:
```bash
docker logs -f parapet-rpc-proxy
```

Check active rules:
```bash
curl http://localhost:8899/rules
```

## Next Steps

- **Custom Rules**: See [Rules Development Guide](../../docs/RULES_DEVELOPMENT.md)
- **Rule Feeds**: See [Rule Feeds Guide](../../docs/RULE_FEEDS.md)
- **API Integration**: See [API Documentation](../../api/README.md)

## Support

- GitHub: https://github.com/securecheckio/parapet
- Issues: https://github.com/securecheckio/parapet/issues
- Docs: https://github.com/securecheckio/parapet/tree/main/docs
