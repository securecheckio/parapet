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
    base_url: String,
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
        Self::new_with_config("https://api.helius.xyz".to_string(), api_key)
    }

    pub fn new_with_base_url(base_url: String) -> Self {
        let api_key = std::env::var("HELIUS_API_KEY").expect("HELIUS_API_KEY not set");
        Self::new_with_config(base_url, api_key)
    }

    pub fn new_with_config(base_url: String, api_key: String) -> Self {
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
            base_url,
        }
    }

    /// Get program verification data from Helius
    pub async fn get_program_data(&self, program_address: &str) -> Result<HeliusData> {
        // Acquire rate limit permit
        let _permit = self.rate_limiter.acquire().await;

        let url = format!(
            "{}/v0/addresses/{}?api-key={}",
            self.base_url, program_address, self.api_key
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_program_data_success() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let mock_response = serde_json::json!({
            "is_verified": true,
            "verifier": "OtterSec",
            "label": "DeFi Protocol",
            "risk_score": 15
        });

        let _mock = server
            .mock("GET", "/v0/addresses/test_program123?api-key=test_key")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response.to_string())
            .create_async()
            .await;

        let client = HeliusClient::new_with_config(mock_url, "test_key".to_string());
        let result = client.get_program_data("test_program123").await;

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.program_address, "test_program123");
        assert!(data.is_verified);
        assert_eq!(data.verifier, Some("OtterSec".to_string()));
        assert_eq!(data.label, Some("DeFi Protocol".to_string()));
        assert_eq!(data.risk_score, Some(15));
    }

    #[tokio::test]
    async fn test_get_program_data_not_verified() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let mock_response = serde_json::json!({
            "is_verified": false
        });

        let _mock = server
            .mock("GET", "/v0/addresses/unverified_program?api-key=test_key")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response.to_string())
            .create_async()
            .await;

        let client = HeliusClient::new_with_config(mock_url, "test_key".to_string());
        let result = client.get_program_data("unverified_program").await;

        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(!data.is_verified);
        assert_eq!(data.verifier, None);
        assert_eq!(data.label, None);
    }

    #[tokio::test]
    async fn test_get_program_data_rate_limited() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let _mock = server
            .mock("GET", "/v0/addresses/test_program?api-key=test_key")
            .with_status(429)
            .with_body("Too Many Requests")
            .create_async()
            .await;

        let client = HeliusClient::new_with_config(mock_url, "test_key".to_string());
        let result = client.get_program_data("test_program").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("rate limited"));
    }

    #[tokio::test]
    async fn test_get_program_data_api_error() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let _mock = server
            .mock("GET", "/v0/addresses/error_program?api-key=test_key")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = HeliusClient::new_with_config(mock_url, "test_key".to_string());
        let result = client.get_program_data("error_program").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("API error"));
    }
}
