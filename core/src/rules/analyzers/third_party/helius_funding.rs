use crate::rules::analyzer::TransactionAnalyzer;
use crate::rules::analyzers::third_party::rate_limiter::ApiRateLimiter;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_sdk::transaction::Transaction;
use std::collections::HashMap;
use std::sync::Arc;

/// Funding source from Helius Wallet API
#[derive(Debug, Clone, Deserialize, Serialize)]
struct FundingSource {
    #[serde(rename = "funder")]
    funder: String,
    #[serde(rename = "funderName")]
    funder_name: Option<String>,
    #[serde(rename = "funderType")]
    funder_type: Option<String>,
    #[serde(rename = "amount")]
    amount: u64,
    #[serde(rename = "timestamp")]
    timestamp: i64,
}

/// Helius Funding Analyzer - detects sybil attacks and bot farms
pub struct HeliusFundingAnalyzer {
    api_key: Option<String>,
    http_client: reqwest::Client,
    cache: Arc<tokio::sync::Mutex<HashMap<String, Option<FundingSource>>>>,
    rate_limiter: ApiRateLimiter,
}

impl HeliusFundingAnalyzer {
    pub fn new() -> Self {
        let api_key = std::env::var("HELIUS_API_KEY").ok();

        if api_key.is_none() {
            log::info!(
                "💡 HeliusFundingAnalyzer: HELIUS_API_KEY not set - analyzer will be disabled"
            );
        } else {
            log::info!("✅ HeliusFundingAnalyzer: API key configured");
        }

        let rate_limiter = ApiRateLimiter::from_env_or_default(
            "HELIUS_RATE_LIMIT",
            20,
            60,
        );

        Self {
            api_key,
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
            cache: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            rate_limiter,
        }
    }

    /// Fetch funding source from Helius API (cached permanently)
    async fn get_funding_source(&self, address: &str) -> Result<Option<FundingSource>> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| anyhow!("HELIUS_API_KEY not configured"))?;

        // Check cache first (funding source never changes)
        {
            let cache = self.cache.lock().await;
            if let Some(cached) = cache.get(address) {
                log::debug!("Helius: Using cached funding source for {}", address);
                return Ok(cached.clone());
            }
        }

        let url = format!(
            "https://api.helius.xyz/v1/wallet/{}/funded-by?api-key={}",
            address, api_key
        );

        // Acquire rate limit permit
        let _permit = self.rate_limiter.acquire().await;

        log::debug!("Fetching funding source for wallet: {}", address);

        let response = self
            .http_client
            .get(&url)
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(anyhow!("Helius API rate limited"));
        }

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            // Wallet has no funding source (never received SOL)
            log::debug!("Wallet {} has no funding source", address);
            let mut cache = self.cache.lock().await;
            cache.insert(address.to_string(), None);
            return Ok(None);
        }

        if !response.status().is_success() {
            return Err(anyhow!("Helius API error: {}", response.status()));
        }

        let funding: FundingSource = response.json().await?;

        // Cache permanently (funding source never changes)
        let mut cache = self.cache.lock().await;
        cache.insert(address.to_string(), Some(funding.clone()));

        Ok(Some(funding))
    }

    /// Calculate funding risk score (0-100)
    fn calculate_risk_score(&self, funding: &Option<FundingSource>) -> u32 {
        let Some(funding) = funding else {
            return 0; // No funding = no risk
        };

        let mut risk_score = 0u32;

        // Unknown funder type = +40 risk
        if funding.funder_type.is_none() || funding.funder_type.as_deref() == Some("unknown") {
            risk_score += 40;
        }

        // Recent funding (<24h) = +30 risk
        let now = chrono::Utc::now().timestamp();
        let age_hours = (now - funding.timestamp) / 3600;
        if age_hours < 24 {
            risk_score += 30;
        }

        // Small funding (<0.1 SOL = 100_000_000 lamports) = +30 risk
        if funding.amount < 100_000_000 {
            risk_score += 30;
        }

        risk_score.min(100)
    }

    /// Detect likely sybil wallet
    fn is_likely_sybil(&self, funding: &Option<FundingSource>, risk_score: u32) -> bool {
        let Some(funding) = funding else {
            return false;
        };

        // Sybil criteria: unknown funder + recent (<24h) + small amount (<0.1 SOL)
        let unknown_funder = funding.funder_type.is_none() 
            || funding.funder_type.as_deref() == Some("unknown");
        
        let now = chrono::Utc::now().timestamp();
        let age_hours = (now - funding.timestamp) / 3600;
        let recent = age_hours < 24;
        
        let small_amount = funding.amount < 100_000_000;

        // High risk score is also an indicator
        (unknown_funder && recent && small_amount) || risk_score >= 80
    }

    /// Calculate funding age in hours
    fn funding_age_hours(&self, funding: &Option<FundingSource>) -> u32 {
        let Some(funding) = funding else {
            return 0;
        };

        let now = chrono::Utc::now().timestamp();
        let age_hours = (now - funding.timestamp) / 3600;
        age_hours.max(0) as u32
    }
}

#[async_trait::async_trait]
impl TransactionAnalyzer for HeliusFundingAnalyzer {
    fn name(&self) -> &str {
        "helius_funding"
    }

    fn fields(&self) -> Vec<String> {
        vec![
            "funding_source".to_string(),
            "funding_source_type".to_string(),
            "funding_risk_score".to_string(),
            "is_likely_sybil".to_string(),
            "funding_age_hours".to_string(),
        ]
    }

    async fn analyze(&self, tx: &Transaction) -> Result<HashMap<String, Value>> {
        // Early return if no API key
        if self.api_key.is_none() {
            return Ok(HashMap::new());
        }

        // Extract fee payer (first account key)
        if tx.message.account_keys.is_empty() {
            return Ok(HashMap::new());
        }

        let fee_payer = tx.message.account_keys[0].to_string();

        // Fetch funding source (cached permanently)
        let funding = match self.get_funding_source(&fee_payer).await {
            Ok(f) => f,
            Err(e) => {
                log::warn!("Helius Funding API call failed for {}: {}", fee_payer, e);
                return Ok(HashMap::new()); // Graceful degradation
            }
        };

        // Calculate metrics
        let risk_score = self.calculate_risk_score(&funding);
        let is_sybil = self.is_likely_sybil(&funding, risk_score);
        let age_hours = self.funding_age_hours(&funding);

        // Extract funding details
        let funding_source = funding.as_ref().map(|f| f.funder.clone()).unwrap_or_default();
        let funding_type = funding
            .as_ref()
            .and_then(|f| f.funder_type.clone())
            .unwrap_or_else(|| "unknown".to_string());

        // Build fields
        let mut fields = HashMap::new();
        fields.insert("funding_source".to_string(), json!(funding_source));
        fields.insert("funding_source_type".to_string(), json!(funding_type));
        fields.insert("funding_risk_score".to_string(), json!(risk_score));
        fields.insert("is_likely_sybil".to_string(), json!(is_sybil));
        fields.insert("funding_age_hours".to_string(), json!(age_hours));

        Ok(fields)
    }

    fn is_available(&self) -> bool {
        self.api_key.is_some()
    }

    fn estimated_latency_ms(&self) -> u64 {
        100
    }

    fn recommended_delay_ms(&self) -> Option<u64> {
        if !self.is_available() {
            return None;
        }
        Some(3000) // 20 requests per 60 seconds
    }
}

impl Default for HeliusFundingAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
