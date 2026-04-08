#[cfg(feature = "reqwest")]
use anyhow::{anyhow, Result};
#[cfg(feature = "reqwest")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "reqwest")]
use std::time::Duration;

#[cfg(feature = "reqwest")]
use crate::rules::analyzers::third_party::rate_limiter::ApiRateLimiter;

#[cfg(feature = "reqwest")]

/// OtterSec API client
pub struct OtterSecClient {
    api_key: String,
    http_client: reqwest::Client,
    rate_limiter: ApiRateLimiter,
}

/// OtterSec verification data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtterSecData {
    pub program_address: String,
    pub is_verified: bool,
    pub verification_level: Option<String>,
    pub audit_date: Option<String>,
    pub source_available: bool,
}

impl OtterSecClient {
    pub fn new() -> Self {
        let api_key = std::env::var("OTTERSEC_API_KEY")
            .expect("OTTERSEC_API_KEY not set");

        log::info!("✅ OtterSecClient: initialized with API key");

        // OtterSec: conservative rate limit
        let rate_limiter = ApiRateLimiter::from_env_or_default(
            "OTTERSEC_RATE_LIMIT",
            30,
            60,
        );

        Self {
            api_key,
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent("Parapet/0.1")
                .build()
                .expect("Failed to create HTTP client"),
            rate_limiter,
        }
    }

    /// Get program verification data from OtterSec
    pub async fn get_verification_data(&self, program_address: &str) -> Result<OtterSecData> {
        // Acquire rate limit permit
        let _permit = self.rate_limiter.acquire().await;

        let url = format!("https://verify.osec.io/status/{}", program_address);

        log::debug!("Fetching OtterSec data for program: {}", program_address);

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(anyhow!("OtterSec API rate limited"));
        }

        if !response.status().is_success() {
            return Err(anyhow!("OtterSec API error: {}", response.status()));
        }

        let api_response: OtterSecApiResponse = response.json().await?;

        Ok(OtterSecData {
            program_address: program_address.to_string(),
            is_verified: api_response.is_verified,
            verification_level: api_response.verification_level,
            audit_date: api_response.audit_date,
            source_available: api_response.source_available.unwrap_or(false),
        })
    }
}

#[derive(Debug, Deserialize)]
struct OtterSecApiResponse {
    is_verified: bool,
    verification_level: Option<String>,
    audit_date: Option<String>,
    source_available: Option<bool>,
}

impl Default for OtterSecClient {
    fn default() -> Self {
        Self::new()
    }
}
