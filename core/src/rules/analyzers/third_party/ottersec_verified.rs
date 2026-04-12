use crate::rules::analyzer::TransactionAnalyzer;
use crate::rules::analyzers::third_party::rate_limiter::ApiRateLimiter;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_sdk::transaction::Transaction;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Response from OtterSec Verification API
#[derive(Debug, Clone, Deserialize, Serialize)]
struct OtterSecVerificationResponse {
    is_verified: bool,
    message: String,
    #[serde(default)]
    on_chain_hash: Option<String>,
    #[serde(default)]
    executable_hash: Option<String>,
    #[serde(default)]
    last_verified_at: Option<String>,
    #[serde(default)]
    repo_url: Option<String>,
}

/// Verification status for a program
#[derive(Debug, Clone)]
struct VerificationStatus {
    is_verified: bool,
    repo_url: Option<String>,
    last_verified_at: Option<String>,
}

/// OtterSec Verified Analyzer - cryptographic verification of program source code
///
/// Uses OtterSec's verifiable build API to confirm that on-chain programs
/// match their published source code via reproducible builds.
///
/// API: https://verify.osec.io
/// GitHub: https://github.com/otter-sec/solana-verified-programs-api
#[derive(Clone)]
pub struct OtterSecVerifiedAnalyzer {
    http_client: reqwest::Client,
    cache: Arc<tokio::sync::Mutex<HashMap<String, VerificationStatus>>>,
    api_base_url: String,
    rate_limiter: ApiRateLimiter,
}

impl OtterSecVerifiedAnalyzer {
    pub fn new() -> Self {
        let api_base_url = std::env::var("OTTERSEC_API_URL")
            .unwrap_or_else(|_| "https://verify.osec.io".to_string());

        log::info!(
            "✅ OtterSecVerifiedAnalyzer: initialized (API: {})",
            api_base_url
        );

        // Configure rate limiter from env or use conservative defaults
        // OtterSec public API: be very conservative to avoid 429s
        let rate_limiter = ApiRateLimiter::from_env_or_default(
            "OTTERSEC_RATE_LIMIT",
            30, // Very conservative: 30 requests per minute (~1 per 2 seconds)
            60, // 60 second window
        );

        Self {
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .user_agent("Parapet-RPC-Proxy/0.1")
                .build()
                .expect("Failed to create HTTP client"),
            cache: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            api_base_url,
            rate_limiter,
        }
    }

    /// Check if a program is verified by OtterSec (with rate limiting)
    async fn check_program_verification(&self, program_id: &str) -> VerificationStatus {
        // Check cache first
        {
            let cache = self.cache.lock().await;
            if let Some(status) = cache.get(program_id) {
                return status.clone();
            }
        }

        // Acquire rate limit permit
        let _permit = self.rate_limiter.acquire().await;

        // Query OtterSec API
        let url = format!("{}/status/{}", self.api_base_url, program_id);

        let status = match self.http_client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<OtterSecVerificationResponse>().await {
                        Ok(verification) => {
                            log::debug!(
                                "OtterSec verification for {}: {} ({})",
                                program_id,
                                verification.is_verified,
                                verification.message
                            );

                            VerificationStatus {
                                is_verified: verification.is_verified,
                                repo_url: verification.repo_url,
                                last_verified_at: verification.last_verified_at,
                            }
                        }
                        Err(e) => {
                            log::debug!(
                                "Failed to parse OtterSec response for {}: {}",
                                program_id,
                                e
                            );
                            VerificationStatus {
                                is_verified: false,
                                repo_url: None,
                                last_verified_at: None,
                            }
                        }
                    }
                } else if response.status().as_u16() == 404 {
                    // Program not in database - not verified
                    log::debug!("Program {} not found in OtterSec database", program_id);
                    VerificationStatus {
                        is_verified: false,
                        repo_url: None,
                        last_verified_at: None,
                    }
                } else {
                    log::warn!(
                        "OtterSec API error for {}: {}",
                        program_id,
                        response.status()
                    );
                    VerificationStatus {
                        is_verified: false,
                        repo_url: None,
                        last_verified_at: None,
                    }
                }
            }
            Err(e) => {
                log::debug!("Failed to query OtterSec API for {}: {}", program_id, e);
                VerificationStatus {
                    is_verified: false,
                    repo_url: None,
                    last_verified_at: None,
                }
            }
        };

        // Cache result
        {
            let mut cache = self.cache.lock().await;
            cache.insert(program_id.to_string(), status.clone());
        }

        status
    }
}

#[async_trait::async_trait]
impl TransactionAnalyzer for OtterSecVerifiedAnalyzer {
    fn name(&self) -> &str {
        "ottersec"
    }

    fn fields(&self) -> Vec<String> {
        vec![
            "programs_verified".to_string(),
            "programs_unverified".to_string(),
            "all_programs_verified".to_string(),
            "verified_count".to_string(),
            "unverified_count".to_string(),
            "program_ids".to_string(),
            "repo_urls".to_string(),
        ]
    }

    async fn analyze(&self, tx: &Transaction) -> Result<HashMap<String, Value>> {
        // Extract unique program IDs from transaction
        let program_ids: HashSet<String> = tx
            .message
            .instructions
            .iter()
            .filter_map(|inst| {
                tx.message
                    .account_keys
                    .get(inst.program_id_index as usize)
                    .map(|pk| pk.to_string())
            })
            .collect();

        if program_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let mut programs_verified = Vec::new();
        let mut programs_unverified = Vec::new();
        let mut repo_urls = Vec::new();

        // Check each program in parallel
        let tasks: Vec<_> = program_ids
            .iter()
            .map(|program_id| {
                let program_id = program_id.clone();
                let analyzer = self.clone();
                async move {
                    let status = analyzer.check_program_verification(&program_id).await;
                    (program_id, status)
                }
            })
            .collect();

        let results = futures::future::join_all(tasks).await;

        for (program_id, status) in results {
            if status.is_verified {
                programs_verified.push(program_id.clone());
                if let Some(ref repo_url) = status.repo_url {
                    repo_urls.push(json!({
                        "program_id": program_id,
                        "repo_url": repo_url,
                        "last_verified": status.last_verified_at,
                    }));
                }
            } else {
                programs_unverified.push(program_id.clone());
            }
        }

        let all_verified = programs_unverified.is_empty() && !program_ids.is_empty();

        let mut fields = HashMap::new();
        fields.insert("programs_verified".to_string(), json!(programs_verified));
        fields.insert(
            "programs_unverified".to_string(),
            json!(programs_unverified),
        );
        fields.insert("all_programs_verified".to_string(), json!(all_verified));
        fields.insert("verified_count".to_string(), json!(programs_verified.len()));
        fields.insert(
            "unverified_count".to_string(),
            json!(programs_unverified.len()),
        );
        fields.insert(
            "program_ids".to_string(),
            json!(program_ids.into_iter().collect::<Vec<_>>()),
        );
        fields.insert("repo_urls".to_string(), json!(repo_urls));

        Ok(fields)
    }

    fn estimated_latency_ms(&self) -> u64 {
        150 // HTTP request latency
    }

    fn recommended_delay_ms(&self) -> Option<u64> {
        // 30 requests per 60 seconds = 2000ms between requests
        Some(2000)
    }
}

impl Default for OtterSecVerifiedAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
