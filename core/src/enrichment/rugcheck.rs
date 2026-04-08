#[cfg(feature = "reqwest")]
use anyhow::{anyhow, Result};
#[cfg(feature = "reqwest")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "reqwest")]
use std::collections::HashMap;
#[cfg(feature = "reqwest")]
use std::sync::Arc;
#[cfg(feature = "reqwest")]
use std::time::Duration;
#[cfg(feature = "reqwest")]
use tokio::sync::RwLock;

#[cfg(feature = "reqwest")]
use crate::rules::analyzers::third_party::rate_limiter::ApiRateLimiter;

#[cfg(feature = "reqwest")]

/// Rugcheck API client
pub struct RugcheckClient {
    http_client: reqwest::Client,
    cache: Arc<RwLock<HashMap<String, CachedData>>>,
    cache_ttl: Duration,
    rate_limiter: ApiRateLimiter,
    api_key: Option<String>,
    is_authenticated: bool,
}

struct CachedData {
    data: RugcheckData,
    cached_at: std::time::Instant,
}

/// Rugcheck token data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugcheckData {
    pub token_address: String,
    pub score: u32,
    pub risk_level: String,
    pub risks: Vec<RiskItem>,
    pub market_cap: Option<f64>,
    pub top_holders_percentage: Option<f64>,
    pub liquidity: Option<f64>,
    pub token_age_days: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskItem {
    pub name: String,
    pub description: String,
    pub level: String,
    pub score: u32,
}

/// Insider trading analysis data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsiderAnalysis {
    pub token_address: String,
    pub trade_networks: u32,
    pub transfer_networks: u32,
    pub total_networks: u32,
    pub total_insiders: u32,
    pub insider_concentration: f64, // Percentage held by insiders
    pub risk_level: String,         // Low, Medium, High, Critical
    pub risk_score: u32,            // 0-100 (higher = more risky)
    pub warnings: Vec<String>,
}

/// Liquidity vault/locker analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultAnalysis {
    pub token_address: String,
    pub has_locked_liquidity: bool,
    pub total_lockers: u32,
    pub locked_percentage: f64,
    pub unlock_date: Option<String>,
    pub rugpull_risk: String, // Low, Medium, High, Critical
    pub lockers: Vec<VaultLocker>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultLocker {
    pub locker_type: String, // flux, streamflow, gemfarm, etc.
    pub locked_amount: f64,
    pub unlock_date: Option<String>,
    pub percentage_of_supply: f64,
}

/// Domain registration data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainRegistration {
    pub domain: String,
    pub token_address: String,
    pub verified: bool,
    pub registered_at: Option<String>,
}

impl RugcheckClient {
    pub fn new() -> Self {
        // Check for API key in environment
        let api_key = std::env::var("RUGCHECK_API_KEY").ok();
        let is_authenticated = api_key.is_some();

        // Different rate limits for free vs authenticated API
        let rate_limiter = if is_authenticated {
            // Authenticated API: Higher limits
            // Check documentation or x-rate-limit-limit header for actual limits
            // Using conservative default of 100/min until we know the real limit
            log::info!("✅ RugcheckClient: initialized with API KEY (authenticated)");
            ApiRateLimiter::from_env_or_default(
                "RUGCHECK_RATE_LIMIT",
                100, // Authenticated: much higher limit (adjust based on your plan)
                60,  // 60 second window
            )
        } else {
            // Free API: returns x-rate-limit-limit: 15
            // We use 10/60s to be very conservative and avoid 429s
            log::info!("✅ RugcheckClient: initialized (FREE API, no key required)");
            ApiRateLimiter::from_env_or_default(
                "RUGCHECK_RATE_LIMIT",
                10, // Very conservative: 10 requests per minute (API limit is 15)
                60, // 60 second window
            )
        };

        Self {
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent("Parapet/0.1")
                .build()
                .expect("Failed to create HTTP client"),
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(600), // 10 minute cache
            rate_limiter,
            api_key,
            is_authenticated,
        }
    }

