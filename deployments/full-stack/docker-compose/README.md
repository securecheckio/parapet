# Parapet Docker Deployment

Complete containerized deployment of Parapet with all components.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Parapet Stack                         │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │   Dashboard  │───▶│     API      │───▶│    Redis     │  │
│  │   (nginx)    │    │   :3001      │    │   :6379      │  │
│  │   :8080      │    │              │    │              │  │
│  └──────────────┘    └──────┬───────┘    └──────────────┘  │
│                              │                    ▲          │
│                              │                    │          │
│  ┌──────────────┐            │                    │          │
│  │  AI Agent/   │───────────▶│    Proxy     ├────┘          │
│  │   Client     │            │    :8899     │               │
│  └──────────────┘            └──────┬───────┘               │
│                                      │                       │
│                                      ▼                       │
│                              ┌──────────────┐               │
│                              │   Solana     │               │
│                              │   Network    │               │
│                              └──────────────┘               │
└─────────────────────────────────────────────────────────────┘
```

## Components

### 1. **Proxy** (port 8899)
- Analyzes transactions before forwarding to Solana
- Blocks high-risk transactions
- Creates escalations for human review
- Main entry point for AI agents and clients

### 2. **API** (port 3001)
- REST API for escalation management
- WebSocket for real-time notifications
- Rule management endpoints
- Wallet authentication

### 3. **Dashboard** (port 8080)
- Web UI for human approvers
- Real-time escalation notifications
- Transaction details and risk analysis
- Approve/deny interface

### 4. **Redis** (port 6379)
- Shared state for escalations
- Rate limiting coordination
- Caching layer

## Quick Start

### 1. Configure Environment

```bash
cd deployments/full-stack/docker-compose
cp .env.example .env
# Edit .env with your settings
```

**Required variables:**
- `ESCALATION_APPROVER_WALLET` - Wallet that can approve escalations
- `UPSTREAM_RPC_URL` - Your Solana RPC endpoint

**Recommended:**
- `HELIUS_API_KEY` - For wallet identity checks
- `AUTHORIZED_WALLETS` - For rule management

### 2. Start Services

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f

# Check status
docker-compose ps
```

### 3. Access Services

- **Dashboard**: http://localhost:8080
- **Proxy RPC**: http://localhost:8899
- **API**: http://localhost:3001

### 4. Configure Your Client

Point your Solana client to the proxy:

```typescript
const connection = new Connection('http://localhost:8899');
```

For AI agents (OpenClaw, Cursor):

```bash
export PARAPET_RPC_URL=http://localhost:8899
export PARAPET_API_URL=http://localhost:3001
```

## Usage Workflows

### Transaction Protection Flow

1. **Client** sends transaction → **Proxy** :8899
2. **Proxy** analyzes risk
3. If **risk < threshold** → Forward to Solana ✅
4. If **risk >= threshold** → Create escalation → Block ❌
5. **Dashboard** shows escalation notification
6. **Human** reviews and approves/denies
7. If approved → Client retries → **Proxy** allows

### Rule Management Flow

1. Connect wallet to **Dashboard**
2. Create custom rule via UI
3. **API** stores in Redis
4. **Proxy** applies rule to new transactions

## Configuration

### Environment Variables

**For Docker deployments**, use `.env` files (Docker Compose standard).

**Note:** For native/local development, prefer TOML config files (see [proxy/README.md](../../../proxy/README.md)).

See `.env.example` for complete list. Key settings:

```bash
# Security threshold (0-100)
DEFAULT_BLOCK_THRESHOLD=70  # Higher = more permissive

# Enable human approvals
ENABLE_ESCALATIONS=true

# Approver wallet
ESCALATION_APPROVER_WALLET=YourWalletAddressHere
```

### Rule Presets

Choose rule strictness:

```bash
# Balanced (default)
RULES_PATH=/app/rules/presets/default.json

# Maximum security
RULES_PATH=/app/rules/presets/strict.json

# Minimal blocking
RULES_PATH=/app/rules/presets/permissive.json
```

### Custom Rules

Mount your custom rules:

```yaml
volumes:
  - ./my-custom-rules.json:/app/rules/custom/rules.json:ro
```

Then set:
```bash
RULES_PATH=/app/rules/custom/rules.json
```

