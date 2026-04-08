use anyhow::Result;
use redis::AsyncCommands;

/// Simple per-wallet rate limiting tracker
/// Tracks requests per wallet address and enforces configurable monthly limits
pub struct UsageTracker {
    redis_client: redis::Client,
    default_monthly_limit: u64,
}

#[derive(Debug, Clone)]
pub struct UsageStats {
    pub requests_used: u64,
    pub monthly_limit: u64,
    pub remaining: u64,
}

impl UsageTracker {
    /// Create a new usage tracker
    ///
    /// # Arguments
    /// * `redis_url` - Redis connection URL
    /// * `default_monthly_limit` - Default requests per wallet per month (e.g., 10_000)
    pub fn new(redis_url: &str, default_monthly_limit: u64) -> Result<Self> {
        let redis_client = redis::Client::open(redis_url)?;
        Ok(Self {
            redis_client,
            default_monthly_limit,
        })
    }

    /// Check if a wallet has remaining requests within their limit
    /// Returns true if within limit, false if limit exceeded
    pub async fn check_rate_limit(&self, wallet_address: &str) -> Result<bool> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;

        let usage_key = format!("usage:{}:current_month", wallet_address);
        let limit_key = format!("limit:{}", wallet_address);

        // Get current usage and custom limit (if any)
        let usage: Option<u64> = conn.get(&usage_key).await?;
        let custom_limit: Option<u64> = conn.get(&limit_key).await?;

        let current_usage = usage.unwrap_or(0);
        let limit = custom_limit.unwrap_or(self.default_monthly_limit);

        if current_usage >= limit {
            log::warn!(
                "⚠️  Rate limit exceeded for {}: {}/{}",
                wallet_address,
                current_usage,
                limit
            );
            Ok(false)
        } else {
            Ok(true)
        }
    }

    /// Increment usage counter for a wallet
    /// Automatically sets 30-day expiry on first request of the month
    pub async fn increment_usage(&self, wallet_address: &str) -> Result<u64> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;

        let usage_key = format!("usage:{}:current_month", wallet_address);

        // Increment counter
        let new_count: u64 = conn.incr(&usage_key, 1).await?;

        // Set expiry to 30 days on first request (resets monthly)
        if new_count == 1 {
            let ttl = 30 * 24 * 60 * 60; // 30 days in seconds
            conn.expire::<_, ()>(&usage_key, ttl).await?;
        }

        log::debug!("📊 Usage for {}: {}", wallet_address, new_count);

        Ok(new_count)
    }

    /// Get current usage statistics for a wallet (operational API)
    #[allow(dead_code)]
    pub async fn get_usage_stats(&self, wallet_address: &str) -> Result<UsageStats> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;

        let usage_key = format!("usage:{}:current_month", wallet_address);
        let limit_key = format!("limit:{}", wallet_address);

        let usage: Option<u64> = conn.get(&usage_key).await?;
        let custom_limit: Option<u64> = conn.get(&limit_key).await?;

        let requests_used = usage.unwrap_or(0);
        let monthly_limit = custom_limit.unwrap_or(self.default_monthly_limit);
        let remaining = monthly_limit.saturating_sub(requests_used);

        Ok(UsageStats {
            requests_used,
            monthly_limit,
            remaining,
        })
    }

    /// Set a custom monthly limit for a specific wallet (operational API)
    #[allow(dead_code)]
    pub async fn set_custom_limit(&self, wallet_address: &str, limit: u64) -> Result<()> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;

        let limit_key = format!("limit:{}", wallet_address);
        conn.set::<_, _, ()>(&limit_key, limit).await?;

        log::info!(
            "✅ Set custom limit for {} to {} requests/month",
            wallet_address,
            limit
        );

        Ok(())
    }

    /// Remove custom limit for a wallet (operational API)
    #[allow(dead_code)]
    pub async fn remove_custom_limit(&self, wallet_address: &str) -> Result<()> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;

        let limit_key = format!("limit:{}", wallet_address);
        conn.del::<_, ()>(&limit_key).await?;

        log::info!(
            "✅ Removed custom limit for {} (now using default: {})",
            wallet_address,
            self.default_monthly_limit
        );

        Ok(())
    }
}
