// Public API for parapet-api-core library
// This allows api-platform and other services to extend the core API

pub mod auth;
pub mod middleware;
pub mod routes;
pub mod state;
pub mod tx_builder;
pub mod types;

use axum::{
    routing::{delete, get, post},
    Router,
};
use redis::aio::ConnectionManager;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

/// Trait for accessing core API state fields
/// Implement this trait to allow your state to work with api-core routes
pub trait ApiStateAccess: Clone + Send + Sync + 'static {
    fn redis(&self) -> &Arc<Option<ConnectionManager>>;
    fn config(&self) -> &Arc<state::Config>;
    fn mcp_rate_limiter(&self) -> &middleware::McpRateLimiter;
}

/// Create the core API router with all routes
/// This router can be used standalone or merged into a larger application
pub fn create_router<S>(state: S) -> Router
where
    S: ApiStateAccess,
{
    Router::new()
        // Health check
        .route("/health", get(health_check::<S>))
        // MCP (Model Context Protocol) HTTP Server
        .route("/mcp/sse", get(routes::mcp::mcp_sse_handler::<S>))
        .route("/mcp/message", post(routes::mcp::mcp_message_handler::<S>))
        // Rule Management API
        .route("/api/v1/rules", post(routes::rules::create_rule::<S>))
        .route("/api/v1/rules/list", post(routes::rules::list_rules::<S>))
        .route(
            "/api/v1/rules/:rule_id",
            delete(routes::rules::delete_rule::<S>),
        )
        .route(
            "/api/v1/rules/export",
            post(routes::rules::export_rules::<S>),
        )
        .route(
            "/api/v1/rules/import",
            post(routes::rules::import_rules::<S>),
        )
        // Authentication (nonce generation)
        .route(
            "/api/v1/auth/nonce",
            post(routes::auth::generate_nonce::<S>),
        )
        // Escalation API
        .route(
            "/api/v1/escalations/:escalation_id",
            get(routes::escalations::get_escalation::<S>),
        )
        .route(
            "/api/v1/escalations/:escalation_id/approve",
            post(routes::escalations::approve_escalation::<S>),
        )
        .route(
            "/api/v1/escalations/:escalation_id/deny",
            post(routes::escalations::deny_escalation::<S>),
        )
        .route(
            "/api/v1/escalations/:escalation_id/status",
            get(routes::escalations::get_status::<S>),
        )
        .route(
            "/api/v1/escalations/pending",
            post(routes::escalations::list_pending::<S>),
        )
        // WebSocket for real-time notifications
        .route(
            "/ws/escalations",
            get(routes::websocket::escalation_websocket_handler::<S>),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state)
}

async fn health_check<S: ApiStateAccess>(
    axum::extract::State(state): axum::extract::State<S>,
) -> axum::Json<serde_json::Value> {
    let redis_status = if state.redis().is_some() {
        "connected"
    } else {
        "unavailable"
    };

    axum::Json(serde_json::json!({
        "status": "ok",
        "service": "parapet-api-core",
        "redis": redis_status
    }))
}

/// Configuration loading functions
pub use config::load_config_from_file;

pub mod config {
    use super::state::Config;
    use anyhow::{Context, Result};
    use serde::Deserialize;

    pub fn load_config_from_file(path: &str) -> Result<Config> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file from {}", path))?;

        #[derive(Deserialize)]
        struct TomlConfig {
            #[serde(default)]
            server: ServerConfig,
            #[serde(default)]
            runtime: RuntimeConfig,
            redis: RedisConfig,
            solana: SolanaConfig,
            auth: AuthConfig,
            mcp: McpConfig,
        }

        #[derive(Deserialize, Default)]
        struct ServerConfig {
            #[serde(default = "default_host")]
            host: String,
            #[serde(default = "default_port")]
            port: u16,
        }

        #[derive(Deserialize, Default)]
        struct RuntimeConfig {
            worker_threads: Option<usize>,
            #[serde(default = "default_max_concurrent_scans")]
            max_concurrent_scans: usize,
            #[serde(default = "default_scans_per_hour")]
            scans_per_hour_per_key: u32,
        }

        #[derive(Deserialize)]
        struct RedisConfig {
            url: String,
        }

        #[derive(Deserialize)]
        struct SolanaConfig {
            rpc_url: String,
            network: Option<String>,
        }

        #[derive(Deserialize)]
        struct AuthConfig {
            authorized_wallets: Vec<String>,
            nonce_ttl_seconds: Option<u64>,
        }

        #[derive(Deserialize)]
        struct McpConfig {
            api_keys: Vec<String>,
        }

        fn default_host() -> String {
            "0.0.0.0".to_string()
        }

        fn default_port() -> u16 {
            3001
        }

        fn default_max_concurrent_scans() -> usize {
            2
        }

        fn default_scans_per_hour() -> u32 {
            10
        }

        let toml_config: TomlConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse TOML config from {}", path))?;

        // Apply environment variable overrides
        let mut config = Config {
            server_host: toml_config.server.host,
            server_port: toml_config.server.port,
            worker_threads: toml_config.runtime.worker_threads,
            max_concurrent_scans: toml_config.runtime.max_concurrent_scans,
            scans_per_hour_per_key: toml_config.runtime.scans_per_hour_per_key,
            redis_url: toml_config.redis.url,
            solana_rpc_url: toml_config.solana.rpc_url,
            solana_network: toml_config
                .solana
                .network
                .unwrap_or_else(|| "mainnet-beta".to_string()),
            authorized_wallets: toml_config.auth.authorized_wallets,
            nonce_ttl_seconds: toml_config.auth.nonce_ttl_seconds.unwrap_or(300),
            mcp_api_keys: toml_config.mcp.api_keys,
        };

        if let Ok(host) = std::env::var("API_HOST") {
            config.server_host = host;
        }
        if let Ok(port) = std::env::var("API_PORT") {
            if let Ok(p) = port.parse() {
                config.server_port = p;
            }
        }
        if let Ok(threads) = std::env::var("WORKER_THREADS") {
            if let Ok(t) = threads.parse() {
                config.worker_threads = Some(t);
            }
        }
        if let Ok(redis_url) = std::env::var("REDIS_URL") {
            config.redis_url = redis_url;
        }
        if let Ok(wallets) = std::env::var("AUTHORIZED_WALLETS") {
            config.authorized_wallets = wallets
                .split(',')
                .map(|w| w.trim().to_string())
                .filter(|w| !w.is_empty())
                .collect();
        }
        if let Ok(rpc_url) = std::env::var("SOLANA_RPC_URL") {
            config.solana_rpc_url = rpc_url;
        }
        if let Ok(keys) = std::env::var("MCP_API_KEYS") {
            config.mcp_api_keys = keys
                .split(',')
                .map(|k| k.trim().to_string())
                .filter(|k| !k.is_empty())
                .collect();
        }

        Ok(config)
    }
}