## Monitoring

### Health Checks

```bash
# Check all services
docker-compose ps

# Individual health checks
curl http://localhost:8899/health  # Proxy
curl http://localhost:3001/health  # API
curl http://localhost:8080/health  # Dashboard
```

### View Logs

```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f proxy
docker-compose logs -f api

# Last 100 lines
docker-compose logs --tail=100 proxy
```

### Redis Monitoring

```bash
# Connect to Redis
docker-compose exec redis redis-cli

# View escalations
KEYS escalation:pending:*

# Check rate limits
KEYS rate_limit:*
```

## Troubleshooting

### Proxy not starting

```bash
# Check logs
docker-compose logs proxy

# Common issues:
# - Invalid UPSTREAM_RPC_URL
# - Port 8899 already in use
# - Redis connection failed
```

### API connection refused

```bash
# Ensure Redis is healthy
docker-compose ps redis

# Restart API
docker-compose restart api
```

### Dashboard not loading

```bash
# Check nginx logs
docker-compose logs dashboard

# Verify API is reachable
curl http://localhost:3001/health
```

### Rate limiting errors (429)

Adjust rate limits in `.env`:

```bash
# Reduce requests to external APIs
RUGCHECK_RATE_LIMIT=5/60  # Slower but safer
HELIUS_RATE_LIMIT=10/60
```

## Updating

### Update specific component

```bash
# Rebuild proxy
docker-compose build proxy
docker-compose up -d proxy

# Rebuild API
docker-compose build api
docker-compose up -d api
```

### Update all

```bash
# Rebuild and restart everything
docker-compose down
docker-compose build
docker-compose up -d
```

## Production Deployment

### Enable HTTPS

Use a reverse proxy (Traefik, nginx, Caddy) in front:

```yaml
# Example with Traefik labels
labels:
  - "traefik.enable=true"
  - "traefik.http.routers.dashboard.rule=Host(`shield.yourdomain.com`)"
  - "traefik.http.routers.dashboard.tls.certresolver=letsencrypt"
```

### Persist Redis Data

```yaml
volumes:
  redis-data:
    driver: local
    driver_opts:
      type: none
      o: bind
      device: /var/lib/parapet/redis
```

### Resource Limits

```yaml
services:
  proxy:
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
        reservations:
          cpus: '1'
          memory: 512M
```

### Use External Redis

For high availability:

```bash
# .env
REDIS_URL=redis://prod-redis.example.com:6379
```

Remove Redis service from docker-compose.yml.

## Advanced Configuration

### Multiple Proxy Instances

Load balance across multiple proxies:

```yaml
proxy:
  deploy:
    replicas: 3
```

Add load balancer (nginx, HAProxy) in front.

### Separate Networks

For security, isolate services:

```yaml
networks:
  frontend:  # Dashboard → API
  backend:   # API → Redis, Proxy → Redis
```

### Custom Dockerfile

For optimizations:

```dockerfile
# Dockerfile.proxy.custom
FROM your-base-image
COPY --from=builder /build/target/release/parapet-proxy .
# Add custom config
```

## Backup & Recovery

### Backup Redis Data

```bash
# Create backup
docker-compose exec redis redis-cli BGSAVE
docker cp parapet-redis:/data/dump.rdb ./backup/

# Restore
docker cp ./backup/dump.rdb parapet-redis:/data/
docker-compose restart redis
```

### Backup Configuration

```bash
# Backup .env and custom rules
tar -czf parapet-backup.tar.gz .env proxy/rules/custom/
```

## Security Considerations

1. **Change default ports** in production
2. **Use API keys** for proxy authentication
3. **Restrict AUTHORIZED_WALLETS** to trusted addresses
4. **Enable firewall** rules to limit access
5. **Monitor logs** for suspicious activity
6. **Rotate Redis password** regularly (add AUTH to Redis)

## Support

- Check logs first: `docker-compose logs -f`
- Test individual components
- Verify Redis connectivity
- Check Solana RPC endpoint is reachable

## Related Docs

- [Operations Guide](../../docs/OPERATIONS_GUIDE.md)
- [Agent Integration Guide](../../docs/AGENT_GUIDE.md)
- [Use Cases](../../docs/USE_CASES.md)
