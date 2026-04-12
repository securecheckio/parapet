mod auth;
mod middleware;
mod routes;
mod state;
mod tx_builder;
mod types;

use anyhow::Result;
use axum::{
    routing::{delete, get, post},
    Router,
};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    log::info!("🚀 Starting Sol Shield API Service");

    // Load configuration
    let config = load_config()?;

    // Initialize state
    let app_state = state::AppState::new(config).await?;

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(|| async { "OK" }))
        // MCP (Model Context Protocol) HTTP Server
        .route("/mcp/sse", get(routes::mcp::mcp_sse_handler))
        .route("/mcp/message", post(routes::mcp::mcp_message_handler))
        // Rule Management API
        .route("/api/v1/rules", post(routes::rules::create_rule))
        .route("/api/v1/rules/list", post(routes::rules::list_rules))
        .route("/api/v1/rules/:rule_id", delete(routes::rules::delete_rule))
        .route("/api/v1/rules/export", post(routes::rules::export_rules))
        .route("/api/v1/rules/import", post(routes::rules::import_rules))
        // Authentication (nonce generation)
        .route("/api/v1/auth/nonce", post(routes::auth::generate_nonce))
        // Escalation API
        .route(
            "/api/v1/escalations/:escalation_id",
            get(routes::escalations::get_escalation),
        )
        .route(
            "/api/v1/escalations/:escalation_id/approve",
            post(routes::escalations::approve_escalation),
        )
        .route(
            "/api/v1/escalations/:escalation_id/deny",
            post(routes::escalations::deny_escalation),
        )
        .route(
            "/api/v1/escalations/:escalation_id/status",
            get(routes::escalations::get_status),
        )
        .route(
            "/api/v1/escalations/pending",
            post(routes::escalations::list_pending),
        )
        // WebSocket for real-time notifications
        .route(
            "/ws/escalations",
            get(routes::websocket::escalation_websocket_handler),
        )
        .layer(cors)
        .with_state(app_state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    log::info!("📡 API listening on http://{}", addr);
    log::info!("📊 WebSocket endpoint: ws://{}/ws/escalations", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn load_config() -> Result<state::Config> {
    Ok(state::Config {
        redis_url: std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
        authorized_wallets: std::env::var("AUTHORIZED_WALLETS")
            .ok()
            .map(|s| s.split(',').map(|w| w.trim().to_string()).collect())
            .unwrap_or_default(),
        solana_rpc_url: std::env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string()),
    })
}
