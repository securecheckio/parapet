use crate::rules::analyzer::TransactionAnalyzer;
use crate::rules::analyzers::third_party::rate_limiter::ApiRateLimiter;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_sdk::transaction::Transaction;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Rugcheck API token report response
#[derive(Debug, Clone, Deserialize, Serialize)]
struct RugcheckReport {
    mint: String,
    #[serde(rename = "tokenProgram")]
    token_program: Option<String>,
    #[serde(rename = "tokenType")]
    token_type: Option<String>,
    
    // Risk assessment
    score: Option<u32>,
    #[serde(rename = "rugged")]
    is_rugged: Option<bool>,
    risks: Option<Vec<Risk>>,
    
    // Market data
    markets: Option<Vec<Market>>,
    
    // Token metadata
    #[serde(rename = "tokenMeta")]
    token_meta: Option<TokenMeta>,
    
    // Holders and supply
    #[serde(rename = "topHolders")]
    top_holders: Option<Vec<Holder>>,
    #[serde(rename = "totalSupply")]
    total_supply: Option<String>,
    
    // LP (Liquidity Pool) analysis
    lp: Option<LiquidityPool>,
    
    // Creator info
    creator: Option<Creator>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Risk {
    name: String,
    value: String,
    description: Option<String>,
    level: String, // "danger", "warning", "info", "good"
    score: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Market {
    lp: String,
    #[serde(rename = "liquidityA")]
    liquidity_a: Option<f64>,
    #[serde(rename = "liquidityB")]
    liquidity_b: Option<f64>,
    #[serde(rename = "liquidityUSD")]
    liquidity_usd: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TokenMeta {
    name: Option<String>,
    symbol: Option<String>,
    uri: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Holder {
    address: String,
    pct: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct LiquidityPool {
    #[serde(rename = "lpBurn")]
    lp_burn: Option<f64>,
    #[serde(rename = "lpLocked")]
    lp_locked: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Creator {
    address: Option<String>,
    pct: Option<f64>,
}

/// Cache entry with TTL
struct CacheEntry {
    report: RugcheckReport,
    cached_at: SystemTime,
}

/// Rugcheck Analyzer - scam/rugpull detection via Rugcheck.xyz API
/// 
/// Rugcheck provides comprehensive token security analysis including:
/// - Rugpull risk scoring (0-100)
/// - Liquidity lock status
/// - Mint/freeze authority checks
/// - Top holder concentration
/// - Creator behavior analysis
/// - Market manipulation detection
///
/// API: https://api.rugcheck.xyz
/// Docs: https://api.rugcheck.xyz/swagger/index.html
/// FREE - No API key required
pub struct RugcheckAnalyzer {
    http_client: reqwest::Client,
    cache: Arc<tokio::sync::RwLock<HashMap<String, CacheEntry>>>,
    cache_ttl: Duration,
    rate_limiter: ApiRateLimiter,
}

impl RugcheckAnalyzer {
    pub fn new() -> Self {
        log::info!("✅ RugcheckAnalyzer: initialized (FREE API, no key required)");

        // CRITICAL: Rugcheck API returns x-rate-limit-limit: 15
        // We use 10/60s to be very conservative and avoid 429s
        let rate_limiter = ApiRateLimiter::from_env_or_default(
            "RUGCHECK_RATE_LIMIT",
            10,  // Very conservative: 10 requests per minute (API limit is 15)
            60,  // 60 second window
        );

        Self {
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent("Sol-Shield/0.1")
                .build()
                .expect("Failed to create HTTP client"),
            cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(600), // 10 minute cache (token risk changes slowly)
            rate_limiter,
        }
    }

    /// Fetch token report from Rugcheck API
    async fn fetch_token_report(&self, mint_address: &str) -> Result<RugcheckReport> {
        let now = SystemTime::now();

        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(mint_address) {
                if now.duration_since(entry.cached_at).unwrap_or(Duration::MAX) < self.cache_ttl {
                    log::debug!("Rugcheck: Cache hit for {}", mint_address);
                    return Ok(entry.report.clone());
                }
            }
        }

        let url = format!(
            "https://api.rugcheck.xyz/v1/tokens/{}/report",
            mint_address
        );

        log::debug!("Rugcheck: Fetching report for {}", mint_address);

        // Retry logic with exponential backoff on 429
        let mut attempt = 0;
        let max_retries = 3;

        loop {
            // Acquire rate limit permit
            let _permit = self.rate_limiter.acquire().await;

            let response = self.http_client.get(&url).send().await?;

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
                attempt += 1;
                if attempt >= max_retries {
                    return Err(anyhow!("Rugcheck API rate limited after {} retries", max_retries));
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

            let report: RugcheckReport = response.json().await?;

            // Store in cache
            {
                let mut cache = self.cache.write().await;
                cache.insert(
                    mint_address.to_string(),
                    CacheEntry {
                        report: report.clone(),
                        cached_at: now,
                    },
                );
            }

            return Ok(report);
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
                    // Token instructions typically have mint at accounts[0] or [1]
                    for &mint_idx in instruction.accounts.iter().take(3) {
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

    /// Count risks by level
    fn count_risks_by_level(&self, risks: &[Risk]) -> (u32, u32, u32, u32) {
        let mut danger = 0;
        let mut warning = 0;
        let mut info = 0;
        let mut good = 0;

        for risk in risks {
            match risk.level.as_str() {
                "danger" => danger += 1,
                "warning" => warning += 1,
                "info" => info += 1,
                "good" => good += 1,
                _ => {}
            }
        }

        (danger, warning, info, good)
    }
}

#[async_trait::async_trait]
impl TransactionAnalyzer for RugcheckAnalyzer {
    fn name(&self) -> &str {
        "rugcheck"
    }

    fn fields(&self) -> Vec<String> {
        vec![
            // Core risk assessment
            "rugcheck_score".to_string(),
            "is_rugged".to_string(),
            "risk_level".to_string(), // critical, high, medium, low
            
            // Risk counts
            "danger_count".to_string(),
            "warning_count".to_string(),
            
            // Specific risk indicators
            "has_freeze_authority".to_string(),
            "has_mint_authority".to_string(),
            "low_liquidity".to_string(),
            "high_creator_percentage".to_string(),
            "high_top_holder_concentration".to_string(),
            
            // Liquidity analysis
            "lp_locked_percentage".to_string(),
            "lp_burned_percentage".to_string(),
            "total_liquidity_usd".to_string(),
            
            // Holder concentration
            "top_holders_percentage".to_string(),
            "creator_percentage".to_string(),
            
            // Combined risk flags
            "is_likely_scam".to_string(),
            "is_high_risk".to_string(),
            "requires_caution".to_string(),
            
            // Risk details (for debugging/logging)
            "risk_details".to_string(),
        ]
    }

    async fn analyze(&self, tx: &Transaction) -> Result<HashMap<String, Value>> {
        let mut fields = HashMap::new();

        // Extract token mints
        let mints = self.extract_token_mints(tx);

        if mints.is_empty() {
            return Ok(fields);
        }

        // Analyze first token only (most transactions involve 1 token)
        let mint = &mints[0];

        let report = match self.fetch_token_report(mint).await {
            Ok(r) => r,
            Err(e) => {
                log::debug!("Rugcheck API call failed for {}: {}", mint, e);
                return Ok(fields); // Graceful degradation
            }
        };

        // Core risk assessment
        let score = report.score.unwrap_or(0);
        fields.insert("rugcheck_score".to_string(), json!(score));
        fields.insert("is_rugged".to_string(), json!(report.is_rugged));

        // Determine risk level based on score
        let risk_level = match score {
            0..=25 => "critical",
            26..=50 => "high",
            51..=75 => "medium",
            _ => "low",
        };
        fields.insert("risk_level".to_string(), json!(risk_level));

        // Analyze risks
        if let Some(ref risks) = report.risks {
            let (danger, warning, _info, _good) = self.count_risks_by_level(risks);
            
            fields.insert("danger_count".to_string(), json!(danger));
            fields.insert("warning_count".to_string(), json!(warning));

            // Extract specific risk indicators
            let mut has_freeze = false;
            let mut has_mint = false;
            let mut low_liquidity = false;

            let risk_details: Vec<String> = risks
                .iter()
                .filter(|r| r.level == "danger" || r.level == "warning")
                .map(|r| format!("{}: {}", r.name, r.value))
                .collect();

            for risk in risks {
                match risk.name.as_str() {
                    "Freeze Authority" | "freeze_authority" => {
                        has_freeze = risk.value.to_lowercase().contains("enabled") 
                            || risk.value.to_lowercase().contains("yes");
                    }
                    "Mint Authority" | "mint_authority" => {
                        has_mint = risk.value.to_lowercase().contains("enabled")
                            || risk.value.to_lowercase().contains("yes");
                    }
                    "Liquidity" => {
                        low_liquidity = risk.level == "danger" || risk.level == "warning";
                    }
                    _ => {}
                }
            }

            fields.insert("has_freeze_authority".to_string(), json!(has_freeze));
            fields.insert("has_mint_authority".to_string(), json!(has_mint));
            fields.insert("low_liquidity".to_string(), json!(low_liquidity));
            fields.insert("risk_details".to_string(), json!(risk_details));
        }

        // Liquidity pool analysis
        if let Some(ref lp) = report.lp {
            fields.insert("lp_locked_percentage".to_string(), json!(lp.lp_locked));
            fields.insert("lp_burned_percentage".to_string(), json!(lp.lp_burn));
        }

        // Calculate total liquidity
        let total_liquidity: f64 = report
            .markets
            .as_ref()
            .map(|markets| {
                markets
                    .iter()
                    .filter_map(|m| m.liquidity_usd)
                    .sum()
            })
            .unwrap_or(0.0);
        fields.insert("total_liquidity_usd".to_string(), json!(total_liquidity));

        // Holder concentration
        let top_holders_pct: f64 = report
            .top_holders
            .as_ref()
            .map(|holders| holders.iter().map(|h| h.pct).sum())
            .unwrap_or(0.0);
        
        fields.insert("top_holders_percentage".to_string(), json!(top_holders_pct));
        fields.insert("high_top_holder_concentration".to_string(), json!(top_holders_pct > 50.0));

        // Creator analysis
        let creator_pct = report.creator.as_ref().and_then(|c| c.pct).unwrap_or(0.0);
        fields.insert("creator_percentage".to_string(), json!(creator_pct));
        fields.insert("high_creator_percentage".to_string(), json!(creator_pct > 20.0));

        // Combined risk flags
        let is_likely_scam = score < 30 || report.is_rugged.unwrap_or(false);
        let is_high_risk = score < 50 || top_holders_pct > 70.0;
        let requires_caution = score < 70 || top_holders_pct > 50.0;

        fields.insert("is_likely_scam".to_string(), json!(is_likely_scam));
        fields.insert("is_high_risk".to_string(), json!(is_high_risk));
        fields.insert("requires_caution".to_string(), json!(requires_caution));

        Ok(fields)
    }

    fn is_available(&self) -> bool {
        true // No API key required!
    }

    fn estimated_latency_ms(&self) -> u64 {
        200 // API call latency
    }

    fn recommended_delay_ms(&self) -> Option<u64> {
        // 10 requests per 60 seconds = 6000ms between requests
        // Conservative to avoid hitting Rugcheck's x-rate-limit-limit: 15
        Some(6000)
    }
}

impl Default for RugcheckAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_fields() {
        let analyzer = RugcheckAnalyzer::new();
        let fields = analyzer.fields();
        
        assert!(fields.contains(&"rugcheck_score".to_string()));
        assert!(fields.contains(&"is_rugged".to_string()));
        assert!(fields.contains(&"is_likely_scam".to_string()));
        assert!(fields.contains(&"lp_locked_percentage".to_string()));
    }

    #[test]
    fn test_risk_counting() {
        let analyzer = RugcheckAnalyzer::new();
        let risks = vec![
            Risk {
                name: "Test1".to_string(),
                value: "bad".to_string(),
                description: None,
                level: "danger".to_string(),
                score: Some(10),
            },
            Risk {
                name: "Test2".to_string(),
                value: "warning".to_string(),
                description: None,
                level: "warning".to_string(),
                score: Some(20),
            },
        ];

        let (danger, warning, info, good) = analyzer.count_risks_by_level(&risks);
        assert_eq!(danger, 1);
        assert_eq!(warning, 1);
        assert_eq!(info, 0);
        assert_eq!(good, 0);
    }
}