    /// Get token risk data from Rugcheck
    pub async fn get_token_data(&self, token_address: &str) -> Result<RugcheckData> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(token_address) {
                if cached.cached_at.elapsed() < self.cache_ttl {
                    log::debug!("✓ Rugcheck cache hit for {}", token_address);
                    return Ok(cached.data.clone());
                }
            }
        }

        // Acquire rate limit permit
        let _permit = self.rate_limiter.acquire().await;

        // Fetch from API
        let url = format!(
            "https://api.rugcheck.xyz/v1/tokens/{}/report",
            token_address
        );

        log::debug!("Fetching Rugcheck data for token: {}", token_address);

        let max_retries = 3;
        for attempt in 1..=max_retries {
            let mut request = self.http_client.get(&url);

            // Add API key header if authenticated
            if let Some(ref key) = self.api_key {
                request = request.header("X-API-KEY", key);
            }

            let response = request.send().await?;

            // Log rate limit headers for monitoring
            if let Some(limit) = response.headers().get("x-rate-limit-limit") {
                if let Some(remaining) = response.headers().get("x-rate-limit-remaining") {
                    log::debug!(
                        "Rugcheck rate limit: {}/{} remaining",
                        remaining.to_str().unwrap_or("?"),
                        limit.to_str().unwrap_or("?")
                    );

                    // Warn if getting close to limit
                    if let Ok(remaining_str) = remaining.to_str() {
                        if let Ok(remaining_num) = remaining_str.parse::<u32>() {
                            if remaining_num <= 3 {
                                log::warn!(
                                    "⚠️  Rugcheck rate limit low: {} requests remaining",
                                    remaining_num
                                );
                            }
                        }
                    }
                }
            }

            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                if attempt >= max_retries {
                    return Err(anyhow!(
                        "Rugcheck API rate limited after {} retries",
                        max_retries
                    ));
                }
                log::warn!(
                    "🚨 Rugcheck returned 429 (rate limit exceeded) - backing off (attempt {}/{})",
                    attempt,
                    max_retries
                );
                ApiRateLimiter::backoff_on_429(attempt).await;
                continue;
            }

            if response.status() == reqwest::StatusCode::NOT_FOUND {
                return Err(anyhow!("Token not found in Rugcheck database"));
            }

            if !response.status().is_success() {
                return Err(anyhow!("Rugcheck API error: {}", response.status()));
            }

            let report: RugcheckApiResponse = response.json().await?;
            let data = Self::parse_response(token_address, report)?;

            // Cache result
            {
                let mut cache = self.cache.write().await;
                cache.insert(
                    token_address.to_string(),
                    CachedData {
                        data: data.clone(),
                        cached_at: std::time::Instant::now(),
                    },
                );
            }

            return Ok(data);
        }

        Err(anyhow!("Failed to fetch Rugcheck data after retries"))
    }

    fn parse_response(token_address: &str, response: RugcheckApiResponse) -> Result<RugcheckData> {
        let risks: Vec<RiskItem> = response
            .risks
            .unwrap_or_default()
            .into_iter()
            .map(|r| RiskItem {
                name: r.name,
                description: r.description,
                level: r.level,
                score: r.score,
            })
            .collect();

        let (danger, warning, _info, _good) = Self::count_risks_by_level(&risks);

        // Calculate score (0-100, higher is better)
        let score = if danger > 0 {
            // Critical risks = very low score
            (20_u32).saturating_sub(danger * 5)
        } else if warning > 0 {
            // Warnings = medium score
            (70_u32).saturating_sub(warning * 10)
        } else {
            // No major risks = good score
            85
        };

        let risk_level = if score < 30 {
            "Poor".to_string()
        } else if score < 50 {
            "Fair".to_string()
        } else if score < 70 {
            "Good".to_string()
        } else {
            "Excellent".to_string()
        };

        Ok(RugcheckData {
            token_address: token_address.to_string(),
            score,
            risk_level,
            risks,
            market_cap: response.market_cap,
            top_holders_percentage: response.top_holders_percentage,
            liquidity: response.liquidity,
            token_age_days: response.token_age_days,
        })
    }

    fn count_risks_by_level(risks: &[RiskItem]) -> (u32, u32, u32, u32) {
        let mut danger = 0;
        let mut warning = 0;
        let mut info = 0;
        let mut good = 0;

        for risk in risks {
            match risk.level.to_lowercase().as_str() {
                "danger" | "critical" => danger += 1,
                "warning" => warning += 1,
                "info" => info += 1,
                "good" => good += 1,
                _ => {}
            }
        }

        (danger, warning, info, good)
    }

    /// Get insider trading analysis (wash trading, holder inflation, etc.)
    pub async fn get_insider_analysis(&self, token_address: &str) -> Result<InsiderAnalysis> {
        // Acquire rate limit permit
        let _permit = self.rate_limiter.acquire().await;

        let url = format!(
            "https://api.rugcheck.xyz/v1/tokens/{}/insiders/networks",
            token_address
        );

        log::debug!("Fetching insider analysis for token: {}", token_address);

        let mut request = self.http_client.get(&url);
        if let Some(ref key) = self.api_key {
            request = request.header("X-API-KEY", key);
        }

        let response = request.send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            // No insider data available
            return Ok(InsiderAnalysis {
                token_address: token_address.to_string(),
                trade_networks: 0,
                transfer_networks: 0,
                total_networks: 0,
                total_insiders: 0,
                insider_concentration: 0.0,
                risk_level: "Unknown".to_string(),
                risk_score: 0,
                warnings: vec![],
            });
        }

        if !response.status().is_success() {
            return Err(anyhow!(
                "Rugcheck insiders API error: {}",
                response.status()
            ));
        }

        let networks: InsiderNetworksResponse = response.json().await?;
        Self::parse_insider_analysis(token_address, networks)
    }

    fn parse_insider_analysis(
        token_address: &str,
        networks: InsiderNetworksResponse,
    ) -> Result<InsiderAnalysis> {
        let trade_networks = networks.trade_networks.unwrap_or(0);
        let transfer_networks = networks.transfer_networks.unwrap_or(0);
        let total_networks = trade_networks + transfer_networks;
        let total_insiders = networks.total_insiders.unwrap_or(0);
        let insider_concentration = networks.insider_concentration.unwrap_or(0.0);

        let mut warnings = Vec::new();
        let mut risk_score = 0;

        // Analyze trade networks (wash trading)
        if trade_networks >= 3 {
            warnings.push("Multiple trade networks detected - possible wash trading".to_string());
            risk_score += 40;
        } else if trade_networks >= 1 {
            warnings.push("Trade network detected - monitor for coordinated activity".to_string());
            risk_score += 20;
        }

        // Analyze transfer networks (holder inflation)
        if transfer_networks >= 2 {
            warnings.push("Multiple transfer networks - possible holder inflation".to_string());
            risk_score += 30;
        } else if transfer_networks >= 1 {
            warnings.push("Transfer network detected - artificial holder distribution".to_string());
            risk_score += 15;
        }

        // Analyze insider concentration
        if insider_concentration > 70.0 {
            warnings.push(format!(
                "Very high insider concentration ({:.1}%) - coordinated dump risk",
                insider_concentration
            ));
            risk_score += 40;
        } else if insider_concentration > 50.0 {
            warnings.push(format!(
                "High insider concentration ({:.1}%) - monitor closely",
                insider_concentration
            ));
            risk_score += 25;
        } else if insider_concentration > 30.0 {
            warnings.push(format!(
                "Moderate insider concentration ({:.1}%)",
                insider_concentration
            ));
            risk_score += 10;
        }

        // Analyze total insiders
        if total_insiders > 50 {
            warnings.push(format!(
                "Large insider network ({} wallets) - complex coordination possible",
                total_insiders
            ));
            risk_score += 15;
        }

        let risk_level = if risk_score >= 75 {
            "Critical".to_string()
        } else if risk_score >= 50 {
            "High".to_string()
        } else if risk_score >= 25 {
            "Medium".to_string()
        } else if risk_score > 0 {
            "Low".to_string()
        } else {
            "None".to_string()
        };

        Ok(InsiderAnalysis {
            token_address: token_address.to_string(),
            trade_networks,
            transfer_networks,
            total_networks,
            total_insiders,
            insider_concentration,
            risk_level,
            risk_score,
            warnings,
        })
    }

    /// Get liquidity vault/locker analysis
    pub async fn get_vault_analysis(&self, token_address: &str) -> Result<VaultAnalysis> {
        // Acquire rate limit permit
        let _permit = self.rate_limiter.acquire().await;

        let url = format!(
            "https://api.rugcheck.xyz/v1/tokens/{}/lockers",
            token_address
        );

        log::debug!("Fetching vault analysis for token: {}", token_address);

        let mut request = self.http_client.get(&url);
        if let Some(ref key) = self.api_key {
            request = request.header("X-API-KEY", key);
        }

        let response = request.send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            // No vault data available
            return Ok(VaultAnalysis {
                token_address: token_address.to_string(),
                has_locked_liquidity: false,
                total_lockers: 0,
                locked_percentage: 0.0,
                unlock_date: None,
                rugpull_risk: "High".to_string(),
                lockers: vec![],
            });
        }

        if !response.status().is_success() {
            return Err(anyhow!("Rugcheck vault API error: {}", response.status()));
        }

        let vault_response: VaultResponse = response.json().await?;
        Self::parse_vault_analysis(token_address, vault_response)
    }

    fn parse_vault_analysis(
        token_address: &str,
        vault_response: VaultResponse,
    ) -> Result<VaultAnalysis> {
        let lockers: Vec<VaultLocker> = vault_response
            .lockers
            .unwrap_or_default()
            .into_iter()
            .map(|l| VaultLocker {
                locker_type: l.locker_type.unwrap_or_else(|| "unknown".to_string()),
                locked_amount: l.locked_amount.unwrap_or(0.0),
                unlock_date: l.unlock_date,
                percentage_of_supply: l.percentage_of_supply.unwrap_or(0.0),
            })
            .collect();

        let has_locked_liquidity = !lockers.is_empty();
        let total_lockers = lockers.len() as u32;
        let locked_percentage: f64 = lockers.iter().map(|l| l.percentage_of_supply).sum();

        // Find earliest unlock date
        let unlock_date = lockers
            .iter()
            .filter_map(|l| l.unlock_date.as_ref())
            .min()
            .cloned();

        // Calculate rugpull risk based on locked percentage
        let rugpull_risk = if locked_percentage >= 80.0 {
            "Low".to_string()
        } else if locked_percentage >= 50.0 {
            "Medium".to_string()
        } else if locked_percentage >= 20.0 {
            "High".to_string()
        } else {
            "Critical".to_string()
        };

        Ok(VaultAnalysis {
            token_address: token_address.to_string(),
            has_locked_liquidity,
            total_lockers,
            locked_percentage,
            unlock_date,
            rugpull_risk,
            lockers,
        })
    }

    /// Lookup domain registration for a token
    pub async fn lookup_domain(&self, token_address: &str) -> Result<Option<DomainRegistration>> {
        // Acquire rate limit permit
        let _permit = self.rate_limiter.acquire().await;

        let url = format!(
            "https://api.rugcheck.xyz/v1/domains/lookup/{}",
            token_address
        );

        log::debug!("Looking up domain for token: {}", token_address);

        let mut request = self.http_client.get(&url);
        if let Some(ref key) = self.api_key {
            request = request.header("X-API-KEY", key);
        }

        let response = request.send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !response.status().is_success() {
            return Err(anyhow!("Rugcheck domain API error: {}", response.status()));
        }

        let domain_response: DomainResponse = response.json().await?;

        Ok(Some(DomainRegistration {
            domain: domain_response.domain,
            token_address: domain_response.mint,
            verified: domain_response.verified.unwrap_or(false),
            registered_at: domain_response.registered_at,
        }))
    }

    /// Get bulk token summaries (much faster than individual calls)
    pub async fn get_bulk_summaries(
        &self,
        token_addresses: &[String],
    ) -> Result<HashMap<String, RugcheckData>> {
        if token_addresses.is_empty() {
            return Ok(HashMap::new());
        }

        // Check cache first
        let mut results = HashMap::new();
        let mut uncached_tokens = Vec::new();

        {
            let cache = self.cache.read().await;
            for token in token_addresses {
                if let Some(cached) = cache.get(token) {
                    if cached.cached_at.elapsed() < self.cache_ttl {
                        log::debug!("✓ Rugcheck cache hit for {}", token);
                        results.insert(token.clone(), cached.data.clone());
                        continue;
                    }
                }
                uncached_tokens.push(token.clone());
            }
        }

        if uncached_tokens.is_empty() {
            return Ok(results);
        }

        // Split into chunks of 50 (API may have limits)
        for chunk in uncached_tokens.chunks(50) {
            // Acquire rate limit permit
            let _permit = self.rate_limiter.acquire().await;

            let url = "https://api.rugcheck.xyz/v1/bulk/tokens/summary";

            log::debug!("Fetching bulk Rugcheck data for {} tokens", chunk.len());

            let body = serde_json::json!({
                "mints": chunk
            });

            let mut request = self.http_client.post(url).json(&body);
            if let Some(ref key) = self.api_key {
                request = request.header("X-API-KEY", key);
            }

            let response = request.send().await?;

            if !response.status().is_success() {
                log::warn!(
                    "Bulk Rugcheck API error: {} - falling back to individual calls",
                    response.status()
                );
                // Fallback: fetch individually
                for token in chunk {
                    match self.get_token_data(token).await {
                        Ok(data) => {
                            results.insert(token.clone(), data);
                        }
                        Err(e) => {
                            log::warn!("Failed to fetch token {}: {}", token, e);
                        }
                    }
                }
                continue;
            }

            let bulk_response: BulkSummaryResponse = response.json().await?;

            // Parse and cache results
            for (token, summary) in bulk_response.summaries.unwrap_or_default() {
                if let Some(summary) = summary {
                    let data = Self::parse_bulk_summary(&token, summary);

                    // Cache result
                    {
                        let mut cache = self.cache.write().await;
                        cache.insert(
                            token.clone(),
                            CachedData {
                                data: data.clone(),
                                cached_at: std::time::Instant::now(),
                            },
                        );
                    }

                    results.insert(token, data);
                }
            }
        }

        Ok(results)
    }

    fn parse_bulk_summary(token_address: &str, summary: BulkTokenSummary) -> RugcheckData {
        let risks: Vec<RiskItem> = summary
            .risks
            .unwrap_or_default()
            .into_iter()
            .map(|r| RiskItem {
                name: r,
                description: String::new(),
                level: "warning".to_string(),
                score: 10,
            })
            .collect();

        let score = summary.score.unwrap_or(50);

        let risk_level = if score < 30 {
            "Poor".to_string()
        } else if score < 50 {
            "Fair".to_string()
        } else if score < 70 {
            "Good".to_string()
        } else {
            "Excellent".to_string()
        };

        RugcheckData {
            token_address: token_address.to_string(),
            score,
            risk_level,
            risks,
            market_cap: None,
            top_holders_percentage: None,
            liquidity: None,
            token_age_days: None,
        }
    }
}

