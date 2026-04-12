use crate::rules::analyzer::TransactionAnalyzer;
use crate::rules::analyzers::third_party::rate_limiter::ApiRateLimiter;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_sdk::transaction::Transaction;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Jupiter Token API response
#[derive(Debug, Clone, Deserialize, Serialize)]
struct JupiterTokenInfo {
    id: String,
    name: Option<String>,
    symbol: Option<String>,
    decimals: Option<u8>,
    #[serde(rename = "isVerified")]
    is_verified: Option<bool>,
    #[serde(rename = "organicScore")]
    organic_score: Option<f64>,
    #[serde(rename = "organicScoreLabel")]
    organic_score_label: Option<String>,
    #[serde(rename = "holderCount")]
    holder_count: Option<u64>,
    #[serde(rename = "createdAt")]
    created_at: Option<String>,
    #[serde(rename = "firstPool")]
    first_pool: Option<FirstPoolInfo>,
    audit: Option<AuditInfo>,
    #[serde(rename = "stats24h")]
    stats_24h: Option<TokenStats>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct FirstPoolInfo {
    #[serde(rename = "createdAt")]
    created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AuditInfo {
    #[serde(rename = "isSus")]
    is_sus: Option<bool>,
    #[serde(rename = "mintAuthorityDisabled")]
    mint_authority_disabled: Option<bool>,
    #[serde(rename = "freezeAuthorityDisabled")]
    freeze_authority_disabled: Option<bool>,
    #[serde(rename = "topHoldersPercentage")]
    top_holders_percentage: Option<f64>,
    #[serde(rename = "devBalancePercentage")]
    dev_balance_percentage: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TokenStats {
    #[serde(rename = "holderChange")]
    holder_change: Option<f64>,
    #[serde(rename = "liquidityChange")]
    liquidity_change: Option<f64>,
    #[serde(rename = "priceChange")]
    price_change: Option<f64>,
    #[serde(rename = "numOrganicBuyers")]
    num_organic_buyers: Option<u64>,
}

/// Cache entry with TTL
struct CacheEntry {
    info: JupiterTokenInfo,
    cached_at: SystemTime,
}

/// Jupiter Token Analyzer - checks token safety via Jupiter API
pub struct JupiterTokenAnalyzer {
    api_key: Option<String>,
    http_client: reqwest::Client,
    cache: Arc<tokio::sync::RwLock<HashMap<String, CacheEntry>>>,
    cache_ttl: Duration,
    rate_limiter: ApiRateLimiter,
}

impl JupiterTokenAnalyzer {
    pub fn new() -> Self {
        let api_key = std::env::var("JUPITER_API_KEY").ok();

        if api_key.is_none() {
            log::info!(
                "💡 JupiterTokenAnalyzer: JUPITER_API_KEY not set - analyzer will be disabled"
            );
        } else {
            log::info!("✅ JupiterTokenAnalyzer: API key configured");
        }

        // Configure rate limiter from env or use free tier defaults (60 req/60s)
        let rate_limiter = ApiRateLimiter::from_env_or_default(
            "JUPITER_RATE_LIMIT",
            60, // Free tier default
            60, // 60 second window
        );

        Self {
            api_key,
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .expect("Failed to create HTTP client"),
            cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(900), // 15 minute cache for new tokens
            rate_limiter,
        }
    }

    /// Fetch token info from Jupiter API (with rate limiting and retry)
    async fn fetch_token_info(&self, mint_address: &str) -> Result<JupiterTokenInfo> {
        let now = SystemTime::now();

        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(mint_address) {
                if now.duration_since(entry.cached_at).unwrap_or(Duration::MAX) < self.cache_ttl {
                    log::debug!("Jupiter: Cache hit for {}", mint_address);
                    return Ok(entry.info.clone());
                }
            }
        }

        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| anyhow!("JUPITER_API_KEY not configured"))?;

        let url = format!("https://api.jup.ag/tokens/v2/search?query={}", mint_address);

        log::debug!("Jupiter: Fetching token info for {}", mint_address);

        // Retry logic with exponential backoff on 429
        let mut attempt = 0;
        let max_retries = 3;

        loop {
            // Acquire rate limit permit (blocks if rate limited)
            let _permit = self.rate_limiter.acquire().await;

            let response = self
                .http_client
                .get(&url)
                .header("x-api-key", api_key)
                .send()
                .await?;

            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                attempt += 1;
                if attempt >= max_retries {
                    return Err(anyhow!(
                        "Jupiter API rate limited after {} retries",
                        max_retries
                    ));
                }
                ApiRateLimiter::backoff_on_429(attempt).await;
                continue;
            }

            if !response.status().is_success() {
                return Err(anyhow!("Jupiter API error: {}", response.status()));
            }

            let tokens: Vec<JupiterTokenInfo> = response.json().await?;

            let token = tokens
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!("Token not found in Jupiter"))?;

            // Store in cache
            {
                let mut cache = self.cache.write().await;
                cache.insert(
                    mint_address.to_string(),
                    CacheEntry {
                        info: token.clone(),
                        cached_at: now,
                    },
                );
            }

            return Ok(token);
        }
    }

    /// Extract SPL Token mint addresses from transaction
    fn extract_token_mints(&self, tx: &Transaction) -> Vec<String> {
        let mut mints = Vec::new();

        const SPL_TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
        const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

        for instruction in &tx.message.instructions {
            if let Some(program_id) = tx
                .message
                .account_keys
                .get(instruction.program_id_index as usize)
            {
                let prog_str = program_id.to_string();
                if prog_str == SPL_TOKEN_PROGRAM || prog_str == TOKEN_2022_PROGRAM {
                    // Token instructions typically have mint at accounts[2]
                    if let Some(&mint_idx) = instruction.accounts.get(2) {
                        if let Some(mint) = tx.message.account_keys.get(mint_idx as usize) {
                            mints.push(mint.to_string());
                        }
                    }
                }
            }
        }

        mints.sort();
        mints.dedup();
        mints
    }

    /// Calculate token age in hours from creation timestamp
    fn calculate_token_age_hours(&self, created_at: Option<&String>) -> Option<f64> {
        let created_str = created_at?;
        let created_time = chrono::DateTime::parse_from_rfc3339(created_str).ok()?;
        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(created_time);
        Some(duration.num_seconds() as f64 / 3600.0)
    }
}

