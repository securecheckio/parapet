terraform {
  required_version = ">= 1.0"

  required_providers {
    digitalocean = {
      source  = "digitalocean/digitalocean"
      version = "~> 2.0"
    }
  }
}

provider "digitalocean" {
  token = var.do_token
}

# Get all existing SSH keys in DigitalOcean account
data "digitalocean_ssh_keys" "existing" {}

# Create SSH key only if it doesn't already exist
resource "digitalocean_ssh_key" "sol_shield" {
  # Only create if no keys exist in account
  count = length(data.digitalocean_ssh_keys.existing.ssh_keys) == 0 ? 1 : 0

  name       = "parapet-${var.deployment_name}"
  public_key = file(var.ssh_public_key_path)
}

locals {
  # Use existing SSH key if available, otherwise use the newly created one
  ssh_key_fingerprint = length(data.digitalocean_ssh_keys.existing.ssh_keys) > 0 ? data.digitalocean_ssh_keys.existing.ssh_keys[0].fingerprint : digitalocean_ssh_key.sol_shield[0].fingerprint
}

# Droplet for Parapet
resource "digitalocean_droplet" "sol_shield" {
  name   = "parapet-${var.deployment_name}"
  region = var.region
  size   = var.droplet_size
  # Use docker image for docker mode, ubuntu for native mode
  image = var.deployment_mode == "docker" ? "docker-20-04" : "ubuntu-22-04-x64"

  ssh_keys = [local.ssh_key_fingerprint]

  # Choose cloud-init template based on deployment mode
  user_data = templatefile("${path.module}/../cloud-init/cloud-init-${var.deployment_mode}.yaml", {
    upstream_rpc_url           = var.upstream_rpc_url
    enable_rate_limiting       = var.enable_rate_limiting
    default_requests_per_month = var.default_requests_per_month
    whitelisted_wallets        = var.whitelisted_wallets
    proxy_port                 = var.proxy_port
    redis_enabled              = var.redis_enabled
    enable_https               = var.enable_https
    domain_name                = var.domain_name
    email                      = var.email
    https_allowed_ips          = join(" ", var.https_allowed_ips)
  })

  tags = concat(
    ["parapet", "rpc-proxy", var.deployment_mode],
    var.tags
  )
}

# Firewall rules
resource "digitalocean_firewall" "sol_shield" {
  name = "parapet-${var.deployment_name}"

  droplet_ids = [digitalocean_droplet.sol_shield.id]

  # SSH - restricted by IP
  inbound_rule {
    protocol         = "tcp"
    port_range       = "22"
    source_addresses = var.ssh_allowed_ips
  }

  # HTTPS (if enabled) - restricted by IP
  dynamic "inbound_rule" {
    for_each = var.enable_https ? [1] : []
    content {
      protocol         = "tcp"
      port_range       = "443"
      source_addresses = var.https_allowed_ips
    }
  }

  # HTTP (for Let's Encrypt validation only)
  dynamic "inbound_rule" {
    for_each = var.enable_https ? [1] : []
    content {
      protocol         = "tcp"
      port_range       = "80"
      source_addresses = ["0.0.0.0/0", "::/0"]
    }
  }

  # Allow all outbound
  outbound_rule {
    protocol              = "tcp"
    port_range            = "1-65535"
    destination_addresses = ["0.0.0.0/0", "::/0"]
  }

  outbound_rule {
    protocol              = "udp"
    port_range            = "1-65535"
    destination_addresses = ["0.0.0.0/0", "::/0"]
  }
}

# Optional: Redis for caching and rate limiting
resource "digitalocean_database_cluster" "redis" {
  count = var.redis_enabled ? 1 : 0

  name       = "parapet-redis-${var.deployment_name}"
  engine     = "redis"
  version    = "7"
  size       = var.redis_size
  region     = var.region
  node_count = 1

  tags = concat(
    ["parapet", "redis"],
    var.tags
  )
}

# Optional: Reserved IP
resource "digitalocean_reserved_ip" "sol_shield" {
  count = var.use_reserved_ip ? 1 : 0

  region     = var.region
  droplet_id = digitalocean_droplet.sol_shield.id
}