// Internal API response structures
#[derive(Debug, Deserialize)]
struct RugcheckApiResponse {
    risks: Option<Vec<RugcheckRisk>>,
    market_cap: Option<f64>,
    top_holders_percentage: Option<f64>,
    liquidity: Option<f64>,
    token_age_days: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct RugcheckRisk {
    name: String,
    description: String,
    level: String,
    score: u32,
}

#[derive(Debug, Deserialize)]
struct InsiderNetworksResponse {
    trade_networks: Option<u32>,
    transfer_networks: Option<u32>,
    total_insiders: Option<u32>,
    insider_concentration: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct VaultResponse {
    lockers: Option<Vec<VaultLockerResponse>>,
}

#[derive(Debug, Deserialize)]
struct VaultLockerResponse {
    locker_type: Option<String>,
    locked_amount: Option<f64>,
    unlock_date: Option<String>,
    percentage_of_supply: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct DomainResponse {
    domain: String,
    mint: String,
    verified: Option<bool>,
    registered_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BulkSummaryResponse {
    summaries: Option<HashMap<String, Option<BulkTokenSummary>>>,
}

#[derive(Debug, Deserialize)]
struct BulkTokenSummary {
    score: Option<u32>,
    risks: Option<Vec<String>>,
}

impl Default for RugcheckClient {
    fn default() -> Self {
        Self::new()
    }
}

impl RugcheckClient {
    /// Check if client is using authenticated API
    pub fn is_authenticated(&self) -> bool {
        self.is_authenticated
    }

    /// Get the current rate limit (requests per minute)
    pub fn rate_limit_per_minute(&self) -> u32 {
        if self.is_authenticated {
            100 // Authenticated default
        } else {
            10 // Free tier default
        }
    }
}
