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

# Use existing SSH key from DigitalOcean account
data "digitalocean_ssh_key" "existing" {
  name = "thinkpad"
}

# Droplet for Parapet
resource "digitalocean_droplet" "parapet" {
  name   = "parapet-${var.deployment_name}"
  region = var.region
  size   = var.droplet_size
  # Use docker image for docker mode, ubuntu for native mode
  image = var.deployment_mode == "docker" ? "docker-20-04" : "ubuntu-22-04-x64"

  ssh_keys = [data.digitalocean_ssh_key.existing.id]

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
    rules_source               = var.rules_source
    rules_feed_enabled         = var.rules_feed_enabled
    rules_feed_url             = var.rules_feed_url
    rules_feed_poll_interval   = var.rules_feed_poll_interval
    local_rules_content        = var.local_rules_file != "" ? file(var.local_rules_file) : ""
  })

  tags = concat(
    ["parapet", "rpc-proxy", var.deployment_mode],
    var.tags
  )
}

# Firewall rules
resource "digitalocean_firewall" "parapet" {
  name = "parapet-${var.deployment_name}"

  droplet_ids = [digitalocean_droplet.parapet.id]

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
resource "digitalocean_reserved_ip" "parapet" {
  count = var.use_reserved_ip ? 1 : 0

  region     = var.region
  droplet_id = digitalocean_droplet.parapet.id
}

# Optional: DNS Management
# Automatically creates/updates A record for your domain
data "digitalocean_domain" "main" {
  count = var.manage_dns && var.dns_zone != "" ? 1 : 0
  name  = var.dns_zone
}

locals {
  # Extract subdomain from domain_name (e.g., "rpc" from "rpc.securecheck.io")
  # If domain_name == dns_zone, use "@" for apex record
  dns_record_name = var.manage_dns && var.domain_name != "" && var.dns_zone != "" ? (
    var.domain_name == var.dns_zone ? "@" : split(".${var.dns_zone}", var.domain_name)[0]
  ) : ""

  # IP to use for DNS record (reserved IP if enabled, otherwise droplet IP)
  dns_record_ip = var.use_reserved_ip ? digitalocean_reserved_ip.parapet[0].ip_address : digitalocean_droplet.parapet.ipv4_address
}

resource "digitalocean_record" "rpc" {
  count = var.manage_dns && var.domain_name != "" && var.dns_zone != "" ? 1 : 0

  domain = data.digitalocean_domain.main[0].id
  type   = "A"
  name   = local.dns_record_name
  value  = local.dns_record_ip
  ttl    = 300 # 5 minutes for faster updates
}
