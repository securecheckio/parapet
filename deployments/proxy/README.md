# Deployment

Infrastructure and container configurations for deploying Parapet RPC Proxy.

## Quick Start

### Automated Deployment (Recommended)

Deploy to DigitalOcean with Terraform:

```bash
cd deployment/terraform/digitalocean
cp terraform.tfvars.example terraform.tfvars
# Edit terraform.tfvars with your settings
terraform init
terraform apply
```

### Manual installation

Build the proxy from this repo (`cargo build --release` in `proxy/`), install the binary, and run under your process manager. Use `proxy/README.md` and `proxy/config.toml.example` for options; wire env/secrets the same way you would for any long-running service.

## Directory Structure

### `terraform/` - Infrastructure as Code

Deploy to cloud providers using Terraform:

- `**digitalocean/**` - DigitalOcean droplet configuration (primary)
- Automated VM provisioning, firewall, HTTPS setup
- See `terraform/README.md` for detailed instructions

### `docker/` - Container Configuration

Docker images for containerized deployments:

- `**Dockerfile**` - Multi-stage optimized build
- Used by terraform when `deployment_mode = "docker"`
- Can also be used standalone with docker-compose

### `cloud-init/` - VM Initialization Scripts

Cloud-init configurations for automated server setup:

- `**cloud-init-docker.yaml**` - Docker-based deployment
- `**cloud-init-native.yaml**` - Native binary deployment (max performance)
- `**cloud-init-legacy.yaml**` - Legacy configuration
- Used automatically by terraform

## Deployment Modes

### Docker (Recommended for Most)

- Easy updates via container images
- Portable across environments
- ~2-5% latency overhead (minimal)

```hcl
deployment_mode = "docker"
```

### Native (Maximum Performance)

- Direct binary execution
- ~2-5% lower latency than Docker
- Best for high-throughput production

```hcl
deployment_mode = "native"
```

See `terraform/DEPLOYMENT_COMPARISON.md` for detailed analysis.

## Requirements

- Terraform 1.0+
- DigitalOcean account (or other cloud provider)
- Domain name (for HTTPS)

## Documentation

- `terraform/README.md` - Full deployment guide
- `terraform/DEPLOYMENT_COMPARISON.md` - Docker vs Native comparison
- `terraform/HTTPS_SETUP.md` - SSL/TLS configuration
- `terraform/ENV_VARS_EXAMPLE.md` - Environment variables reference

