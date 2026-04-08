#[cfg(feature = "reqwest")]
use anyhow::{anyhow, Result};
#[cfg(feature = "reqwest")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "reqwest")]
use std::time::Duration;

#[cfg(feature = "reqwest")]
use crate::rules::analyzers::third_party::rate_limiter::ApiRateLimiter;

#[cfg(feature = "reqwest")]

/// Jupiter API client
pub struct JupiterClient {
    api_key: Option<String>,
    http_client: reqwest::Client,
    rate_limiter: ApiRateLimiter,
}

/// Jupiter token data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterData {
    pub token_address: String,
    pub price_usd: Option<f64>,
    pub volume_24h: Option<f64>,
    pub liquidity: Option<f64>,
    pub organic_score: Option<u32>,
    pub has_rugpull_indicators: bool,
}

impl JupiterClient {
    pub fn new() -> Self {
        let api_key = std::env::var("JUPITER_API_KEY").ok();

        if api_key.is_some() {
            log::info!("✅ JupiterClient: initialized with API key");
        } else {
            log::info!("✅ JupiterClient: initialized (using public API)");
        }

        // Jupiter free tier: 60 req/min
        let rate_limiter = ApiRateLimiter::from_env_or_default(
            "JUPITER_RATE_LIMIT",
            60,
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

    /// Get token data from Jupiter
    pub async fn get_token_data(&self, token_address: &str) -> Result<JupiterData> {
        // Acquire rate limit permit
        let _permit = self.rate_limiter.acquire().await;

        let url = if let Some(key) = &self.api_key {
            format!("https://api.jup.ag/v1/tokens/{}?api-key={}", token_address, key)
        } else {
            format!("https://api.jup.ag/v1/tokens/{}", token_address)
        };

        log::debug!("Fetching Jupiter data for token: {}", token_address);

        let response = self.http_client.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(anyhow!("Jupiter API rate limited"));
        }

        if !response.status().is_success() {
            return Err(anyhow!("Jupiter API error: {}", response.status()));
        }

        let api_response: JupiterApiResponse = response.json().await?;

        Ok(JupiterData {
            token_address: token_address.to_string(),
            price_usd: api_response.price_usd,
            volume_24h: api_response.volume_24h,
            liquidity: api_response.liquidity,
            organic_score: api_response.organic_score,
            has_rugpull_indicators: api_response.rugpull_risk.unwrap_or(false),
        })
    }
}

#[derive(Debug, Deserialize)]
struct JupiterApiResponse {
    price_usd: Option<f64>,
    volume_24h: Option<f64>,
    liquidity: Option<f64>,
    organic_score: Option<u32>,
    rugpull_risk: Option<bool>,
}

impl Default for JupiterClient {
    fn default() -> Self {
        Self::new()
    }
}
