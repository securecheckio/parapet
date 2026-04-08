# DigitalOcean Configuration for SecureCheck SaaS
# IMPORTANT: This file contains secrets - DO NOT commit to git!

# =============================================================================
# DigitalOcean API Token
# =============================================================================
do_token = "dop_v1_5efa5730afef90122c3c1fc47faa55ea1e0255a83209b29a96c0cfda0c34b984"

# =============================================================================
# Project Configuration
# =============================================================================
project_name = "securecheck-saas"
environment  = "production"
region       = "nyc3"

# =============================================================================
# Deployment Mode: All-in-one droplet (DB + Redis + Auth API + RPC Proxy)
# =============================================================================
deployment_mode = "all-in-one"

# =============================================================================
# Droplet Configuration
# =============================================================================
app_droplet_size = "s-1vcpu-1gb"  # $6/month - 1 vCPU, 1GB RAM

# =============================================================================
# SSH Configuration
# =============================================================================
ssh_keys = ["53753488"]  # thinkpad SSH key
ssh_allowed_ips = ["0.0.0.0/0", "::/0"]

# =============================================================================
# Monitoring and Backups
# =============================================================================
enable_monitoring = true
enable_backups    = true  # +20% cost (~$5/month) but critical for production

# =============================================================================
# Database Credentials (Secure passwords generated)
# =============================================================================
db_name     = "securecheck"
db_user     = "securecheck"
db_password = "bMZ3+ZVdZGXZSGf8xD4cWgNQfedmHd6dGf4YJY0fam4="

# =============================================================================
# Redis Password (Secure password generated)
# =============================================================================
redis_password = "OkocPlooDEZb767lpCGPme8t3HZ4Ol8vS0W6S95gAoQ="

# =============================================================================
# Upstream Solana RPC (Helius)
# =============================================================================
upstream_rpc_url = "https://mainnet.helius-rpc.com/?api-key=a1dcb99c-304a-458f-b4d6-a9fcdb8eb7e3"

# =============================================================================
# REQUIRED: Domain Configuration
# This should match the domains used in App Platform (api.securecheck.io, rpc.securecheck.io)
# =============================================================================
domain     = "securecheck.io"
manage_dns = false  # Set to true if you want Terraform to manage DNS records

# Subdomains (these will be: rpc.securecheck.io and api.securecheck.io)
rpc_subdomain = "rpc"
api_subdomain = "api"

# =============================================================================
# GitHub Deployment Configuration (with authentication for private repo)
# =============================================================================
github_repo   = "https://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@github.com/securecheckio/saas-platform.git"
github_branch = "main"

# =============================================================================
# Payment Configuration (Disabled for now - can enable later)
# =============================================================================
payments_enabled       = false  # Set to true when ready to accept payments
payment_token_mint     = "7B2tQy8DwYt6aXHzt6UVDuqBB6WmykyZQodLSReQ9Wcz"  # xLABS
payment_token_name     = "xLABS"
payment_token_symbol   = "xLABS"
payment_token_logo     = "https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/7B2tQy8DwYt6aXHzt6UVDuqBB6WmykyZQodLSReQ9Wcz/logo.png"
payment_token_decimals = "6"
usdc_token_mint        = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
treasury_wallet        = ""  # Add your wallet address when enabling payments

# =============================================================================
# Credits Pricing (in token lamports, 6 decimals)
# =============================================================================
credits_price_small  = "10000000"   # 10 tokens = 100k requests
credits_price_medium = "50000000"   # 50 tokens = 500k requests
credits_price_large  = "100000000"  # 100 tokens = 1M requests
credits_price_xlarge = "500000000"  # 500 tokens = 5M requests

# Credits amounts (number of requests)
credits_amount_small  = "100000"   # 100k requests
credits_amount_medium = "500000"   # 500k requests
credits_amount_large  = "1000000"  # 1M requests
credits_amount_xlarge = "5000000"  # 5M requests
