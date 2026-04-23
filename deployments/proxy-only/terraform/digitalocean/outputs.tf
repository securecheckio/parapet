output "droplet_ip" {
  description = "Public IP address of the Parapet droplet"
  value       = digitalocean_droplet.parapet.ipv4_address
}

output "reserved_ip" {
  description = "Reserved IP address (if enabled)"
  value       = var.use_reserved_ip ? digitalocean_reserved_ip.parapet[0].ip_address : null
}

output "rpc_endpoint" {
  description = "RPC endpoint URL"
  value       = var.enable_https && var.domain_name != "" ? "https://${var.domain_name}" : "http://${var.use_reserved_ip ? digitalocean_reserved_ip.parapet[0].ip_address : digitalocean_droplet.parapet.ipv4_address}:${var.proxy_port}"
}

output "redis_connection_string" {
  description = "Redis connection string (if enabled)"
  value       = var.redis_enabled ? digitalocean_database_cluster.redis[0].uri : null
  sensitive   = true
}

output "ssh_command" {
  description = "SSH command to connect to the droplet"
  value       = "ssh root@${digitalocean_droplet.parapet.ipv4_address}"
}

output "droplet_id" {
  description = "DigitalOcean droplet ID"
  value       = digitalocean_droplet.parapet.id
}

output "deployment_mode" {
  description = "Deployment mode (docker or native)"
  value       = var.deployment_mode
}

output "performance_notes" {
  description = "Performance characteristics of this deployment"
  value       = var.deployment_mode == "docker" ? "Docker mode: ~2-5% latency overhead, excellent portability" : "Native mode: Maximum performance, lowest latency"
}

output "post_deployment_checks" {
  description = "Commands to verify deployment"
  value       = <<-EOT
    # Test HTTPS endpoint
    curl -X POST ${var.enable_https && var.domain_name != "" ? "https://${var.domain_name}" : "http://${digitalocean_droplet.parapet.ipv4_address}:${var.proxy_port}"} \
      -H "Content-Type: application/json" \
      -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
    
    # SSH and check services
    ssh root@${digitalocean_droplet.parapet.ipv4_address}
    systemctl status parapet caddy
    ufw status verbose
    docker logs parapet-rpc-proxy
  EOT
}

output "dns_record" {
  description = "DNS record configuration"
  value = var.manage_dns && var.domain_name != "" ? {
    managed = true
    zone    = var.dns_zone
    record  = var.domain_name
    type    = "A"
    value   = var.use_reserved_ip ? digitalocean_reserved_ip.parapet[0].ip_address : digitalocean_droplet.parapet.ipv4_address
    ttl     = 300
    } : {
    managed = false
    message = "DNS not managed by Terraform. Please update manually: ${var.domain_name} -> ${digitalocean_droplet.parapet.ipv4_address}"
  }
}

output "rules_configuration" {
  description = "Security rules configuration"
  value = var.rules_source == "feed" ? {
    source        = "HTTP feed (auto-updates)"
    feed_url      = var.rules_feed_url
    poll_interval = "${var.rules_feed_poll_interval}s (${var.rules_feed_poll_interval / 60} minutes)"
    updates       = var.rules_feed_enabled ? "enabled (zero-downtime)" : "disabled"
    documentation = "https://github.com/securecheckio/parapet-rules"
    } : var.rules_source == "local" ? {
    source        = var.local_rules_file != "" ? "Local file (deployed from: ${var.local_rules_file})" : "Local file (default rules)"
    path          = "/opt/parapet/rules/default-protection.json"
    update_method = var.local_rules_file != "" ? "Edit ${var.local_rules_file} and run 'terraform apply' OR SSH to edit /opt/parapet/rules/default-protection.json" : "SSH to droplet and edit file, then: systemctl restart parapet-rpc-proxy"
    } : {
    source        = "Embedded in container"
    update_method = "Rebuild Docker image and redeploy"
  }
}
