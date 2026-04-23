# Parapet on [Fly.io](http://Fly.io)

Deploy Parapet globally with low-latency RPC proxying.

## Deployment Options

Choose your deployment model:

### [Basic](./basic/) - Proxy Only (Recommended)

- RPC proxy with transaction inspection and blocking
- No default rules (you must configure your own)
- No Redis, no API server needed
- Fast, simple, cheap

### [Full](./full/) - Proxy + API + Redis + Dashboard

- Complete stack with API server and web UI
- Dashboard for monitoring activity feed
- Redis for caching and state
- Dynamic rule management via API

**Ideal for AI agent operators:** Monitor your agent's transactions in real-time, see what's blocked, and manage rules without redeployment.

## Quick Start

### Basic (Proxy Only)

```bash
# Deploy from parapet root directory
cd /path/to/parapet
fly launch --config deployments/flyio/basic/fly.toml --dockerfile deployments/flyio/basic/Dockerfile --no-deploy
fly deploy --config deployments/flyio/basic/fly.toml --dockerfile deployments/flyio/basic/Dockerfile -a parapet-rpc-proxy
```

**Note:** Must deploy from parapet root (not from `deployments/flyio/basic`) because the Dockerfile needs access to all workspace crates.

### Full Stack

```bash
cd deployments/flyio/full
./deploy.sh
```

See the README in each directory for detailed instructions.