mod auth;
mod postgres_sink;

use anyhow::Result;
use parapet_proxy::server::{start_server, AuthMode, ServerConfig};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenvy::dotenv().ok();
    env_logger::init();

    log::info!("🚀 Starting SecureCheck RPC Gateway");

    // Load configuration from environment
    let upstream_url = std::env::var("UPSTREAM_RPC_URL")
        .expect("UPSTREAM_RPC_URL must be set");
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let redis_url = std::env::var("REDIS_URL")
        .expect("REDIS_URL must be set");
    let port: u16 = std::env::var("PROXY_PORT")
        .unwrap_or_else(|_| "8899".to_string())
        .parse()
        .expect("PROXY_PORT must be a valid port number");

    // Connect to database
    log::info!("📊 Connecting to PostgreSQL...");
    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");
    log::info!("✅ Database connected");

    // Run migrations (disabled - auth-api handles migrations)
    // log::info!("🔄 Running database migrations...");
    // sqlx::migrate!("../migrations")
    //     .run(&db)
    //     .await
    //     .expect("Failed to run migrations");
    log::info!("✅ Migrations handled by auth-api");

    // Connect to Redis
    log::info!("💾 Connecting to Redis...");
    let redis = redis::Client::open(redis_url.clone())
        .expect("Failed to create Redis client");
    
    // Test Redis connection
    redis.get_multiplexed_async_connection()
        .await
        .expect("Failed to connect to Redis");
    log::info!("✅ Redis connected");

    // Create SaaS auth provider
    let auth = Arc::new(auth::SaasAuthProvider::new(db.clone(), redis));

    // Create output manager with PostgreSQL sink for security events
    log::info!("📊 Configuring output manager with PostgreSQL sink...");
    let mut output_manager = parapet_proxy::output::OutputManager::new();
    
    // Add PostgreSQL sink with JSON formatter (pass Redis for notifications)
    let redis_for_sink = redis::Client::open(redis_url.clone())
        .expect("Failed to create Redis client for sink");
    let postgres_sink = Arc::new(postgres_sink::PostgresSecuritySink::new(db.clone(), redis_for_sink));
    let json_formatter = Arc::new(parapet_proxy::output::formatters::JsonLsFormatter);
    output_manager.add_pipeline(json_formatter, postgres_sink, true);
    
    log::info!("✅ Output manager configured with PostgreSQL sink");

    // Upstream rate limiting configuration
    let upstream_max_concurrent = std::env::var("UPSTREAM_MAX_CONCURRENT")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(10);
    
    let upstream_delay_ms = std::env::var("UPSTREAM_DELAY_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(100);

    // Configure server with SaaS auth and bot-essentials rules
    let config = ServerConfig {
        port,
        upstream_url,
        redis_url: Some(redis_url),
        bind_address: [0, 0, 0, 0],
        auth_mode: AuthMode::Custom(auth),
        enable_usage_tracking: false, // Handled by auth provider
        default_requests_per_month: 10_000,
        allowed_wallets: None, // Handled by auth provider
        blocked_programs: None, // Will use rules
        rules_path: std::env::var("RULES_PATH").ok().or(Some("../../parapet/proxy/rules/presets/bot-essentials.json".to_string())),
        rule_action_override: None,
        wasm_analyzers_path: None, // No WASM analyzers in SaaS by default
        output_manager: Some(Arc::new(output_manager)),
        upstream_max_concurrent,
        upstream_delay_ms,
        upstream_timeout_secs: None,
        upstream_max_retries: None,
        upstream_retry_base_delay_ms: None,
        upstream_circuit_breaker_threshold: None,
        upstream_circuit_breaker_timeout_secs: None,
        default_blocking_threshold: 70, // Fallback for SaaS (per-user threshold in auth provider)
        enable_escalations: false, // Disabled in SaaS gateway
        rules_feed_enabled: false, // Disabled in SaaS gateway (use static rules)
        rules_feed_sources: None,
        rules_feed_poll_interval: 3600,
    };

    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log::info!("🎯 SecureCheck RPC Gateway");
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log::info!("🔑 Authentication: Database-backed (SaaS)");
    log::info!("📋 Rules: bot-essentials.json (shared)");
    log::info!("🚀 Port: {}", port);
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Start OSS server with SaaS auth!
    start_server(config).await
}