#[async_trait::async_trait]
impl TransactionAnalyzer for JupiterTokenAnalyzer {
    fn name(&self) -> &str {
        "jupiter"
    }

    fn fields(&self) -> Vec<String> {
        vec![
            // Verification & Trust
            "is_verified".to_string(),
            "is_sus".to_string(),
            "organic_score".to_string(),
            "organic_score_label".to_string(),
            // Authority Safety
            "freeze_authority_disabled".to_string(),
            "mint_authority_disabled".to_string(),
            // Concentration Risks
            "top_holders_percentage".to_string(),
            "dev_balance_percentage".to_string(),
            // Activity Patterns (24h)
            "holder_change_24h".to_string(),
            "liquidity_change_24h".to_string(),
            "price_change_24h".to_string(),
            "num_organic_buyers_24h".to_string(),
            // Token Age
            "token_age_hours".to_string(),
            "is_very_new".to_string(), // <24 hours (research paper threshold)
            // Holder Count
            "holder_count".to_string(),
            // Combined Risk Flags
            "has_critical_risk".to_string(),
            "has_high_risk".to_string(),
            "rug_pull_indicators".to_string(),
        ]
    }

    async fn analyze(&self, tx: &Transaction) -> Result<HashMap<String, Value>> {
        let mut fields = HashMap::new();

        // Early return if no API key
        if self.api_key.is_none() {
            return Ok(fields);
        }

        // Extract token mints from transaction
        let mints = self.extract_token_mints(tx);

        if mints.is_empty() {
            return Ok(fields);
        }

        // Analyze first token only (most transactions involve 1-2 tokens)
        // For multiple tokens, we'd need batch API or multiple calls
        let mint = &mints[0];

        let token_info = match self.fetch_token_info(mint).await {
            Ok(info) => info,
            Err(e) => {
                log::warn!("Jupiter API call failed for {}: {}", mint, e);
                return Ok(fields); // Graceful degradation
            }
        };

        // Extract audit fields
        let audit = token_info.audit.as_ref();
        let stats = token_info.stats_24h.as_ref();

        // Verification & Trust
        fields.insert("is_verified".to_string(), json!(token_info.is_verified));
        fields.insert("is_sus".to_string(), json!(audit.and_then(|a| a.is_sus)));
        fields.insert("organic_score".to_string(), json!(token_info.organic_score));
        fields.insert(
            "organic_score_label".to_string(),
            json!(token_info.organic_score_label),
        );

        // Authority Safety
        fields.insert(
            "freeze_authority_disabled".to_string(),
            json!(audit.and_then(|a| a.freeze_authority_disabled)),
        );
        fields.insert(
            "mint_authority_disabled".to_string(),
            json!(audit.and_then(|a| a.mint_authority_disabled)),
        );

        // Concentration Risks
        let top_holders_pct = audit.and_then(|a| a.top_holders_percentage);
        let dev_balance_pct = audit.and_then(|a| a.dev_balance_percentage);
        fields.insert("top_holders_percentage".to_string(), json!(top_holders_pct));
        fields.insert("dev_balance_percentage".to_string(), json!(dev_balance_pct));

        // Activity Patterns (24h)
        fields.insert(
            "holder_change_24h".to_string(),
            json!(stats.and_then(|s| s.holder_change)),
        );
        fields.insert(
            "liquidity_change_24h".to_string(),
            json!(stats.and_then(|s| s.liquidity_change)),
        );
        fields.insert(
            "price_change_24h".to_string(),
            json!(stats.and_then(|s| s.price_change)),
        );
        fields.insert(
            "num_organic_buyers_24h".to_string(),
            json!(stats.and_then(|s| s.num_organic_buyers)),
        );

        // Token Age (from firstPool creation - research paper uses 24h window)
        let pool_created_at = token_info
            .first_pool
            .as_ref()
            .and_then(|p| p.created_at.as_ref());
        let token_age = self.calculate_token_age_hours(pool_created_at);
        fields.insert("token_age_hours".to_string(), json!(token_age));
        fields.insert(
            "is_very_new".to_string(),
            json!(token_age.map(|age| age < 24.0).unwrap_or(false)),
        );

        // Holder Count
        fields.insert("holder_count".to_string(), json!(token_info.holder_count));

        // Combined Risk Flags (based on research paper findings)
        let is_sus = audit.and_then(|a| a.is_sus).unwrap_or(false);
        let freeze_enabled = audit
            .and_then(|a| a.freeze_authority_disabled)
            .map(|disabled| !disabled)
            .unwrap_or(false);
        let high_concentration = top_holders_pct.unwrap_or(0.0) > 50.0;
        let low_organic = token_info.organic_score_label.as_deref() == Some("low");
        let holder_exodus = stats
            .and_then(|s| s.holder_change)
            .map(|change| change < -70.0)
            .unwrap_or(false);
        let liquidity_drain = stats
            .and_then(|s| s.liquidity_change)
            .map(|change| change < -80.0)
            .unwrap_or(false);

        // Critical risk: Pre-flagged or active rug pull
        let has_critical_risk = is_sus || holder_exodus || liquidity_drain;

        // High risk: Multiple red flags
        let risk_indicators = vec![
            freeze_enabled,
            high_concentration,
            low_organic,
            token_age.map(|age| age < 24.0).unwrap_or(false),
        ]
        .iter()
        .filter(|&&x| x)
        .count();

        let has_high_risk = risk_indicators >= 3;

        fields.insert("has_critical_risk".to_string(), json!(has_critical_risk));
        fields.insert("has_high_risk".to_string(), json!(has_high_risk));
        fields.insert("rug_pull_indicators".to_string(), json!(risk_indicators));

        Ok(fields)
    }

