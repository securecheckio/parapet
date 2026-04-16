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
    base_url: String,
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
        Self::new_with_config("https://api.jup.ag".to_string(), api_key)
    }

    pub fn new_with_base_url(base_url: String) -> Self {
        let api_key = std::env::var("JUPITER_API_KEY").ok();
        Self::new_with_config(base_url, api_key)
    }

    pub fn new_with_config(base_url: String, api_key: Option<String>) -> Self {
        if api_key.is_some() {
            log::info!("✅ JupiterClient: initialized with API key");
        } else {
            log::info!("✅ JupiterClient: initialized (using public API)");
        }

        // Jupiter free tier: 60 req/min
        let rate_limiter = ApiRateLimiter::from_env_or_default("JUPITER_RATE_LIMIT", 60, 60);

        Self {
            api_key,
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent("Parapet/0.1")
                .build()
                .expect("Failed to create HTTP client"),
            rate_limiter,
            base_url,
        }
    }

    /// Get token data from Jupiter
    pub async fn get_token_data(&self, token_address: &str) -> Result<JupiterData> {
        // Acquire rate limit permit
        let _permit = self.rate_limiter.acquire().await;

        let url = if let Some(key) = &self.api_key {
            format!(
                "{}/v1/tokens/{}?api-key={}",
                self.base_url, token_address, key
            )
        } else {
            format!("{}/v1/tokens/{}", self.base_url, token_address)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_token_data_success() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let mock_response = serde_json::json!({
            "price_usd": 1.23,
            "volume_24h": 1000000.0,
            "liquidity": 5000000.0,
            "organic_score": 85,
            "rugpull_risk": false
        });

        let _mock = server
            .mock("GET", "/v1/tokens/test_token123")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response.to_string())
            .create_async()
            .await;

        let client = JupiterClient::new_with_config(mock_url, None);
        let result = client.get_token_data("test_token123").await;

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.token_address, "test_token123");
        assert_eq!(data.price_usd, Some(1.23));
        assert_eq!(data.volume_24h, Some(1000000.0));
        assert_eq!(data.liquidity, Some(5000000.0));
        assert_eq!(data.organic_score, Some(85));
        assert!(!data.has_rugpull_indicators);
    }

    #[tokio::test]
    async fn test_get_token_data_with_rugpull_risk() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let mock_response = serde_json::json!({
            "price_usd": 0.01,
            "volume_24h": 100.0,
            "liquidity": 500.0,
            "organic_score": 15,
            "rugpull_risk": true
        });

        let _mock = server
            .mock("GET", "/v1/tokens/risky_token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response.to_string())
            .create_async()
            .await;

        let client = JupiterClient::new_with_config(mock_url, None);
        let result = client.get_token_data("risky_token").await;

        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(data.has_rugpull_indicators);
        assert_eq!(data.organic_score, Some(15));
    }

    #[tokio::test]
    async fn test_get_token_data_minimal_fields() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let mock_response = serde_json::json!({});

        let _mock = server
            .mock("GET", "/v1/tokens/minimal_token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response.to_string())
            .create_async()
            .await;

        let client = JupiterClient::new_with_config(mock_url, None);
        let result = client.get_token_data("minimal_token").await;

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.price_usd, None);
        assert_eq!(data.volume_24h, None);
        assert_eq!(data.liquidity, None);
        assert_eq!(data.organic_score, None);
        assert!(!data.has_rugpull_indicators);
    }

    #[tokio::test]
    async fn test_get_token_data_rate_limited() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let _mock = server
            .mock("GET", "/v1/tokens/test_token")
            .with_status(429)
            .with_body("Too Many Requests")
            .create_async()
            .await;

        let client = JupiterClient::new_with_config(mock_url, None);
        let result = client.get_token_data("test_token").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("rate limited"));
    }

    #[tokio::test]
    async fn test_get_token_data_api_error() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let _mock = server
            .mock("GET", "/v1/tokens/error_token")
            .with_status(404)
            .with_body("Not Found")
            .create_async()
            .await;

        let client = JupiterClient::new_with_config(mock_url, None);
        let result = client.get_token_data("error_token").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("API error"));
    }
}
