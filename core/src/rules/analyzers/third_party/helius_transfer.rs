use crate::rules::analyzer::TransactionAnalyzer;
use crate::rules::analyzers::third_party::rate_limiter::ApiRateLimiter;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_sdk::transaction::Transaction;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Transfer activity from Helius Wallet API
#[derive(Debug, Clone, Deserialize, Serialize)]
struct TransferActivity {
    #[serde(rename = "counterparty")]
    counterparty: String,
    #[serde(rename = "amount")]
    amount: u64,
    #[serde(rename = "timestamp")]
    timestamp: i64,
    #[serde(rename = "direction")]
    direction: String, // "incoming" or "outgoing"
}

/// Cached transfer data with timestamp
#[derive(Debug, Clone)]
struct CachedTransfers {
    transfers: Vec<TransferActivity>,
    cached_at: Instant,
}

/// Helius Transfer Analyzer - detects velocity and counterparty patterns
pub struct HeliusTransferAnalyzer {
    api_key: Option<String>,
    http_client: reqwest::Client,
    cache: Arc<tokio::sync::Mutex<HashMap<String, CachedTransfers>>>,
    rate_limiter: ApiRateLimiter,
    cache_ttl: Duration,
}

impl HeliusTransferAnalyzer {
    pub fn new() -> Self {
        let api_key = std::env::var("HELIUS_API_KEY").ok();

        if api_key.is_none() {
            log::info!(
                "💡 HeliusTransferAnalyzer: HELIUS_API_KEY not set - analyzer will be disabled"
            );
        } else {
            log::info!("✅ HeliusTransferAnalyzer: API key configured");
        }

        let rate_limiter = ApiRateLimiter::from_env_or_default("HELIUS_RATE_LIMIT", 20, 60);

        Self {
            api_key,
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
            cache: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            rate_limiter,
            cache_ttl: Duration::from_secs(3600), // 1 hour cache
        }
    }

    /// Fetch wallet transfers from Helius API
    async fn get_wallet_transfers(
        &self,
        address: &str,
        hours: u32,
    ) -> Result<Vec<TransferActivity>> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| anyhow!("HELIUS_API_KEY not configured"))?;

        // Check cache first
        {
            let cache = self.cache.lock().await;
            if let Some(cached) = cache.get(address) {
                if cached.cached_at.elapsed() < self.cache_ttl {
                    log::debug!("Helius: Using cached transfers for {}", address);
                    return Ok(cached.transfers.clone());
                }
            }
        }

        // Calculate timestamp for filtering (hours ago)
        let cutoff_timestamp = chrono::Utc::now().timestamp() - (hours as i64 * 3600);

        let url = format!(
            "https://api.helius.xyz/v1/wallet/{}/transfers?api-key={}",
            address, api_key
        );

        // Acquire rate limit permit
        let _permit = self.rate_limiter.acquire().await;

        log::debug!("Fetching transfers for wallet: {}", address);

        let response = self.http_client.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(anyhow!("Helius API rate limited"));
        }

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            // Wallet has no transfers
            log::debug!("Wallet {} has no transfer history", address);
            return Ok(Vec::new());
        }

        if !response.status().is_success() {
            return Err(anyhow!("Helius API error: {}", response.status()));
        }

        let transfers: Vec<TransferActivity> = response.json().await?;

        // Filter to recent transfers only
        let recent_transfers: Vec<TransferActivity> = transfers
            .into_iter()
            .filter(|t| t.timestamp >= cutoff_timestamp)
            .collect();

        // Update cache
        let mut cache = self.cache.lock().await;
        cache.insert(
            address.to_string(),
            CachedTransfers {
                transfers: recent_transfers.clone(),
                cached_at: Instant::now(),
            },
        );

        Ok(recent_transfers)
    }

    /// Analyze transfer velocity (outgoing transfers per hour)
    fn analyze_velocity(&self, transfers: &[TransferActivity]) -> (u32, u32, bool) {
        let outgoing: Vec<&TransferActivity> = transfers
            .iter()
            .filter(|t| t.direction == "outgoing")
            .collect();

        let outgoing_count = outgoing.len() as u32;

        // Count max transfers to same address
        let mut counterparty_counts: HashMap<String, u32> = HashMap::new();
        for transfer in &outgoing {
            *counterparty_counts
                .entry(transfer.counterparty.clone())
                .or_insert(0) += 1;
        }

        let max_to_same_address = counterparty_counts.values().max().copied().unwrap_or(0);

        // High velocity: >10 outgoing tx/hour to same address
        let is_high_velocity = max_to_same_address > 10;

        (outgoing_count, max_to_same_address, is_high_velocity)
    }

    /// Analyze counterparty patterns
    fn analyze_patterns(&self, transfers: &[TransferActivity]) -> (String, f32) {
        let outgoing: Vec<&TransferActivity> = transfers
            .iter()
            .filter(|t| t.direction == "outgoing")
            .collect();

        if outgoing.is_empty() {
            return (String::new(), 0.0);
        }

        // Count transfers per counterparty
        let mut counterparty_counts: HashMap<String, u32> = HashMap::new();
        for transfer in &outgoing {
            *counterparty_counts
                .entry(transfer.counterparty.clone())
                .or_insert(0) += 1;
        }

        // Find top counterparty
        let top_counterparty = counterparty_counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(addr, _)| addr.clone())
            .unwrap_or_default();

        // Calculate concentration (top counterparty / total outgoing)
        let top_count = counterparty_counts
            .get(&top_counterparty)
            .copied()
            .unwrap_or(0);
        let concentration = if outgoing.len() > 0 {
            top_count as f32 / outgoing.len() as f32
        } else {
            0.0
        };

        (top_counterparty, concentration)
    }
}

#[async_trait::async_trait]
impl TransactionAnalyzer for HeliusTransferAnalyzer {
    fn name(&self) -> &str {
        "helius_transfer"
    }

    fn fields(&self) -> Vec<String> {
        vec![
            "outgoing_tx_per_hour".to_string(),
            "max_transfers_to_same_address".to_string(),
            "is_high_velocity".to_string(),
            "top_counterparty".to_string(),
            "counterparty_concentration".to_string(),
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

        // Fetch recent transfers (last 1 hour)
        let transfers = match self.get_wallet_transfers(&fee_payer, 1).await {
            Ok(t) => t,
            Err(e) => {
                log::warn!("Helius Transfer API call failed for {}: {}", fee_payer, e);
                return Ok(HashMap::new()); // Graceful degradation
            }
        };

        // Analyze velocity
        let (outgoing_count, max_to_same, is_high_velocity) = self.analyze_velocity(&transfers);

        // Analyze patterns
        let (top_counterparty, concentration) = self.analyze_patterns(&transfers);

        // Build fields
        let mut fields = HashMap::new();
        fields.insert("outgoing_tx_per_hour".to_string(), json!(outgoing_count));
        fields.insert(
            "max_transfers_to_same_address".to_string(),
            json!(max_to_same),
        );
        fields.insert("is_high_velocity".to_string(), json!(is_high_velocity));
        fields.insert("top_counterparty".to_string(), json!(top_counterparty));
        fields.insert(
            "counterparty_concentration".to_string(),
            json!(concentration),
        );

        Ok(fields)
    }

    fn is_available(&self) -> bool {
        self.api_key.is_some()
    }

    fn estimated_latency_ms(&self) -> u64 {
        150
    }

    fn recommended_delay_ms(&self) -> Option<u64> {
        if !self.is_available() {
            return None;
        }
        Some(3000) // 20 requests per 60 seconds
    }
}

impl Default for HeliusTransferAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
