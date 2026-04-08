output "droplet_ip" {
  description = "Public IP address of the Sol-Shield droplet"
  value       = digitalocean_droplet.sol_shield.ipv4_address
}

output "reserved_ip" {
  description = "Reserved IP address (if enabled)"
  value       = var.use_reserved_ip ? digitalocean_reserved_ip.sol_shield[0].ip_address : null
}

output "rpc_endpoint" {
  description = "RPC endpoint URL"
  value       = var.enable_https && var.domain_name != "" ? "https://${var.domain_name}" : "http://${var.use_reserved_ip ? digitalocean_reserved_ip.sol_shield[0].ip_address : digitalocean_droplet.sol_shield.ipv4_address}:${var.proxy_port}"
}

output "redis_connection_string" {
  description = "Redis connection string (if enabled)"
  value       = var.redis_enabled ? digitalocean_database_cluster.redis[0].uri : null
  sensitive   = true
}

output "ssh_command" {
  description = "SSH command to connect to the droplet"
  value       = "ssh root@${digitalocean_droplet.sol_shield.ipv4_address}"
}

output "droplet_id" {
  description = "DigitalOcean droplet ID"
  value       = digitalocean_droplet.sol_shield.id
}

output "deployment_mode" {
  description = "Deployment mode (docker or native)"
  value       = var.deployment_mode
}

output "performance_notes" {
  description = "Performance characteristics of this deployment"
  value       = var.deployment_mode == "docker" ? "Docker mode: ~2-5% latency overhead, excellent portability" : "Native mode: Maximum performance, lowest latency"
}
