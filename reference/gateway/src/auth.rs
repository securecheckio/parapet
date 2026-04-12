use anyhow::{anyhow, Result};
use async_trait::async_trait;
use axum::http::HeaderMap;
use chrono::{Datelike, Timelike, Utc};
use parapet_proxy::auth::{AuthContext, AuthProvider, AuthResult};
use redis::AsyncCommands;
use sqlx::PgPool;

pub struct SaasAuthProvider {
    db: PgPool,
    redis: redis::Client,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
struct User {
    id: uuid::Uuid,
    wallet_address: String,
    tier: String,
    credits_balance: i64,
    credits_used_this_month: i64,
    credits_reset_at: chrono::DateTime<chrono::Utc>,
    blocking_threshold: i32,
}

impl SaasAuthProvider {
    pub fn new(db: PgPool, redis: redis::Client) -> Self {
        Self { db, redis }
    }

    async fn validate_api_key(&self, key: &str) -> Result<User> {
        // Hash the API key for lookup
        let key_hash = hash_api_key(key);

        // Check Redis cache FIRST using API key hash
        // This avoids the database lookup entirely on cache hits
        let cache_key = format!("auth:key:{}", key_hash);
        let mut conn = self.redis.get_multiplexed_async_connection().await?;

        if let Ok(Some(cached_json)) = conn.get::<_, Option<String>>(&cache_key).await {
            if let Ok(cached_user) = serde_json::from_str::<User>(&cached_json) {
                log::debug!("✅ Cache hit for API key (user {})", cached_user.id);
                return Ok(cached_user);
            }
        }

        // Cache miss - query database
        let user = sqlx::query_as::<_, User>(
            "SELECT id, wallet_address, tier, credits_balance, credits_used_this_month, credits_reset_at, blocking_threshold 
             FROM users WHERE api_key_hash = $1 AND active = true",
        )
        .bind(&key_hash)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| anyhow!("Invalid API key"))?;

        // Cache user data by API key hash for 5 minutes
        // Also cache by user_id to enable invalidation when user settings change
        let user_json = serde_json::to_string(&user)?;
        let _: () = conn.set_ex(&cache_key, &user_json, 300).await?;

        let user_cache_key = format!("auth:user:{}", user.id);
        let _: () = conn.set_ex(&user_cache_key, user_json, 300).await?;

        log::debug!("✅ Cached user data for {} via API key", user.id);
        Ok(user)
    }

    async fn check_rate_limit(&self, user: &User, method: &str) -> Result<(bool, i64, i64)> {
        // Check if we need to reset monthly credits
        if Utc::now() >= user.credits_reset_at {
            // Reset monthly credits
            let next_reset = Utc::now()
                .with_day(1)
                .unwrap()
                .with_hour(0)
                .unwrap()
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap()
                + chrono::Duration::days(32);
            let next_reset = next_reset.with_day(1).unwrap();

            sqlx::query(
                "UPDATE users 
                 SET credits_used_this_month = 0, credits_reset_at = $1 
                 WHERE id = $2",
            )
            .bind(next_reset)
            .bind(&user.id)
            .execute(&self.db)
            .await?;

            // User now has full balance available
            return Ok((true, user.credits_balance, 0));
        }

        // Check if user has credits available
        let available = user.credits_balance - user.credits_used_this_month;
        if available <= 0 {
            return Ok((false, 0, user.credits_used_this_month));
        }

        // Increment usage counter and get actual value with RETURNING
        let actual_used: i64 = sqlx::query_scalar(
            "UPDATE users 
             SET credits_used_this_month = credits_used_this_month + 1 
             WHERE id = $1
             RETURNING credits_used_this_month",
        )
        .bind(&user.id)
        .fetch_one(&self.db)
        .await?;

        // Calculate actual remaining credits based on DB value
        let actual_remaining = user.credits_balance - actual_used;

        // Log RPC usage for transparency (non-blocking, ignore errors)
        let user_id_for_log = user.id.clone();
        let method_for_log = method.to_string();
        let db_for_log = self.db.clone();
        tokio::spawn(async move {
            let _ = sqlx::query(
                "INSERT INTO rpc_usage_logs (user_id, method, success) 
                 VALUES ($1, $2, true)",
            )
            .bind(&user_id_for_log)
            .bind(&method_for_log)
            .execute(&db_for_log)
            .await;
        });

        // Publish stats update to Redis for WebSocket clients with ACTUAL values
        match self.redis.get_multiplexed_async_connection().await {
            Ok(mut conn) => {
                let channel = format!("user:{}:updates", user.id);
                let update = serde_json::json!({
                    "type": "stats_update",
                    "credits_balance": user.credits_balance,
                    "credits_used_this_month": actual_used,
                    "credits_remaining": actual_remaining,
                });
                let payload = serde_json::to_string(&update).unwrap();
                match conn.publish::<_, _, ()>(&channel, &payload).await {
                    Ok(_) => {
                        log::debug!(
                            "📤 Published stats update to {} (remaining: {})",
                            channel,
                            actual_remaining
                        );
                    }
                    Err(e) => {
                        log::error!("❌ Failed to publish stats update: {}", e);
                    }
                }
            }
            Err(e) => {
                log::error!("❌ Failed to get Redis connection for publish: {}", e);
            }
        }

        Ok((true, actual_remaining, actual_used))
    }
}

