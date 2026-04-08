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

# VPC for private networking
resource "digitalocean_vpc" "saas" {
  name   = "${var.project_name}-${var.environment}-vpc"
  region = var.region
}

# Firewall for app server (reverse-proxy + auth-api)
resource "digitalocean_firewall" "app" {
  name = "${var.project_name}-${var.environment}-app"

  # HTTP/HTTPS for RPC and API
  inbound_rule {
    protocol         = "tcp"
    port_range       = "80"
    source_addresses = ["0.0.0.0/0", "::/0"]
  }

  inbound_rule {
    protocol         = "tcp"
    port_range       = "443"
    source_addresses = ["0.0.0.0/0", "::/0"]
  }

  # SSH
  inbound_rule {
    protocol         = "tcp"
    port_range       = "22"
    source_addresses = var.ssh_allowed_ips
  }

  # RPC port (8899) - behind nginx
  inbound_rule {
    protocol         = "tcp"
    port_range       = "8899"
    source_addresses = var.deployment_mode == "managed-db" ? ["0.0.0.0/0", "::/0"] : [digitalocean_vpc.saas.ip_range]
  }

  # Auth API port (3001) - behind nginx
  inbound_rule {
    protocol         = "tcp"
    port_range       = "3001"
    source_addresses = var.deployment_mode == "managed-db" ? ["0.0.0.0/0", "::/0"] : [digitalocean_vpc.saas.ip_range]
  }

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

  droplet_ids = [digitalocean_droplet.app.id]
}

# Firewall for database droplet (if not using managed)
resource "digitalocean_firewall" "database" {
  count = var.deployment_mode == "separate-db" ? 1 : 0
  name  = "${var.project_name}-${var.environment}-database"

  inbound_rule {
    protocol         = "tcp"
    port_range       = "5432"
    source_addresses = [digitalocean_vpc.saas.ip_range]
  }

  inbound_rule {
    protocol         = "tcp"
    port_range       = "6379"
    source_addresses = [digitalocean_vpc.saas.ip_range]
  }

  inbound_rule {
    protocol         = "tcp"
    port_range       = "22"
    source_addresses = var.ssh_allowed_ips
  }

  outbound_rule {
    protocol              = "tcp"
    port_range            = "1-65535"
    destination_addresses = ["0.0.0.0/0", "::/0"]
  }

  droplet_ids = [digitalocean_droplet.database[0].id]
}

# App droplet (reverse-proxy + auth-api)
resource "digitalocean_droplet" "app" {
  image    = "ubuntu-22-04-x64"
  name     = "${var.project_name}-${var.environment}-app"
  region   = var.region
  size     = var.app_droplet_size
  vpc_uuid = digitalocean_vpc.saas.id
  ssh_keys = var.ssh_keys

  monitoring = var.enable_monitoring
  backups    = var.enable_backups

  user_data = templatefile("${path.module}/cloud-init/app-saas.yaml", {
    db_host               = var.deployment_mode == "managed-db" ? var.managed_db_host : (var.deployment_mode == "separate-db" ? digitalocean_droplet.database[0].ipv4_address_private : "localhost")
    db_port               = var.deployment_mode == "managed-db" ? var.managed_db_port : "5432"
    db_name               = var.db_name
    db_user               = var.db_user
    db_password           = var.db_password
    redis_host            = var.deployment_mode == "managed-db" ? var.managed_redis_host : (var.deployment_mode == "separate-db" ? digitalocean_droplet.database[0].ipv4_address_private : "localhost")
    redis_port            = var.deployment_mode == "managed-db" ? var.managed_redis_port : "6379"
    redis_password        = var.redis_password
    upstream_rpc_url       = var.upstream_rpc_url
    payments_enabled       = var.payments_enabled
    payment_token_mint     = var.payment_token_mint
    payment_token_name     = var.payment_token_name
    payment_token_symbol   = var.payment_token_symbol
    payment_token_logo     = var.payment_token_logo
    payment_token_decimals = var.payment_token_decimals
    usdc_token_mint        = var.usdc_token_mint
    treasury_wallet        = var.treasury_wallet
    credits_price_small   = var.credits_price_small
    credits_price_medium  = var.credits_price_medium
    credits_price_large   = var.credits_price_large
    credits_price_xlarge  = var.credits_price_xlarge
    credits_amount_small  = var.credits_amount_small
    credits_amount_medium = var.credits_amount_medium
    credits_amount_large  = var.credits_amount_large
    credits_amount_xlarge = var.credits_amount_xlarge
    domain                = var.domain
    rpc_subdomain         = var.rpc_subdomain
    api_subdomain         = var.api_subdomain
    github_repo           = var.github_repo
    github_branch         = var.github_branch
  })

  tags = ["${var.environment}", "saas-app", "securecheck"]
}

