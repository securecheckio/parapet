#[cfg(feature = "reqwest")]
use anyhow::{anyhow, Result};
#[cfg(feature = "reqwest")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "reqwest")]
use std::time::Duration;

#[cfg(feature = "reqwest")]
use crate::rules::analyzers::third_party::rate_limiter::ApiRateLimiter;

#[cfg(feature = "reqwest")]

/// Helius API client
pub struct HeliusClient {
    api_key: String,
    http_client: reqwest::Client,
    rate_limiter: ApiRateLimiter,
}

/// Helius program verification data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeliusData {
    pub program_address: String,
    pub is_verified: bool,
    pub verifier: Option<String>,
    pub label: Option<String>,
    pub risk_score: Option<u32>,
}

impl HeliusClient {
    pub fn new() -> Self {
        let api_key = std::env::var("HELIUS_API_KEY").expect("HELIUS_API_KEY not set");

        log::info!("✅ HeliusClient: initialized with API key");

        // Helius free tier: ~10k/day, be conservative
        let rate_limiter = ApiRateLimiter::from_env_or_default(
            "HELIUS_RATE_LIMIT",
            20, // Conservative: 20 requests per minute
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

    /// Get program verification data from Helius
    pub async fn get_program_data(&self, program_address: &str) -> Result<HeliusData> {
        // Acquire rate limit permit
        let _permit = self.rate_limiter.acquire().await;

        let url = format!(
            "https://api.helius.xyz/v0/addresses/{}?api-key={}",
            program_address, self.api_key
        );

        log::debug!("Fetching Helius data for program: {}", program_address);

        let response = self.http_client.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(anyhow!("Helius API rate limited"));
        }

        if !response.status().is_success() {
            return Err(anyhow!("Helius API error: {}", response.status()));
        }

        let api_response: HeliusApiResponse = response.json().await?;

        Ok(HeliusData {
            program_address: program_address.to_string(),
            is_verified: api_response.is_verified.unwrap_or(false),
            verifier: api_response.verifier,
            label: api_response.label,
            risk_score: api_response.risk_score,
        })
    }
}

#[derive(Debug, Deserialize)]
struct HeliusApiResponse {
    is_verified: Option<bool>,
    verifier: Option<String>,
    label: Option<String>,
    risk_score: Option<u32>,
}

impl Default for HeliusClient {
    fn default() -> Self {
        Self::new()
    }
}