#[async_trait]
impl AuthProvider for SaasAuthProvider {
    async fn authenticate(&self, headers: &HeaderMap, method: &str) -> Result<AuthResult> {
        // Extract API key from Authorization header or URL path
        let api_key = extract_api_key(headers)?;

        // Validate API key and get user
        let user = self
            .validate_api_key(&api_key)
            .await
            .map_err(|e| anyhow!("Authentication failed: {}", e))?;

        // Check credits (and log usage)
        let (allowed, remaining, used) = self
            .check_rate_limit(&user, method)
            .await
            .unwrap_or((true, 0, 0));

        if !allowed {
            return Err(anyhow!(
                "Credits exhausted. Purchase more with xLABS tokens."
            ));
        }

        let context = AuthContext {
            identity: user.id.to_string(),
            wallets: vec![user.wallet_address.clone()],
            scopes: get_tier_scopes(&user.tier),
            tier: Some(user.tier.clone()),
            metadata: std::collections::HashMap::from([
                (
                    "wallet_address".into(),
                    serde_json::json!(user.wallet_address),
                ),
                ("tier".into(), serde_json::json!(user.tier)),
                (
                    "credits_balance".into(),
                    serde_json::json!(user.credits_balance),
                ),
                ("credits_remaining".into(), serde_json::json!(remaining)),
                ("credits_used".into(), serde_json::json!(used)),
                (
                    "blocking_threshold".into(),
                    serde_json::json!(user.blocking_threshold),
                ),
            ]),
        };

        Ok(AuthResult::success(context).with_quota(remaining as u64, 0))
    }

    async fn on_success(&self, context: &AuthContext, method: &str, _status: u16) -> Result<()> {
        log::info!(
            "user={} tier={} method={}",
            context.identity,
            context.tier.as_ref().unwrap_or(&"unknown".to_string()),
            method
        );
        Ok(())
    }

    async fn on_failure(
        &self,
        context: Option<&AuthContext>,
        method: &str,
        error: &str,
    ) -> Result<()> {
        if let Some(ctx) = context {
            log::warn!("user={} method={} error={}", ctx.identity, method, error);
        } else {
            log::warn!("method={} error={}", method, error);
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "saas_auth"
    }
}

fn extract_api_key(headers: &HeaderMap) -> Result<String> {
    // Try Authorization: Bearer {key}
    if let Some(auth_header) = headers.get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(key) = auth_str.strip_prefix("Bearer ") {
                return Ok(key.to_string());
            }
            // Also accept without "Bearer " prefix
            return Ok(auth_str.to_string());
        }
    }

    // Try X-API-Key header (set by proxy from query parameter)
    if let Some(api_key_header) = headers.get("X-API-Key") {
        if let Ok(key) = api_key_header.to_str() {
            return Ok(key.to_string());
        }
    }

    Err(anyhow!("Missing Authorization header"))
}

fn hash_api_key(key: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn get_tier_scopes(tier: &str) -> Vec<String> {
    match tier {
        "enterprise" => vec!["rpc:*".to_string(), "admin".to_string()],
        _ => vec!["rpc:*".to_string()],
    }
}