# Separate database droplet (optional)
resource "digitalocean_droplet" "database" {
  count = var.deployment_mode == "separate-db" ? 1 : 0
  
  image    = "ubuntu-22-04-x64"
  name     = "${var.project_name}-${var.environment}-db"
  region   = var.region
  size     = var.db_droplet_size
  vpc_uuid = digitalocean_vpc.saas.id
  ssh_keys = var.ssh_keys

  monitoring = var.enable_monitoring
  backups    = true  # Always backup database

  user_data = templatefile("${path.module}/cloud-init/database-saas.yaml", {
    db_name        = var.db_name
    db_user        = var.db_user
    db_password    = var.db_password
    redis_password = var.redis_password
  })

  tags = ["${var.environment}", "database", "securecheck"]
}

# Managed PostgreSQL database (optional)
resource "digitalocean_database_cluster" "postgres" {
  count = var.deployment_mode == "managed-db" && var.create_managed_db ? 1 : 0

  name       = "${var.project_name}-${var.environment}-pg"
  engine     = "pg"
  version    = "15"
  size       = var.managed_db_size
  region     = var.region
  node_count = var.managed_db_nodes

  tags = ["${var.environment}", "postgresql", "securecheck"]
}

resource "digitalocean_database_db" "saas" {
  count      = var.deployment_mode == "managed-db" && var.create_managed_db ? 1 : 0
  cluster_id = digitalocean_database_cluster.postgres[0].id
  name       = var.db_name
}

resource "digitalocean_database_user" "saas" {
  count      = var.deployment_mode == "managed-db" && var.create_managed_db ? 1 : 0
  cluster_id = digitalocean_database_cluster.postgres[0].id
  name       = var.db_user
}

# Managed Redis (optional)
resource "digitalocean_database_cluster" "redis" {
  count = var.deployment_mode == "managed-db" && var.create_managed_redis ? 1 : 0

  name       = "${var.project_name}-${var.environment}-redis"
  engine     = "redis"
  version    = "7"
  size       = var.managed_redis_size
  region     = var.region
  node_count = 1

  tags = ["${var.environment}", "redis", "securecheck"]
}

# Domain records
resource "digitalocean_domain" "main" {
  count = var.domain != "" && var.manage_dns ? 1 : 0
  name  = var.domain
}

resource "digitalocean_record" "rpc" {
  count  = var.domain != "" && var.manage_dns ? 1 : 0
  domain = digitalocean_domain.main[0].id
  type   = "A"
  name   = var.rpc_subdomain
  value  = digitalocean_droplet.app.ipv4_address
  ttl    = 300
}

resource "digitalocean_record" "api" {
  count  = var.domain != "" && var.manage_dns ? 1 : 0
  domain = digitalocean_domain.main[0].id
  type   = "A"
  name   = var.api_subdomain
  value  = digitalocean_droplet.app.ipv4_address
  ttl    = 300
}

# Project
resource "digitalocean_project" "saas" {
  name        = "${var.project_name}-${var.environment}"
  description = "SecureCheck Community RPC (${var.environment})"
  purpose     = "Web Application"
  environment = var.environment

  resources = concat(
    [digitalocean_droplet.app.urn],
    var.deployment_mode == "separate-db" ? [digitalocean_droplet.database[0].urn] : [],
    var.deployment_mode == "managed-db" && var.create_managed_db ? [digitalocean_database_cluster.postgres[0].urn] : [],
    var.deployment_mode == "managed-db" && var.create_managed_redis ? [digitalocean_database_cluster.redis[0].urn] : []
  )
}
