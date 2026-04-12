use anyhow::Result;
use parapet_api_core::{ApiStateAccess, middleware::McpRateLimiter};
use parapet_api_core::state::Config as ApiConfig;
use redis::aio::ConnectionManager;
use sqlx::PgPool;
use std::sync::Arc;
use crate::config::PlatformConfig;
use crate::session::SessionStore;

/// Extended state that includes both API core state and platform-specific state
#[derive(Clone)]
pub struct PlatformState {
    // Core API state fields (extended, not wrapped)
    pub redis_conn_mgr: Arc<Option<ConnectionManager>>,
    pub config: Arc<ApiConfig>,
    pub mcp_rate_limiter: McpRateLimiter,
    
    // Platform-specific additions
    pub redis: redis::Client,  // For cache operations that need multiplexed connections
    pub db: PgPool,
    pub sessions: SessionStore,
    pub platform_config: Arc<PlatformConfig>,
}

// Implement ApiStateAccess so api-core routes work with PlatformState
impl ApiStateAccess for PlatformState {
    fn redis(&self) -> &Arc<Option<ConnectionManager>> {
        &self.redis_conn_mgr
    }
    
    fn config(&self) -> &Arc<ApiConfig> {
        &self.config
    }
    
    fn mcp_rate_limiter(&self) -> &McpRateLimiter {
        &self.mcp_rate_limiter
    }
}

impl PlatformState {
    pub async fn new(
        api_config: ApiConfig,
        platform_config: PlatformConfig,
    ) -> Result<Self> {
        log::info!("🔗 Initializing platform state");
        
        // Initialize Redis client for cache operations
        let redis_client = redis::Client::open(api_config.redis_url.as_str())?;
        
        // Initialize Redis ConnectionManager for core API (with graceful degradation)
        let redis_conn_mgr = match ConnectionManager::new(redis_client.clone()).await {
            Ok(conn) => {
                log::info!("✅ Redis ConnectionManager initialized");
                Some(conn)
            }
            Err(e) => {
                log::warn!("⚠️ Redis connection failed: {}. Core API routes requiring Redis will return 503.", e);
                None
            }
        };
        
        // Initialize MCP rate limiter from api_config
        log::info!("🚦 MCP rate limiter: {} concurrent scans, {} scans/hour per key",
            api_config.max_concurrent_scans,
            api_config.scans_per_hour_per_key
        );
        let mcp_rate_limiter = McpRateLimiter::new(
            api_config.max_concurrent_scans,
            api_config.scans_per_hour_per_key
        );
        
        // Initialize platform-specific state
        let db = sqlx::postgres::PgPoolOptions::new()
            .max_connections(10)
            .connect(&platform_config.database_url)
            .await?;
        
        log::info!("✅ PostgreSQL connected");
        
        // Sessions use Redis from api_config (same connection URL)
        let sessions = SessionStore::new(api_config.redis_url.clone());
        
        log::info!("✅ Platform state initialized");
        log::info!("  - Frontend CORS: {}", platform_config.frontend_url);
        log::info!("  - Payments: {}", if platform_config.payments.enabled { "enabled" } else { "disabled" });
        log::info!("  - Push notifications: {}", if platform_config.push_notifications.enabled { "enabled" } else { "disabled" });
        
        Ok(Self {
            redis_conn_mgr: Arc::new(redis_conn_mgr),
            redis: redis_client,
            config: Arc::new(api_config),
            mcp_rate_limiter,
            db,
            sessions,
            platform_config: Arc::new(platform_config),
        })
    }
}
