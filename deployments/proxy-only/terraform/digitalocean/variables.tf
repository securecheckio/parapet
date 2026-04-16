variable "do_token" {
  description = "DigitalOcean API token"
  type        = string
  sensitive   = true
}

variable "deployment_name" {
  description = "Name for this deployment (used in resource names)"
  type        = string
  default     = "production"
}

variable "region" {
  description = "DigitalOcean region"
  type        = string
  default     = "nyc3"
}

variable "droplet_size" {
  description = "Droplet size (e.g., s-1vcpu-512mb-10gb, s-1vcpu-1gb, s-2vcpu-2gb)"
  type        = string
  default     = "s-1vcpu-1gb"
}

variable "ssh_public_key_path" {
  description = "Path to SSH public key for server access"
  type        = string
  default     = "~/.ssh/id_rsa.pub"
}

variable "ssh_allowed_ips" {
  description = "IP addresses allowed to SSH (CIDR notation). Example: [\"1.2.3.4/32\", \"5.6.7.0/24\"]"
  type        = list(string)
  default     = ["0.0.0.0/0"]
}

variable "https_allowed_ips" {
  description = "IP addresses allowed to access HTTPS/RPC (CIDR notation). Example: [\"1.2.3.4/32\", \"5.6.7.0/24\"]"
  type        = list(string)
  default     = ["0.0.0.0/0", "::/0"]
}

variable "upstream_rpc_url" {
  description = "Upstream Solana RPC URL (e.g., Helius, QuickNode)"
  type        = string
}

variable "proxy_port" {
  description = "Port for RPC proxy to listen on"
  type        = number
  default     = 8899
}

variable "enable_rate_limiting" {
  description = "Enable per-wallet rate limiting"
  type        = bool
  default     = false
}

variable "default_requests_per_month" {
  description = "Default monthly request limit per wallet"
  type        = number
  default     = 10000
}

variable "whitelisted_wallets" {
  description = "Comma-separated list of whitelisted wallet addresses (empty = all allowed)"
  type        = string
  default     = ""
}

variable "redis_enabled" {
  description = "Deploy managed Redis database for caching and rate limiting"
  type        = bool
  default     = false
}

variable "redis_size" {
  description = "Redis database size (e.g., db-s-1vcpu-1gb)"
  type        = string
  default     = "db-s-1vcpu-1gb"
}

variable "use_reserved_ip" {
  description = "Allocate a reserved IP address"
  type        = bool
  default     = false
}

variable "tags" {
  description = "Additional tags for resources"
  type        = list(string)
  default     = []
}

variable "domain_name" {
  description = "Domain name for HTTPS (e.g., rpc.yourdomain.com). Leave empty for HTTP-only deployment."
  type        = string
  default     = ""
}

variable "email" {
  description = "Email for Let's Encrypt notifications (required if domain_name is set)"
  type        = string
  default     = ""
}

variable "enable_https" {
  description = "Enable automatic HTTPS with Let's Encrypt via Caddy (requires domain_name)"
  type        = bool
  default     = false
}

variable "deployment_mode" {
  description = "Deployment mode: 'docker' for containerized (easy, portable) or 'native' for binary (max performance)"
  type        = string
  default     = "docker"
  
  validation {
    condition     = contains(["docker", "native"], var.deployment_mode)
    error_message = "The deployment_mode must be either 'docker' or 'native'."
  }
}
