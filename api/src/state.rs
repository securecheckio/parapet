use crate::middleware::McpRateLimiter;
use crate::ApiStateAccess;
use anyhow::Result;
use redis::aio::ConnectionManager;
use std::sync::Arc;

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
        log::info!(
            "🚦 MCP rate limiter: {} concurrent scans, {} scans/hour per key",
            config.max_concurrent_scans,
            config.scans_per_hour_per_key
        );

        if config.authorized_wallets.is_empty() {
            let allow = std::env::var("ALLOW_INSECURE_EMPTY_AUTHORIZED_WALLETS")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);
            if allow {
                log::warn!(
                    "⚠️  authorized_wallets is empty — allowing all rule-management wallets (INSECURE). \
                     Set AUTHORIZED_WALLETS or remove ALLOW_INSECURE_EMPTY_AUTHORIZED_WALLETS."
                );
            } else {
                anyhow::bail!(
                    "authorized_wallets is empty in config. This allows any wallet to manage rules (INSECURE). \
                     Set AUTHORIZED_WALLETS, or set ALLOW_INSECURE_EMPTY_AUTHORIZED_WALLETS=1 for development only."
                );
            }
        }

        let config_arc = Arc::new(config);

        Ok(Self {
            redis: Arc::new(redis),
            config: config_arc.clone(),
            mcp_rate_limiter: McpRateLimiter::new(
                config_arc.max_concurrent_scans,
                config_arc.scans_per_hour_per_key,
            ),
        })
    }

    /// Create AppState without Redis connection (for testing)
    pub fn new_without_redis(config: Config) -> Self {
        let config_arc = Arc::new(config);

        Self {
            redis: Arc::new(None),
            config: config_arc.clone(),
            mcp_rate_limiter: McpRateLimiter::new(
                config_arc.max_concurrent_scans,
                config_arc.scans_per_hour_per_key,
            ),
        }
    }
}

// Implement ApiStateAccess trait so AppState can be used with api routes
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
