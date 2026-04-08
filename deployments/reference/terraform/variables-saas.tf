variable "do_token" {
  description = "DigitalOcean API token"
  type        = string
  sensitive   = true
}

variable "project_name" {
  description = "Project name"
  type        = string
  default     = "securecheck-saas"
}

variable "environment" {
  description = "Environment (prod, staging, dev)"
  type        = string
  default     = "prod"
}

variable "region" {
  description = "DigitalOcean region"
  type        = string
  default     = "nyc3"
}

variable "deployment_mode" {
  description = "Deployment mode: all-in-one (single droplet), separate-db (2 droplets), managed-db (app + managed databases)"
  type        = string
  default     = "all-in-one"
  
  validation {
    condition     = contains(["all-in-one", "separate-db", "managed-db"], var.deployment_mode)
    error_message = "The deployment_mode must be one of: all-in-one, separate-db, managed-db."
  }
}

# Droplet sizes
variable "app_droplet_size" {
  description = "App droplet size (reverse-proxy + auth-api)"
  type        = string
  default     = "s-2vcpu-4gb"  # $24/month
}

variable "db_droplet_size" {
  description = "Database droplet size (only if separate-db mode)"
  type        = string
  default     = "s-2vcpu-4gb"  # $24/month
}

# Managed database options
variable "create_managed_db" {
  description = "Create managed PostgreSQL database (only if managed-db mode)"
  type        = bool
  default     = false
}

variable "create_managed_redis" {
  description = "Create managed Redis (only if managed-db mode)"
  type        = bool
  default     = false
}

variable "managed_db_size" {
  description = "Managed PostgreSQL database size"
  type        = string
  default     = "db-s-1vcpu-1gb"  # $15/month
}

variable "managed_db_nodes" {
  description = "Number of managed PostgreSQL nodes"
  type        = number
  default     = 1
}

variable "managed_redis_size" {
  description = "Managed Redis size"
  type        = string
  default     = "db-s-1vcpu-1gb"  # $15/month
}

variable "managed_db_host" {
  description = "Managed database host (if using external managed DB)"
  type        = string
  default     = ""
}

variable "managed_db_port" {
  description = "Managed database port"
  type        = string
  default     = "25060"
}

variable "managed_redis_host" {
  description = "Managed Redis host (if using external managed Redis)"
  type        = string
  default     = ""
}

variable "managed_redis_port" {
  description = "Managed Redis port"
  type        = string
  default     = "25061"
}

# SSH configuration
variable "ssh_keys" {
  description = "List of SSH key IDs"
  type        = list(string)
}

variable "ssh_allowed_ips" {
  description = "IP addresses allowed for SSH"
  type        = list(string)
  default     = ["0.0.0.0/0", "::/0"]
}

# Monitoring
variable "enable_monitoring" {
  description = "Enable DigitalOcean monitoring"
  type        = bool
  default     = true
}

variable "enable_backups" {
  description = "Enable automatic backups"
  type        = bool
  default     = true
}

# Database configuration
variable "db_name" {
  description = "PostgreSQL database name"
  type        = string
  default     = "securecheck"
}

variable "db_user" {
  description = "PostgreSQL database user"
  type        = string
  default     = "securecheck"
}

variable "db_password" {
  description = "PostgreSQL database password"
  type        = string
  sensitive   = true
}

variable "redis_password" {
  description = "Redis password"
  type        = string
  sensitive   = true
  default     = ""
}

# Application configuration
variable "upstream_rpc_url" {
  description = "Upstream Solana RPC URL (Helius, QuickNode, etc.)"
  type        = string
}

variable "domain" {
  description = "Domain name for the service"
  type        = string
  default     = ""
}

variable "manage_dns" {
  description = "Manage DNS records in DigitalOcean"
  type        = bool
  default     = false
}

variable "rpc_subdomain" {
  description = "RPC subdomain (e.g., 'rpc' for rpc.example.com)"
  type        = string
  default     = "rpc"
}

variable "api_subdomain" {
  description = "API subdomain (e.g., 'api' for api.example.com)"
  type        = string
  default     = "api"
}

# GitHub deployment
variable "github_repo" {
  description = "GitHub repository URL"
  type        = string
  default     = "https://github.com/securecheckio/securecheck-saas.git"
}

variable "github_branch" {
  description = "GitHub branch to deploy"
  type        = string
  default     = "main"
}

# Payment configuration
variable "payments_enabled" {
  description = "Enable payment system (set to false for free/private instances)"
  type        = bool
  default     = true
}

variable "payment_token_mint" {
  description = "Payment token mint address (xLABS for credits)"
  type        = string
  default     = "7B2tQy8DwYt6aXHzt6UVDuqBB6WmykyZQodLSReQ9Wcz"
}

variable "payment_token_name" {
  description = "Payment token display name"
  type        = string
  default     = "xLABS"
}

variable "payment_token_symbol" {
  description = "Payment token symbol"
  type        = string
  default     = "xLABS"
}

variable "payment_token_logo" {
  description = "Payment token logo URL"
  type        = string
  default     = "https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/7B2tQy8DwYt6aXHzt6UVDuqBB6WmykyZQodLSReQ9Wcz/logo.png"
}

variable "payment_token_decimals" {
  description = "Payment token decimals (6 for xLABS)"
  type        = string
  default     = "6"
}

variable "usdc_token_mint" {
  description = "USDC token mint address (for future use)"
  type        = string
  default     = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
}

variable "treasury_wallet" {
  description = "Treasury wallet address for receiving payments"
  type        = string
  sensitive   = true
}

# Credits pricing (token amounts in lamports, 6 decimals)
variable "credits_price_small" {
  description = "Price for small credits package in token lamports"
  type        = string
  default     = "10000000"
}

variable "credits_price_medium" {
  description = "Price for medium credits package in token lamports"
  type        = string
  default     = "50000000"
}

variable "credits_price_large" {
  description = "Price for large credits package in token lamports"
  type        = string
  default     = "100000000"
}

variable "credits_price_xlarge" {
  description = "Price for xlarge credits package in token lamports"
  type        = string
  default     = "500000000"
}

# Credits amounts (requests granted)
variable "credits_amount_small" {
  description = "Requests granted for small package"
  type        = string
  default     = "100000"
}

variable "credits_amount_medium" {
  description = "Requests granted for medium package"
  type        = string
  default     = "500000"
}

variable "credits_amount_large" {
  description = "Requests granted for large package"
  type        = string
  default     = "1000000"
}

variable "credits_amount_xlarge" {
  description = "Requests granted for xlarge package"
  type        = string
  default     = "5000000"
}
