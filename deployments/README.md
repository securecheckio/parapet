# Parapet Deployments

Production deployment configurations for Parapet components.

## Available Deployments

### Two Primary Deployment Modes

- **[basic/](./basic/)** - Simple protected RPC (proxy only)
- **[full-stack/](./full-stack/)** - Complete stack (proxy + API + Redis + dashboard + MCP)

### Cloud Deployments (Fly.io)

- **[flyio/basic/](./flyio/basic/)** - Basic deployment (maps to `basic`)
- **[flyio/full/](./flyio/full/)** - Full stack deployment (maps to `full-stack`)
  - Ideal for AI agent operators: monitor activity, manage rules dynamically, and use the dashboard

### Infrastructure

- **[https/](./https/)** - HTTPS reverse proxy with Caddy/nginx (required for wallet connections)

## Decision Matrix

Choose your deployment based on your specific needs:

### Step 1: Basic vs Full Stack?


| Choose **Basic** (Proxy-Only) if you...   | Choose **Full Stack** if you...           |
| ----------------------------------------- | ----------------------------------------- |
| ✅ Only need RPC security layer            | ✅ Need real-time monitoring dashboard     |
| ✅ Want minimal complexity                 | ✅ Operating AI agents and want visibility |
| ✅ Don't need dynamic rule management      | ✅ Need dynamic rule updates via API       |
| ✅ Can use HTTP rule feeds or static rules | ✅ Want activity feed and analytics        |
| ✅ Want lowest operational cost            | ✅ Need MCP support for AI assistants      |
| ✅ Don't need Redis/database               | ✅ Have team collaboration needs           |


### Step 2: Choose Your Platform


| Platform                     | Best For                                | Pros                                                                                              | Cons                                                                  | Deployment                                                                                  |
| ---------------------------- | --------------------------------------- | ------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| **Fly.io**                   | Production, multi-region, managed infra | ✅ Easiest setup ✅ Auto-scaling ✅ Global CDN ✅ Managed Redis                                       | ❌ Vendor lock-in ❌ Cost for high traffic                              | [flyio/basic/](./flyio/basic/) [flyio/full/](./flyio/full/)                                 |
| **Docker Compose**           | Local dev, testing, single-server       | ✅ Complete local control ✅ Easy to customize ✅ No external deps ✅ Free                            | ❌ Manual scaling ❌ Single server ❌ Manual backups                     | [basic/docker/](./basic/docker/) [full-stack/docker-compose/](./full-stack/docker-compose/) |
| **Terraform + DigitalOcean** | Production IaC, custom infra            | ✅ Full infrastructure control ✅ Repeatable deploys ✅ Git-tracked config ✅ Choice of Docker/Native | ❌ More complexity ❌ Manual Redis setup ❌ Requires Terraform knowledge | [basic/terraform/](./basic/terraform/)                                                      |


### Step 3: Docker vs Native? (Terraform only)

If using Terraform, choose your runtime mode:


| Metric               | Docker Mode              | Native Mode                   |
| -------------------- | ------------------------ | ----------------------------- |
| **Setup Complexity** | ⭐ Very Easy              | ⭐⭐ Easy                       |
| **Updates**          | `docker pull` + restart  | Manual binary download        |
| **Performance**      | Baseline +2-5% latency   | Zero overhead                 |
| **Memory**           | +10-20MB overhead        | No overhead                   |
| **Security**         | Docker isolation         | systemd hardening             |
| **Use Case**         | <5000 req/s, general use | >5000 req/s, latency-critical |


**Use Docker Mode if:**

- Throughput < 5000 requests/second
- You want easy updates and rollbacks
- Portability matters
- You're distributing open-source software

**Use Native Mode if:**

- Throughput > 5000 requests/second
- Every millisecond of latency matters
- Resource-constrained environment (<512MB RAM)
- You want absolute maximum performance

See [basic/terraform/DEPLOYMENT_COMPARISON.md](./basic/terraform/DEPLOYMENT_COMPARISON.md) for detailed benchmarks.

### Step 4: Special Cases


| Scenario                                 | Solution                                                       | Notes                                    |
| ---------------------------------------- | -------------------------------------------------------------- | ---------------------------------------- |
| **Upgrade existing proxy to full-stack** | [full-stack/retrofit/](./full-stack/retrofit/)                 | Add monitoring without redeploying proxy |
| **Local HTTPS for wallet testing**       | [https/](./https/)                                             | Use Caddy/nginx reverse proxy            |
| **Quick local test**                     | [quickstart/](./quickstart/)                                   | Fastest way to try Parapet               |
| **Multi-region production**              | [flyio/basic/](./flyio/basic/) or [flyio/full/](./flyio/full/) | Fly.io handles regions automatically     |
| **Air-gapped/offline deployment**        | [basic/terraform/](./basic/terraform/) (Native) + static rules | No external dependencies                 |


### Quick Decision Tree

```
Start: What do you need?
│
├─ Just RPC protection?
│  └─ YES → Basic (Proxy-Only)
│      │
│      ├─ Cloud managed? → Fly.io Basic [flyio/basic/]
│      ├─ Local/testing? → Docker Compose [basic/docker/]
│      └─ Production IaC? → Terraform [basic/terraform/]
│
└─ NO → Monitoring/Dashboard needed?
    └─ YES → Full Stack
        │
        ├─ Cloud managed? → Fly.io Full [flyio/full/]
        ├─ Local/testing? → Docker Compose [full-stack/docker-compose/]
        └─ Upgrade existing? → Retrofit [full-stack/retrofit/]
```

### Example Use Cases


| Use Case                         | Recommended Deployment                                                                     | Why                                                            |
| -------------------------------- | ------------------------------------------------------------------------------------------ | -------------------------------------------------------------- |
| 🤖 **AI Agent RPC**              | [flyio/full/](./flyio/full/)                                                               | Monitor agent activity, see blocked transactions, manage rules |
| 🏢 **Team/Enterprise**           | [flyio/full/](./flyio/full/) or [full-stack/docker-compose/](./full-stack/docker-compose/) | Centralized dashboard, collaboration, audit trail              |
| 📱 **Mobile Wallet**             | [flyio/basic/](./flyio/basic/)                                                             | Lightweight, fast, global edge network                         |
| 🤝 **Trading Bot**               | [basic/terraform/](./basic/terraform/) (Native)                                            | Maximum performance, low latency                               |
| 🧪 **Development**               | [basic/docker/](./basic/docker/) or [quickstart/](./quickstart/)                           | Quick setup, easy iteration                                    |
| 🏭 **High-traffic RPC Provider** | [basic/terraform/](./basic/terraform/) (Native)                                            | Handles >5000 req/s efficiently                                |


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

### Basic (RPC Security Layer)

```bash
cd basic/docker/
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

## Production Considerations

1. **TLS/HTTPS**: Use Caddy or nginx with Let's Encrypt
2. **Redis Persistence**: Enable AOF or RDB snapshots
3. **Monitoring**: Set up health checks and alerting
4. **Rate Limiting**: Configure per-IP limits on proxy
5. **Log Aggregation**: Ship logs to centralized system

## Support

See main [README.md](../README.md) and [docs/](../docs/) for more information.