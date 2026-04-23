# Parapet Terraform Deployment

Deploy Parapet RPC proxy to DigitalOcean with automatic HTTPS, rate limiting, and security best practices.

## TL;DR - Quick Deploy

```bash
# 1. Get prerequisites
# - DigitalOcean account with WRITE-enabled API token
# - Domain hosted in DigitalOcean (for automatic DNS)
# - Upstream RPC URL (Helius, QuickNode, etc.)

# 2. Configure
cd deployments/proxy-only/terraform/digitalocean
cp ../terraform.tfvars.example terraform.tfvars
nano terraform.tfvars  # Set: do_token, upstream_rpc_url, domain_name, email, manage_dns, dns_zone

# 3. Deploy (DNS updates automatically!)
terraform init
terraform apply -var-file=terraform.tfvars

# 4. Test (wait ~60 seconds for HTTPS cert)
curl -X POST https://rpc.yourdomain.com \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'

# Security rules auto-update hourly from community feed (zero downtime!)
```

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
- DigitalOcean account with **write-enabled** API token (read-only tokens will fail)
- Upstream RPC URL (e.g., Helius, QuickNode)
- Domain name pointed to your droplet IP (required for HTTPS)

### 2. Configure Variables

```bash
cd deployments/proxy-only/terraform/digitalocean

# Copy example config
cp ../terraform.tfvars.example terraform.tfvars

# Edit with your values
nano terraform.tfvars
```

**Minimum required variables:**

```hcl
# DigitalOcean
do_token = "dop_v1_..."  # MUST have write permissions
upstream_rpc_url = "https://mainnet.helius-rpc.com/?api-key=YOUR_KEY"

# For HTTPS with automatic DNS (recommended for DO-hosted domains)
enable_https = true
domain_name = "rpc.yourdomain.com"
email = "your@email.com"
manage_dns = true           # Let Terraform handle DNS
dns_zone = "yourdomain.com" # Your DO-hosted domain

# Auto-updating security rules (recommended)
rules_source = "feed"
rules_feed_url = "https://parapet-rules.securecheck.io/community/core-protection.json"
```

**Important Notes:**

