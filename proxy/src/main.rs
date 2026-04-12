use parapet_proxy::{config, server};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init();

    // Load configuration
    // Priority: config.toml (if exists) + env overrides, else env only (backwards compat)
    let config = if std::path::Path::new("config.toml").exists() {
        log::info!("📄 Loading configuration from config.toml");
        config::Config::from_file_with_env("config.toml")?
    } else if std::path::Path::new("proxy/config.toml").exists() {
        log::info!("📄 Loading configuration from proxy/config.toml");
        config::Config::from_file_with_env("proxy/config.toml")?
    } else {
        log::info!("📄 Loading configuration from environment variables");
        config::Config::from_env()?
    };

    // Parse bind address
    let bind_address = parse_bind_address(&config.server.bind_address);

    // Determine auth mode
    let auth_mode = match config.auth.mode.as_str() {
        "api_key" => server::AuthMode::ApiKey,
        "wallet_allowlist" => server::AuthMode::WalletAllowlist,
        _ => server::AuthMode::None,
    };

    // Convert feed sources from config format
    let rules_feed_sources = if !config.rule_feeds.sources.is_empty() {
        Some(
            config
                .rule_feeds
                .sources
                .iter()
                .map(|src| server::FeedSourceConfig {
                    url: src.url.clone(),
                    name: src.name.clone(),
                    priority: src.priority,
                    min_request_interval: src
                        .min_interval
                        .unwrap_or(config.rule_feeds.default_min_interval),
                })
                .collect(),
        )
    } else {
        None
    };

    // Create server configuration
    let server_config = server::ServerConfig {
        port: config.server.port,
        upstream_url: config.upstream.url,
        redis_url: config.redis.url,
        bind_address,
        auth_mode,
        enable_usage_tracking: config.usage.enabled,
        default_requests_per_month: config.usage.default_requests_per_month,
        allowed_wallets: config.auth.allowed_wallets,
        blocked_programs: config.security.blocked_programs,
        rules_path: config.security.rules_path,
        rule_action_override: config.security.rule_action_override,
        wasm_analyzers_path: config.wasm.analyzers_path,
        output_manager: None,
        upstream_max_concurrent: config.upstream.max_concurrent,
        upstream_delay_ms: config.upstream.delay_ms,
        upstream_timeout_secs: Some(config.upstream.timeout_secs),
        upstream_max_retries: Some(config.upstream.max_retries),
        upstream_retry_base_delay_ms: Some(config.upstream.retry_base_delay_ms),
        upstream_circuit_breaker_threshold: Some(config.upstream.circuit_breaker_threshold),
        upstream_circuit_breaker_timeout_secs: Some(config.upstream.circuit_breaker_timeout_secs),
        default_blocking_threshold: config.security.default_blocking_threshold,
        enable_escalations: config.escalations.enabled,
        rules_feed_enabled: config.rule_feeds.enabled,
        rules_feed_sources,
        rules_feed_poll_interval: config.rule_feeds.poll_interval,
    };

    // Start server
    server::start_server(server_config).await
}

fn parse_bind_address(addr: &str) -> [u8; 4] {
    let parts: Vec<&str> = addr.split('.').collect();
    if parts.len() == 4 {
        if let Ok(octets) = parts
            .iter()
            .map(|s| s.parse::<u8>())
            .collect::<Result<Vec<_>, _>>()
        {
            return [octets[0], octets[1], octets[2], octets[3]];
        }
    }
    log::warn!("Invalid bind address '{}', using 0.0.0.0", addr);
    [0, 0, 0, 0]
}
