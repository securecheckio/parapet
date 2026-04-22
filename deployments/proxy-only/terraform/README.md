# Parapet Terraform Deployment

Deploy Parapet RPC proxy to DigitalOcean with automatic HTTPS, rate limiting, and security best practices.

## Features

- 🚀 **Dual deployment modes** - Docker (easy, portable) or Native (max performance)
- ✅ **Pre-built binaries** - Fast deployment (~1 minute vs 7 minutes building from source)
- 🔒 **Security hardened** - Non-root user, firewall rules, IP allowlisting
- 🔐 **Automatic HTTPS** - Let's Encrypt SSL with Caddy
- 🚦 **Rate limiting** - Per-wallet request limits with optional Redis
- 🎯 **Wallet allowlisting** - Restrict RPC access to specific wallets
- ☁️ **DigitalOcean** - Optimized for DigitalOcean droplets

## Deployment Modes

Choose between two deployment modes based on your needs. See [DEPLOYMENT_COMPARISON.md](DEPLOYMENT_COMPARISON.md) for detailed analysis.

### Docker Mode (Default) - Optimized for Ease

```hcl
deployment_mode = "docker"
```

**Pros:**

- Easy setup and updates via container images
- Portable across environments
- Dependency isolation
- Perfect for open-source distribution
- Optimized with host networking and resource limits

**Cons:**

- ~2-5% network latency overhead (minimal for most use cases)
- ~10-20MB extra memory usage

**Best for:** Development, staging, open-source users, <5000 req/s

### Native Mode - Optimized for Performance

```hcl
deployment_mode = "native"
```

**Pros:**

- Maximum performance (~2-5% lower latency than Docker)
- Direct hardware access, no containerization overhead
- Minimal memory footprint
- Systemd security hardening

**Cons:**

- Less portable (platform-specific binaries)
- Slightly more complex dependency management

**Best for:** Production, high-throughput (>5000 req/s), latency-critical applications

## Quick Start

### 1. Prerequisites

