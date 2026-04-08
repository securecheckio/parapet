output "app_droplet_ip" {
  description = "App droplet public IP address"
  value       = digitalocean_droplet.app.ipv4_address
}

output "app_droplet_private_ip" {
  description = "App droplet private IP address"
  value       = digitalocean_droplet.app.ipv4_address_private
}

output "database_droplet_ip" {
  description = "Database droplet public IP (if separate-db mode)"
  value       = var.deployment_mode == "separate-db" ? digitalocean_droplet.database[0].ipv4_address : null
}

output "database_droplet_private_ip" {
  description = "Database droplet private IP (if separate-db mode)"
  value       = var.deployment_mode == "separate-db" ? digitalocean_droplet.database[0].ipv4_address_private : null
}

output "managed_db_host" {
  description = "Managed PostgreSQL host"
  value       = var.deployment_mode == "managed-db" && var.create_managed_db ? digitalocean_database_cluster.postgres[0].host : null
}

output "managed_db_port" {
  description = "Managed PostgreSQL port"
  value       = var.deployment_mode == "managed-db" && var.create_managed_db ? digitalocean_database_cluster.postgres[0].port : null
}

output "managed_db_uri" {
  description = "Managed PostgreSQL connection URI"
  value       = var.deployment_mode == "managed-db" && var.create_managed_db ? digitalocean_database_cluster.postgres[0].uri : null
  sensitive   = true
}

output "managed_redis_host" {
  description = "Managed Redis host"
  value       = var.deployment_mode == "managed-db" && var.create_managed_redis ? digitalocean_database_cluster.redis[0].host : null
}

output "managed_redis_port" {
  description = "Managed Redis port"
  value       = var.deployment_mode == "managed-db" && var.create_managed_redis ? digitalocean_database_cluster.redis[0].port : null
}

output "managed_redis_uri" {
  description = "Managed Redis connection URI"
  value       = var.deployment_mode == "managed-db" && var.create_managed_redis ? digitalocean_database_cluster.redis[0].uri : null
  sensitive   = true
}

output "rpc_endpoint" {
  description = "RPC endpoint URL"
  value       = var.domain != "" ? "https://${var.rpc_subdomain}.${var.domain}" : "http://${digitalocean_droplet.app.ipv4_address}:8899"
}

output "api_endpoint" {
  description = "Auth API endpoint URL"
  value       = var.domain != "" ? "https://${var.api_subdomain}.${var.domain}" : "http://${digitalocean_droplet.app.ipv4_address}:3001"
}

output "ssh_command_app" {
  description = "SSH command for app droplet"
  value       = "ssh root@${digitalocean_droplet.app.ipv4_address}"
}

output "ssh_command_database" {
  description = "SSH command for database droplet (if separate-db mode)"
  value       = var.deployment_mode == "separate-db" ? "ssh root@${digitalocean_droplet.database[0].ipv4_address}" : null
}

output "deployment_summary" {
  description = "Deployment summary"
  value = {
    mode              = var.deployment_mode
    app_ip            = digitalocean_droplet.app.ipv4_address
    database_type     = var.deployment_mode == "managed-db" ? "Managed" : (var.deployment_mode == "separate-db" ? "Separate droplet" : "All-in-one")
    rpc_url           = var.domain != "" ? "https://${var.rpc_subdomain}.${var.domain}" : "http://${digitalocean_droplet.app.ipv4_address}:8899"
    api_url           = var.domain != "" ? "https://${var.api_subdomain}.${var.domain}" : "http://${digitalocean_droplet.app.ipv4_address}:3001"
    estimated_monthly_cost = var.deployment_mode == "all-in-one" ? "$24" : (var.deployment_mode == "separate-db" ? "$48" : "$54+")
  }
}