- Variable is `whitelisted_wallets` (not `allowlisted_wallets`)
- If `manage_dns = true`, your domain MUST be hosted in DigitalOcean
- If your domain is elsewhere (Cloudflare, etc.), set `manage_dns = false` and update DNS manually after deployment
- For rules configuration options, see [Security Rules Configuration](#security-rules-configuration)
- The default droplet size (`s-1vcpu-1gb`) costs ~$6/month
- Firewall only allows ports 22 (SSH), 80 (HTTP redirect), and 443 (HTTPS)
- Port 8899 is not accessible externally - use HTTPS endpoint

### 3. DNS Configuration

You have **two options** for DNS management:

#### Option A: Automatic DNS (Recommended - DigitalOcean hosted domains) ⭐

**Best choice if your domain is hosted in DigitalOcean:**

```hcl
# In terraform.tfvars
enable_https = true
domain_name = "rpc.securecheck.io"
email = "your@email.com"

# Enable automatic DNS management
manage_dns = true
dns_zone = "securecheck.io"  # Your DO-hosted domain
```

**Benefits:**

- ✅ Terraform automatically creates/updates the A record
- ✅ DNS updates immediately when droplet IP changes
- ✅ No manual steps - just run `terraform apply`
- ✅ No chicken-and-egg problem

**Then just deploy:**

```bash
terraform apply -var-file=terraform.tfvars
# DNS is automatically configured!
```

#### Option B: Manual DNS (External DNS providers)

**Only if your domain is NOT hosted in DigitalOcean** (Cloudflare, Route53, etc.):

```hcl
# In terraform.tfvars
enable_https = true
domain_name = "rpc.yourdomain.com"
email = "your@email.com"
manage_dns = false  # Don't let Terraform manage DNS
```

**Then:**

```bash
# 1. Deploy first
terraform apply -var-file=terraform.tfvars

# 2. Get the IP from output
terraform output droplet_ip

# 3. Manually update your DNS provider:
# rpc.yourdomain.com → A → <droplet_ip>
```

**Note:** Let's Encrypt will retry certificate generation every few minutes until DNS is correct.

### 4. Deploy

```bash
# Initialize Terraform
terraform init

# Preview changes
terraform plan -var-file=terraform.tfvars

# Deploy
terraform apply -var-file=terraform.tfvars
```

Deployment takes ~2-3 minutes:

- 40s: Droplet creation
- 90-120s: Cloud-init (package updates, Docker setup, Parapet deployment)
- 10-30s: Let's Encrypt certificate (if DNS is ready)

### 5. Verify Deployment

**Get deployment info:**

```bash
terraform output
```

**Test HTTPS endpoint:**

```bash
curl -X POST https://rpc.yourdomain.com \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'

# Expected response:
# {"jsonrpc":"2.0","id":1,"result":"ok"}
```

**Test locally on server (if HTTPS doesn't work yet):**

```bash
ssh root@<DROPLET_IP>
curl -X POST http://localhost:8899 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
```

**Note:** Port 8899 is NOT externally accessible by design (firewall blocks it). Only HTTPS on port 443 is publicly available.

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

# Allowlist specific Solana wallets (note: variable name is 'whitelisted_wallets')
whitelisted_wallets = "wallet1,wallet2,wallet3"
```

### HTTPS Setup

```hcl
enable_https = true
domain_name = "rpc.yourdomain.com"  
email = "your@email.com"
```

**DNS Management Options:**

**Automatic (DigitalOcean hosted domains):**

```hcl
manage_dns = true
dns_zone = "yourdomain.com"
```

Terraform will create/update the A record automatically!

**Manual (external DNS):**

- Point your DNS A record to the droplet IP **before or immediately after** deployment
- Caddy will automatically obtain a Let's Encrypt SSL certificate once DNS resolves
- If DNS isn't ready during deployment, certificate generation will retry automatically
- Check certificate status: `ssh root@<IP> "journalctl -u caddy -f"`

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

### Security Rules Configuration

Parapet supports three methods for managing security rules:

#### 1. Rules Feed (Recommended, Default)

Auto-updating rules from HTTP URLs with zero-downtime updates.

```hcl
rules_source               = "feed"
rules_feed_url             = "https://parapet-rules.securecheck.io/community/core-protection.json"
rules_feed_enabled         = true
rules_feed_poll_interval   = 3600  # Check every hour
```

**Benefits:**

- ✅ Zero-downtime rule updates (no redeployment needed)
- ✅ Instant protection against new threats
- ✅ HTTP caching for minimal bandwidth usage
- ✅ Community-maintained rulesets
- ✅ Composable (combine multiple feeds)

**Available community feeds:**

- `core-protection.json` - Built-in analyzers, no API keys required
- `helius-protection.json` - Requires HELIUS_API_KEY
- `jupiter-protection.json` - Requires JUPITER_API_KEY
- `rugcheck-protection.json` - Requires RUGCHECK_API_KEY
- `ai-agent-protection.json` - AI agent / flowstate patterns
- `advanced-patterns.json` - CPI + instruction-padding patterns
- `trading-bot-alerts.json` - Alert-first trading patterns

See [RULE_FEEDS.md](../../docs/RULE_FEEDS.md) for full documentation.

#### 2. Local Rules File

Static rules file on the server. Rules can come from:

- **Your local machine** (deployed via Terraform)
- **Default built-in rules** (if no local file specified)
- **Manual edits** (SSH to server and modify)

```hcl
rules_source = "local"

# Option A: Deploy your own rules file from local machine
local_rules_file = "../rules/my-custom-rules.json"

# Option B: Use default rules (leave empty)
# local_rules_file = ""
```

**Use when:**

- You have custom rules not suitable for public feeds
- You want full control over rule updates
- Network access to external feeds is restricted
- You keep rules in version control alongside terraform

**Example usage:**

```hcl
# In terraform.tfvars
rules_source = "local"
local_rules_file = "../rules/example-custom-rules.json"
```

See `[rules/example-custom-rules.json](../rules/example-custom-rules.json)` for the format.

**To update:** 

- **From local machine:** Edit your rules file and run `terraform apply`
- **On server:** SSH to droplet, edit `/opt/parapet/rules/default-protection.json`, then `systemctl restart parapet-rpc-proxy`

**Benefits:**

- Rules versioned in Git alongside infrastructure
- Consistent deployments across environments
- Easy rollback to previous rules
- No manual SSH copy/paste needed

#### 3. Embedded Rules

Use rules packaged in the Docker image (no external dependencies).

```hcl
rules_source = "embedded"
```

**Use when:**

- You want a fully self-contained deployment
- No external network dependencies are allowed
- You rebuild/redeploy containers regularly

**To update:** Rebuild Docker image with new rules and redeploy.

### Security Features

- ✅ Service runs as dedicated `parapet` user (not root)
- ✅ Systemd security hardening (`NoNewPrivileges`, `ProtectSystem`, etc.)
- ✅ **Dual firewall protection**: DigitalOcean cloud firewall + UFW on droplet
- ✅ Firewall restricts access to SSH (22), HTTP (80), and HTTPS (443) only
- ✅ IP allowlisting at both firewall and application level
- ✅ Optional wallet allowlisting for RPC access

**Note:** DigitalOcean Docker images come with UFW pre-configured. Cloud-init automatically opens ports 80 and 443 for HTTPS.

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
parapet-stats

# View statistics for last 24 hours
parapet-stats "24 hours ago"

# Watch security events in real-time (color-coded)
parapet-watch
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
id parapet

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

```bash
# View all outputs
terraform output

# Example output:
deployment_mode   = "docker"
droplet_id        = "566792411"
droplet_ip        = "134.209.34.9"
performance_notes = "Docker mode: ~2-5% latency overhead, excellent portability"
rpc_endpoint      = "https://rpc.securecheck.io"
ssh_command       = "ssh root@134.209.34.9"
```

**Key outputs:**

- `droplet_ip` - Use this for DNS configuration (if not using automatic DNS)
- `rpc_endpoint` - Your production RPC URL (HTTPS only)
- `ssh_command` - Quick SSH access to the server
- `dns_record` - Shows DNS management status and configuration

## Cost Estimates

### DigitalOcean

- **s-1vcpu-1gb** (default): $6/month (1 vCPU, 1GB RAM) - Good for development/low traffic
- **s-2vcpu-2gb**: $18/month (2 vCPU, 2GB RAM) - Recommended for production
- **Managed Redis** (optional): $15/month
- **Total**: ~$6-33/month depending on configuration

**Note:** The default configuration uses `s-1vcpu-1gb` which is optimized for cost. For production, consider upgrading to `s-2vcpu-2gb` by setting:

```hcl
droplet_size = "s-2vcpu-2gb"
```

### AWS

- **EC2 t3.small**: ~$15/month
- **ElastiCache** (optional): ~$12/month
- **Total**: ~$15-27/month

## Troubleshooting

### Common Issues

**1. "You are not authorized to perform this operation" (403)**

- Your DigitalOcean token is read-only
- Solution: Generate a new token with **write** permissions

**2. HTTPS not working / "Connection timed out"**

- DNS not pointed to droplet IP yet
- Let's Encrypt certificate still generating (wait 5-10 min)
- Check: `ssh root@<IP> "systemctl status caddy"` and `journalctl -u caddy -f`

**3. "Port 8899 connection refused/timeout"**

- This is expected! Port 8899 is blocked by firewall for security
- Use HTTPS endpoint instead: `https://rpc.yourdomain.com`
- For testing, SSH into server and test locally: `curl http://localhost:8899`

**3a. "HTTPS works locally but not externally"**

- UFW (local firewall) may be blocking ports 80/443
- Cloud-init should configure this automatically, but if needed:
  ```bash
  ssh root@<IP>
  ufw allow 80/tcp
  ufw allow 443/tcp
  ufw status
  ```

**4. Service failing to start**

- Check Docker container logs: `ssh root@<IP> "docker logs parapet-rpc-proxy"`
- Check for resource issues on 1vCPU droplets
- View cloud-init logs: `tail -100 /var/log/cloud-init-output.log`

**5. DNS already pointing to old IP**

- Update your DNS A record to the new droplet IP
- Wait for DNS propagation (usually 5-30 minutes)
- Caddy will automatically obtain a new certificate once DNS is correct

**6. Rule validation errors in logs**

- Some preset security rules may reference analyzer fields not available in all configurations
- These are automatically handled - check if service is running despite warnings
- Rules are loaded from `/opt/parapet/rules/` if needed to customize

### Check Deployment Status

```bash
# SSH into droplet
ssh root@$(terraform output -raw droplet_ip)

# Check all services
systemctl status parapet caddy

# Check firewall status
ufw status verbose

# View Parapet logs
docker logs parapet-rpc-proxy -f

# Test RPC locally
curl -X POST http://localhost:8899 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'

# Check SSL certificate
curl -vI https://rpc.yourdomain.com

# Verify ports are open
ss -tulpn | grep -E ':80|:443|:8899'
```

## Support

For issues, see:

- [GitHub Issues](https://github.com/securecheckio/parapet/issues)
- [HTTPS Setup Guide](HTTPS_SETUP.md)
- [Environment Variables Guide](ENV_VARS_EXAMPLE.md)

## License

MIT License - see repository root for details