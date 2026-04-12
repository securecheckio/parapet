use crate::middleware::McpRateLimiter;
use anyhow::Result;
use redis::aio::ConnectionManager;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub redis: Arc<ConnectionManager>,
    pub config: Arc<Config>,
    pub mcp_rate_limiter: McpRateLimiter,
}

#[derive(Clone)]
pub struct Config {
    pub redis_url: String,
    pub authorized_wallets: Vec<String>,
    pub solana_rpc_url: String,
}

impl AppState {
    pub async fn new(config: Config) -> Result<Self> {
        log::info!("🔗 Connecting to Redis: {}", config.redis_url);

        let client = redis::Client::open(config.redis_url.as_str())?;
        let redis = ConnectionManager::new(client).await?;

        log::info!("✅ Redis connected");
        log::info!("🔑 Authorized wallets: {}", config.authorized_wallets.len());

        // Configure MCP rate limiting to prevent API quota exhaustion
        let max_concurrent = std::env::var("MCP_MAX_CONCURRENT_SCANS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2); // Only 2 concurrent scans by default

        let scans_per_hour = std::env::var("MCP_SCANS_PER_HOUR_PER_KEY")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10); // 10 scans/hour per API key

        Ok(Self {
            redis: Arc::new(redis),
            config: Arc::new(config),
            mcp_rate_limiter: McpRateLimiter::new(max_concurrent, scans_per_hour),
        })
    }
}