- [Terraform](https://www.terraform.io/downloads.html) installed
- DigitalOcean or AWS account
- API token/credentials
- (Optional) Domain name for HTTPS

### 2. Configure Variables

```bash
cd terraform/digitalocean  # or terraform/aws

# Copy example config
cp ../terraform.tfvars.example terraform.tfvars

# Edit with your values
nano terraform.tfvars
```

**Minimum required variables:**

```hcl
# DigitalOcean
do_token = "your_token_here"
upstream_rpc_url = "https://mainnet.helius-rpc.com/?api-key=YOUR_KEY"

# AWS  
aws_access_key = "your_access_key"
aws_secret_key = "your_secret_key"
upstream_rpc_url = "https://mainnet.helius-rpc.com/?api-key=YOUR_KEY"
```

### 3. Deploy

```bash
# Initialize Terraform
terraform init

# Preview changes
terraform plan

# Deploy
terraform apply
```

### 4. Test

**Without HTTPS (port 8899):**

```bash
curl -X POST http://YOUR_IP:8899 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
```

**With HTTPS (port 443):**

```bash
curl -X POST https://rpc.yourdomain.com \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
```

## Configuration Options

### Deployment Mode

```hcl
# Choose deployment mode
deployment_mode = "docker"  # or "native" for max performance
```

See "Deployment Modes" section above for detailed comparison.

### Security & Access Control

```hcl
# Restrict SSH access to your IP
ssh_allowed_ips = ["203.0.113.5/32"]

# Restrict HTTPS/RPC access
https_allowed_ips = ["203.0.113.0/24"]

# Allowlist specific Solana wallets
allowlisted_wallets = "wallet1,wallet2,wallet3"
```

### HTTPS Setup

```hcl
enable_https = true
domain_name = "rpc.yourdomain.com"  
email = "your@email.com"
```

**Important:** Point your DNS A record to the server IP before deploying!

### Rate Limiting

```hcl
enable_rate_limiting = true
default_requests_per_month = 10000
redis_enabled = true  # Use managed Redis for distributed rate limiting
```

### Environment Variables (Recommended for Secrets)

Instead of storing secrets in `terraform.tfvars`, use environment variables:

```bash
export TF_VAR_do_token="your_token_here"
export TF_VAR_upstream_rpc_url="https://mainnet.helius-rpc.com/?api-key=YOUR_KEY"
export TF_VAR_ssh_allowed_ips='["203.0.113.5/32"]'

terraform apply
```

See [ENV_VARS_EXAMPLE.md](ENV_VARS_EXAMPLE.md) for complete guide.

## Architecture

### Deployment Flow

1. **GitHub Actions** builds release binary on every push/tag
2. **Binary uploaded** to GitHub Releases as artifact
3. **Terraform provisions** cloud resources (droplet/instance, firewall, etc.)
4. **Cloud-init downloads** pre-built binary from GitHub
5. **Systemd starts** service as non-privileged user
6. **Caddy (optional)** provides automatic HTTPS with Let's Encrypt

### Deployment Time

- **With pre-built binaries**: ~1 minute
- **Building from source**: ~7 minutes

### Security Features

- ✅ Service runs as dedicated `solshield` user (not root)
- ✅ Systemd security hardening (`NoNewPrivileges`, `ProtectSystem`, etc.)
- ✅ Firewall restricts access to SSH (22) and HTTPS (443) only
- ✅ IP allowlisting at both firewall and application level
- ✅ Optional wallet allowlisting for RPC access

## Production Recommendations

1. **Choose deployment mode** - Use `deployment_mode = "native"` for production (max performance)
2. **Use HTTPS** - Enable `enable_https = true` with a real domain
3. **Restrict IPs** - Set `ssh_allowed_ips` and `https_allowed_ips` to your networks
4. **Enable rate limiting** - Protect against abuse
5. **Use environment variables** - Keep secrets out of `.tfvars` files
6. **Use managed Redis** - Set `redis_enabled = true` for distributed deployments
7. **Monitor logs** - Check `journalctl -u parapet -f`

## Performance Comparison


| Metric              | Docker (Optimized)       | Native Binary           |
| ------------------- | ------------------------ | ----------------------- |
| Latency overhead    | ~2-5%                    | Baseline (0%)           |
| Memory overhead     | +10-20MB                 | Minimal                 |
| Deployment time     | ~1 min                   | ~1-2 min                |
| Update complexity   | Very easy                | Easy                    |
| Portability         | Excellent                | Platform-specific       |
| **Recommended for** | Open-source, <5000 req/s | Production, >5000 req/s |


**Real-world impact for RPC proxy:**

- For a 10ms upstream RPC call, Docker adds ~0.2-0.5ms
- For a 100ms call, Docker adds ~2-5ms
- **Bottleneck is usually upstream RPC latency, not Docker**

## Monitoring & Troubleshooting

### Quick Stats (Built-in Helper)

```bash
ssh root@YOUR_IP

# View statistics for last hour
solshield-stats

# View statistics for last 24 hours
solshield-stats "24 hours ago"

# Watch security events in real-time (color-coded)
solshield-watch
```

### Check service status

```bash
# Service status
systemctl status parapet

# Follow all logs
journalctl -u parapet -f

# View only blocks and alerts
journalctl -u parapet -f | grep -E "BLOCKED|ALERT"
```

### Check deployment mode

```bash
cat /opt/parapet/deployment-info.txt
```

### Docker mode specific

```bash
# Check container status
docker ps -a | grep parapet

# View container logs
docker logs parapet-rpc-proxy -f

# Check resource usage
docker stats parapet-rpc-proxy
```

### Native mode specific

```bash
# Verify binary
ls -la /opt/parapet/
file /opt/parapet/parapet-rpc-proxy

# Check user
id solshield

# View systemd hardening
systemctl show parapet | grep -E 'NoNewPrivileges|ProtectSystem'
```

### Search logs for specific events

```bash
# Find all blocks in last 24 hours
journalctl -u parapet --since "24 hours ago" | grep "BLOCKED"

# Find all alerts for specific wallet
journalctl -u parapet | grep "ABC123..."

# Count events by type
journalctl -u parapet --since today | grep -c "BLOCKED"
journalctl -u parapet --since today | grep -c "ALERT"
```

### Check cloud-init logs

```bash
tail -100 /var/log/cloud-init-output.log
```

### Test RPC locally on server

```bash
curl -X POST http://localhost:8899 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
```

See [MONITORING_LOGS.md](MONITORING_LOGS.md) for detailed log monitoring guide.

## Updating

### Update to latest release

```bash
cd terraform/digitalocean  # or aws
terraform apply  # Downloads latest binary automatically
```

### Create new release

```bash
git tag v1.0.0
git push origin v1.0.0
# GitHub Actions builds and publishes automatically
```

## Outputs

After deployment, Terraform provides:

```
droplet_ip   = "104.131.164.61"
rpc_endpoint = "https://rpc.yourdomain.com" (or http://IP:8899)
ssh_command  = "ssh root@104.131.164.61"
```

## Cost Estimates

### DigitalOcean

- **Droplet**: $18/month (2 vCPU, 2GB RAM)
- **Managed Redis** (optional): $15/month
- **Total**: ~$18-33/month

### AWS

- **EC2 t3.small**: ~$15/month
- **ElastiCache** (optional): ~$12/month
- **Total**: ~$15-27/month

## Support

For issues, see:

- [GitHub Issues](https://github.com/securecheckio/parapet/issues)
- [HTTPS Setup Guide](HTTPS_SETUP.md)
- [Environment Variables Guide](ENV_VARS_EXAMPLE.md)

## License

MIT License - see repository root for details