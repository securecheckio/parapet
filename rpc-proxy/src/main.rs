use parapet_core::rules::analyzers::BlockedHash;
use parapet_rpc_proxy::{config, server};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init();

    // Determine config file path
    let config_path = if std::path::Path::new("config.toml").exists() {
        "config.toml"
    } else if std::path::Path::new("rpc-proxy/config.toml").exists() {
        "rpc-proxy/config.toml"
    } else {
        ""
    };

    // Load configuration
    // Priority: config.toml (if exists) + env overrides, else env only (backwards compat)
    let config = if !config_path.is_empty() {
        log::info!("📄 Loading configuration from {}", config_path);
        config::Config::from_file_with_env(config_path)?
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

    // Store rules path for reloading before moving config
    let rules_path_for_reload = config.security.rules_path.clone();

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

    // Auto-enable feeds if sources are provided (even if RULES_FEED_ENABLED not explicitly set)
    let rules_feed_enabled = config.rule_feeds.enabled || rules_feed_sources.is_some();

    // Create server configuration
    let server_config = server::ServerConfig {
        port: config.server.port,
        upstream_url: config.upstream.primary_url(),
        upstream_endpoint_configs: config.upstream.ordered_upstream_http_settings(),
        upstream_strategy: config.upstream.strategy.clone(),
        upstream_smart_max_slot_lag: config.upstream.smart_max_slot_lag,
        rpc_allowed_methods: config.security.allowed_methods.clone(),
        rpc_blocked_methods: config.security.blocked_methods.clone(),
        redis_url: config.redis.url,
        bind_address,
        auth_mode,
        enable_usage_tracking: config.usage.enabled,
        default_requests_per_month: config.usage.default_requests_per_month,
        allowed_wallets: config.auth.allowed_wallets,
        blocked_programs: config.security.blocked_programs,
        blocked_hashes: config.security.blocked_hashes.map(|entries| {
            entries
                .into_iter()
                .map(|entry| BlockedHash {
                    program_id: entry.program_id,
                    hash: entry.hash,
                })
                .collect()
        }),
        blocked_program_feeds: config.security.blocked_program_feeds,
        feed_poll_interval_secs: config.security.feed_poll_interval_secs,
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
        rules_feed_enabled,
        rules_feed_sources,
        rules_feed_poll_interval: config.rule_feeds.poll_interval,
        enable_activity_feed: config.activity_feed.enabled,
        activity_feed_min_risk_score: config.activity_feed.min_risk_score,
        activity_feed_max_events_per_wallet: config.activity_feed.max_events_per_wallet,
        activity_feed_ttl_seconds: config.activity_feed.ttl_seconds,
        network: config.network.network,
        prefetch_alts: config.network.prefetch_alts,
        alt_cache_ttl_secs: config.network.alt_cache_ttl_secs,
    };

    // Store config path for reloading
    let config_path = Arc::new(config_path.to_string());
    let rules_path = Arc::new(rules_path_for_reload);

    // Start server (returns the rule engine handle for hot-reloading)
    let (server_handle, rule_engine) = server::start_server_with_reload(server_config).await?;

    // Spawn signal handler for SIGHUP (Unix only)
    #[cfg(unix)]
    {
        let config_path = Arc::clone(&config_path);
        let rules_path = Arc::clone(&rules_path);
        let rule_engine = Arc::clone(&rule_engine);

        tokio::spawn(async move {
            use tokio::signal::unix::{signal, SignalKind};

            let mut sighup =
                signal(SignalKind::hangup()).expect("Failed to register SIGHUP handler");

            loop {
                sighup.recv().await;
                log::info!("🔄 Received SIGHUP signal, reloading configuration...");

                if let Err(e) = reload_configuration(&config_path, &rules_path, &rule_engine).await
                {
                    log::error!("❌ Failed to reload configuration: {}", e);
                } else {
                    log::info!("✅ Configuration reloaded successfully");
                }
            }
        });

        log::info!("📡 Signal handler registered - send SIGHUP to reload config");
    }

    // Wait for server to complete
    server_handle.await?
}

/// Reload configuration without restarting the server
async fn reload_configuration(
    config_path: &str,
    rules_path: &Option<String>,
    rule_engine: &Arc<tokio::sync::RwLock<parapet_core::rules::RuleEngine>>,
) -> anyhow::Result<()> {
    // Reload config from file if it exists
    let _config = if !config_path.is_empty() {
        log::info!("📄 Reloading configuration from {}", config_path);
        config::Config::from_file_with_env(config_path)?
    } else {
        log::info!("📄 Reloading configuration from environment variables");
        config::Config::from_env()?
    };

    // Reload rules if path is specified
    if let Some(rules_file) = rules_path {
        log::info!("📋 Reloading rules from {}", rules_file);

        let mut engine = rule_engine.write().await;
        engine.load_rules_from_file(rules_file)?;

        log::info!("✅ Rules reloaded from {}", rules_file);
    }

    Ok(())
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
