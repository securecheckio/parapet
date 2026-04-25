use crate::rules::analyzer::TransactionAnalyzer;
use crate::rules::analyzers::third_party::rate_limiter::ApiRateLimiter;
use crate::rules::analyzers::third_party::redis_cache::SharedCache;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_sdk::transaction::Transaction;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

const ASSOCIATED_TOKEN_PROGRAM: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
const SPL_TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
const INITIALIZE_ACCOUNT: u8 = 1;

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

    /// Extract owners of newly created token accounts from transaction
    fn extract_newly_created_account_owners(tx: &Transaction) -> HashSet<String> {
        let mut owners = HashSet::new();
        let fee_payer = tx.message.account_keys.first().map(|k| k.to_string());

        for instruction in &tx.message.instructions {
            if let Some(program_id) = tx
                .message
                .account_keys
                .get(instruction.program_id_index as usize)
            {
                let prog_str = program_id.to_string();

                // Check ATA creation
                if prog_str == ASSOCIATED_TOKEN_PROGRAM {
                    if let Some(&owner_idx) = instruction.accounts.get(1) {
                        if let Some(owner) = tx.message.account_keys.get(owner_idx as usize) {
                            let owner_str = owner.to_string();
                            if Some(owner_str.clone()) != fee_payer {
                                owners.insert(owner_str);
                            }
                        }
                    }
                }

                // Check InitializeAccount
                if prog_str == SPL_TOKEN_PROGRAM || prog_str == TOKEN_2022_PROGRAM {
                    if let Some(&discriminator) = instruction.data.first() {
                        if discriminator == INITIALIZE_ACCOUNT {
                            if let Some(&owner_idx) = instruction.accounts.get(3) {
                                if let Some(owner) = tx.message.account_keys.get(owner_idx as usize)
                                {
                                    let owner_str = owner.to_string();
                                    if Some(owner_str.clone()) != fee_payer {
                                        owners.insert(owner_str);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        owners
    }

    /// Fetch SOL balance (in lamports) for an address via Helius RPC
    async fn get_sol_balance(&self, address: &str) -> Result<u64> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| anyhow!("HELIUS_API_KEY not configured"))?;

        // Check cache first
        let cache_key = format!("helius:sol_balance:{}", address);
        if let Ok(Some(balance)) = self.cache.get::<u64>(&cache_key).await {
            log::debug!("SOL balance cache hit for {}", address);
            return Ok(balance);
        }

        // Fetch from Helius RPC
        let url = format!(
            "https://api.helius.xyz/v0/addresses/{}/balances?api-key={}",
            address, api_key
        );

        let _permit = self.rate_limiter.acquire().await;

        let response = self.http_client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("Helius balance API error: {}", response.status()));
        }

        #[derive(Deserialize)]
        struct BalanceResponse {
            #[serde(rename = "nativeBalance")]
            native_balance: u64,
        }

        let balance_data: BalanceResponse = response.json().await?;

        // Cache for 30 seconds (balance changes frequently)
        if let Err(e) = self
            .cache
            .set(
                &cache_key,
                &balance_data.native_balance,
                Duration::from_secs(30),
            )
            .await
        {
            log::warn!("Failed to cache SOL balance for {}: {}", address, e);
        }

        Ok(balance_data.native_balance)
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
            "zero_sol_addresses".to_string(),
            "low_sol_addresses".to_string(),
            "owner_has_zero_sol".to_string(),
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

        // Extract newly created token account owners
        let newly_created_owners = Self::extract_newly_created_account_owners(tx);

        // Check SOL balances for all addresses
        let mut zero_sol_addresses = Vec::new();
        let mut low_sol_addresses = Vec::new();
        let mut owner_has_zero_sol = false;

        for address in &addresses {
            match self.get_sol_balance(address).await {
                Ok(balance) => {
                    if balance == 0 {
                        zero_sol_addresses.push(address.clone());
                        // Check if this zero-balance address is a newly created account owner
                        if newly_created_owners.contains(address) {
                            owner_has_zero_sol = true;
                            log::info!(
                                "🚨 Detected token transfer to unfunded wallet owner: {}",
                                address
                            );
                        }
                    } else if balance < 10_000_000 {
                        // < 0.01 SOL
                        low_sol_addresses.push(address.clone());
                    }
                }
                Err(e) => {
                    log::debug!("Failed to fetch SOL balance for {}: {}", address, e);
                    // Continue on error - graceful degradation
                }
            }
        }

        let mut fields = HashMap::new();
        fields.insert(
            "signer_classifications".to_string(),
            json!(signer_classifications),
        );
        fields.insert(
            "other_classifications".to_string(),
            json!(other_classifications),
        );
        fields.insert("zero_sol_addresses".to_string(), json!(zero_sol_addresses));
        fields.insert("low_sol_addresses".to_string(), json!(low_sol_addresses));
        fields.insert("owner_has_zero_sol".to_string(), json!(owner_has_zero_sol));

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
