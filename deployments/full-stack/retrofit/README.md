# Parapet Retrofit Deployment (Advanced)

Docker Compose deployment for adding dashboard/API monitoring to an existing proxy deployment.

This is an advanced retrofit path. Primary deployment options are `basic` and `full-stack`.

## Quick Start

```bash
# Copy example config
cp ../../api/config.example.toml ../../api/config.toml

# Edit configuration
vim ../../api/config.toml

# Start services
docker-compose up -d

# Check logs
docker-compose logs -f
```

## Services

- **dashboard**: React frontend (port 8080)
- **api**: Parapet API server (port 3001)
- **redis**: Redis for activity feed (port 6379)

## Configuration

Edit `docker-compose.yml` to customize:

```yaml
environment:
  - REDIS_URL=redis://redis:6379
  - HELIUS_API_KEY=${HELIUS_API_KEY}  # Optional
  - LOG_LEVEL=info
```

## Volumes

- `redis-data`: Persistent Redis storage
- `../../api/config.toml`: API configuration (mounted)

## Networking

Default ports:

- Dashboard: `http://localhost:8080`
- API: `http://localhost:3001`
- Redis: `localhost:6379` (internal only)

## Health Checks

```bash
# Check all services
docker-compose ps

# Check API health
curl http://localhost:3001/health

# Check Redis
docker-compose exec redis redis-cli ping
```

## Production

For production, use nginx/Caddy for HTTPS:

```bash
# Build production dashboard
cd ../../dashboard/
npm run build

# Deploy with nginx
docker run -d \
  -p 80:80 \
  -v $(pwd)/dist:/usr/share/nginx/html:ro \
  nginx:alpine
```

See [dashboard/Dockerfile](../../dashboard/Dockerfile) for containerized production build.

## Troubleshooting

### Dashboard not loading

- Check if services are running: `docker-compose ps`
- Check API logs: `docker-compose logs api`

### No activity showing

- Verify proxy is running and forwarding to API
- Check Redis connection: `docker-compose exec redis redis-cli keys activity:*`
- Verify wallet is connected in dashboard

### CORS errors

- Ensure API allows dashboard origin in config.toml
- Check browser console for specific CORS issues

## Development

To develop locally without Docker:

```bash
# Terminal 1: Start Redis
docker run -p 6379:6379 redis:7-alpine

# Terminal 2: Start API
cd ../../api/
cargo run

# Terminal 3: Start dashboard dev server
cd ../../dashboard/
npm run dev
```

Access at `http://localhost:5173` (Vite dev server).