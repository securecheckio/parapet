use anyhow::Result;
use redis::aio::ConnectionManager;
use std::sync::Arc;
use crate::middleware::McpRateLimiter;
use crate::ApiStateAccess;

#[derive(Clone)]
pub struct AppState {
    pub redis: Arc<Option<ConnectionManager>>,
    pub config: Arc<Config>,
    pub mcp_rate_limiter: McpRateLimiter,
}

#[derive(Clone)]
pub struct Config {
    // Server configuration
    pub server_host: String,
    pub server_port: u16,
    
    // Runtime configuration
    pub worker_threads: Option<usize>,
    pub max_concurrent_scans: usize,
    pub scans_per_hour_per_key: u32,
    
    // Redis configuration
    pub redis_url: String,
    
    // Solana configuration
    pub solana_rpc_url: String,
    pub solana_network: String,
    
    // Auth configuration
    pub authorized_wallets: Vec<String>,
    pub nonce_ttl_seconds: u64,
    
    // MCP configuration
    pub mcp_api_keys: Vec<String>,
}

impl AppState {
    pub async fn new(config: Config) -> Result<Self> {
        log::info!("🔗 Connecting to Redis: {}", config.redis_url);
        
        // Attempt Redis connection with graceful degradation
        let redis = match redis::Client::open(config.redis_url.as_str()) {
            Ok(client) => match ConnectionManager::new(client).await {
                Ok(conn) => {
                    log::info!("✅ Redis connected");
                    Some(conn)
                }
                Err(e) => {
                    log::warn!("⚠️ Redis connection failed: {}. API will start but routes requiring Redis will return 503.", e);
                    None
                }
            },
            Err(e) => {
                log::warn!("⚠️ Invalid Redis URL: {}. API will start but routes requiring Redis will return 503.", e);
                None
            }
        };
        
        log::info!("🔑 Authorized wallets: {}", config.authorized_wallets.len());
        log::info!("🚦 MCP rate limiter: {} concurrent scans, {} scans/hour per key",
            config.max_concurrent_scans,
            config.scans_per_hour_per_key
        );
        
        let config_arc = Arc::new(config);
        
        Ok(Self {
            redis: Arc::new(redis),
            config: config_arc.clone(),
            mcp_rate_limiter: McpRateLimiter::new(
                config_arc.max_concurrent_scans,
                config_arc.scans_per_hour_per_key
            ),
        })
    }
}

// Implement ApiStateAccess trait so AppState can be used with api-core routes
impl ApiStateAccess for AppState {
    fn redis(&self) -> &Arc<Option<ConnectionManager>> {
        &self.redis
    }
    
    fn config(&self) -> &Arc<Config> {
        &self.config
    }
    
    fn mcp_rate_limiter(&self) -> &McpRateLimiter {
        &self.mcp_rate_limiter
    }
}