    fn is_available(&self) -> bool {
        self.api_key.is_some()
    }

    fn estimated_latency_ms(&self) -> u64 {
        100 // API call latency
    }

    fn recommended_delay_ms(&self) -> Option<u64> {
        if !self.is_available() {
            return None;
        }
        // 60 requests per 60 seconds = 1000ms between requests
        Some(1000)
    }
}

impl Default for JupiterTokenAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_fields() {
        let analyzer = JupiterTokenAnalyzer::new();
        let fields = analyzer.fields();

        assert!(fields.contains(&"is_verified".to_string()));
        assert!(fields.contains(&"freeze_authority_disabled".to_string()));
        assert!(fields.contains(&"top_holders_percentage".to_string()));
        assert!(fields.contains(&"holder_change_24h".to_string()));
        assert!(fields.contains(&"has_critical_risk".to_string()));
    }

    #[test]
    fn test_token_age_calculation() {
        let analyzer = JupiterTokenAnalyzer::new();

        // Test with recent timestamp (should be < 24 hours if run soon)
        let now = chrono::Utc::now();
        let one_hour_ago = now - chrono::Duration::hours(1);
        let timestamp = one_hour_ago.to_rfc3339();

        let age = analyzer.calculate_token_age_hours(Some(&timestamp));
        assert!(age.is_some());
        assert!(age.unwrap() > 0.9 && age.unwrap() < 1.1); // ~1 hour
    }
}
