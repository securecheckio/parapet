use crate::rules::analyzer::TransactionAnalyzer;
use crate::rules::analyzers::third_party::rate_limiter::ApiRateLimiter;
use crate::rules::analyzers::third_party::redis_cache::SharedCache;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_sdk::transaction::Transaction;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Response from Helius Identity API
#[derive(Debug, Clone, Deserialize, Serialize)]
struct IdentityResponse {
    address: String,
    #[serde(rename = "type")]
    identity_type: Option<String>,
    name: Option<String>,
    category: Option<String>,
    tags: Option<Vec<String>>,
}

/// Helius Identity Analyzer - checks wallet/address reputations
pub struct HeliusIdentityAnalyzer {
    api_key: Option<String>,
    http_client: reqwest::Client,
    // Shared cache (Redis when available, in-memory fallback)
    cache: Arc<SharedCache>,
    rate_limiter: ApiRateLimiter,
}

impl HeliusIdentityAnalyzer {
    pub fn new() -> Self {
        Self::new_with_cache(None)
    }

    /// Create with optional Redis URL for shared caching
    pub fn new_with_cache(redis_url: Option<String>) -> Self {
        let api_key = std::env::var("HELIUS_API_KEY").ok();

        if api_key.is_none() {
            log::info!(
                "💡 HeliusIdentityAnalyzer: HELIUS_API_KEY not set - analyzer will be disabled"
            );
        } else {
            log::info!("✅ HeliusIdentityAnalyzer: API key configured");
        }

        // Configure rate limiter from env or use conservative defaults
        // Helius free tier: ~10k/day, but be very conservative to avoid 429s
        let rate_limiter = ApiRateLimiter::from_env_or_default(
            "HELIUS_RATE_LIMIT",
            20, // Very conservative: 20 requests per minute (~1 per 3 seconds)
            60, // 60 second window
        );

        let cache = Arc::new(SharedCache::new(redis_url));

        Self {
            api_key,
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .expect("Failed to create HTTP client"),
            cache,
            rate_limiter,
        }
    }

    /// Batch lookup identities for multiple addresses (with rate limiting and retry)
    async fn batch_identity_lookup(&self, addresses: &[String]) -> Result<Vec<IdentityResponse>> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| anyhow!("HELIUS_API_KEY not configured"))?;

        // Check cache first for all addresses
        let mut cached_results = Vec::new();
        let mut uncached_addresses = Vec::new();

        for addr in addresses {
            let cache_key = format!("helius:identity:{}", addr);
            match self.cache.get::<IdentityResponse>(&cache_key).await {
                Ok(Some(identity)) => {
                    log::debug!("Helius: Cache hit for {}", addr);
                    cached_results.push(identity);
                }
                _ => {
                    uncached_addresses.push(addr.clone());
                }
            }
        }

        // If all addresses were cached, return immediately
        if uncached_addresses.is_empty() {
            log::debug!("Helius: All {} addresses found in cache", addresses.len());
            return Ok(cached_results);
        }

        // Fetch uncached addresses from API
        let url = format!(
            "https://api.helius.xyz/v1/wallet/batch-identity?api-key={}",
            api_key
        );

        // Retry logic with exponential backoff on 429
        let mut attempt = 0;
        let max_retries = 3;

        let fresh_identities = loop {
            // Acquire rate limit permit (blocks if rate limited)
            let _permit = self.rate_limiter.acquire().await;

            let response = self
                .http_client
                .post(&url)
                .json(&serde_json::json!({
                    "addresses": uncached_addresses
                }))
                .send()
                .await?;

            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                attempt += 1;
                if attempt >= max_retries {
                    return Err(anyhow!(
                        "Helius API rate limited after {} retries",
                        max_retries
                    ));
                }
                ApiRateLimiter::backoff_on_429(attempt).await;
                continue;
            }

            if !response.status().is_success() {
                return Err(anyhow!("Helius API error: {}", response.status()));
            }

            let identities: Vec<IdentityResponse> = response.json().await?;
            break identities;
        };

        // Update cache with fresh results (permanent cache for identity data)
        for identity in &fresh_identities {
            let cache_key = format!("helius:identity:{}", identity.address);
            // Cache for 7 days (identity data rarely changes)
            if let Err(e) = self
                .cache
                .set(&cache_key, identity, Duration::from_secs(7 * 24 * 3600))
                .await
            {
                log::warn!("Failed to cache identity for {}: {}", identity.address, e);
            }
        }

        // Combine cached and fresh results
        cached_results.extend(fresh_identities);
        Ok(cached_results)
    }
}

#[async_trait::async_trait]
impl TransactionAnalyzer for HeliusIdentityAnalyzer {
    fn name(&self) -> &str {
        "helius_identity"
    }

    fn fields(&self) -> Vec<String> {
        vec![
            "signer_classifications".to_string(),
            "other_classifications".to_string(),
        ]
    }

    async fn analyze(&self, tx: &Transaction) -> Result<HashMap<String, Value>> {
        // Early return if no API key
        if self.api_key.is_none() {
            return Ok(HashMap::new());
        }

        // Extract all addresses from transaction
        let addresses: Vec<String> = tx
            .message
            .account_keys
            .iter()
            .map(|pk| pk.to_string())
            .collect();

        if addresses.is_empty() {
            return Ok(HashMap::new());
        }

        // Fetch identities (async call)
        let identities = match self.batch_identity_lookup(&addresses).await {
            Ok(ids) => ids,
            Err(e) => {
                log::warn!("Helius Identity API call failed: {}", e);
                return Ok(HashMap::new()); // Graceful degradation
            }
        };

        // Separate signer (first address) from others
        let signer_classifications: Vec<String> = identities
            .first()
            .and_then(|i| i.category.as_ref())
            .map(|c| vec![c.clone()])
            .unwrap_or_default();

        let other_classifications: Vec<String> = identities
            .iter()
            .skip(1) // Skip first (signer)
            .filter_map(|i| i.category.clone())
            .collect();

        let mut fields = HashMap::new();
        fields.insert(
            "signer_classifications".to_string(),
            json!(signer_classifications),
        );
        fields.insert(
            "other_classifications".to_string(),
            json!(other_classifications),
        );

        Ok(fields)
    }

    fn is_available(&self) -> bool {
        self.api_key.is_some()
    }

    fn estimated_latency_ms(&self) -> u64 {
        150 // API call latency
    }

    fn recommended_delay_ms(&self) -> Option<u64> {
        if !self.is_available() {
            return None;
        }
        // 20 requests per 60 seconds = 3000ms between requests
        Some(3000)
    }
}

impl Default for HeliusIdentityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
