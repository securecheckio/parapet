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
    base_url: String,
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
        let api_key = std::env::var("OTTERSEC_API_KEY").expect("OTTERSEC_API_KEY not set");
        Self::new_with_config("https://verify.osec.io".to_string(), api_key)
    }

    pub fn new_with_base_url(base_url: String) -> Self {
        let api_key = std::env::var("OTTERSEC_API_KEY").expect("OTTERSEC_API_KEY not set");
        Self::new_with_config(base_url, api_key)
    }

    pub fn new_with_config(base_url: String, api_key: String) -> Self {
        log::info!("✅ OtterSecClient: initialized with API key");

        // OtterSec: conservative rate limit
        let rate_limiter = ApiRateLimiter::from_env_or_default("OTTERSEC_RATE_LIMIT", 30, 60);

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

    /// Get program verification data from OtterSec
    pub async fn get_verification_data(&self, program_address: &str) -> Result<OtterSecData> {
        // Acquire rate limit permit
        let _permit = self.rate_limiter.acquire().await;

        let url = format!("{}/status/{}", self.base_url, program_address);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_verification_data_verified() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let mock_response = serde_json::json!({
            "is_verified": true,
            "verification_level": "Full Audit",
            "audit_date": "2024-01-15",
            "source_available": true
        });

        let _mock = server
            .mock("GET", "/status/verified_program")
            .match_header("Authorization", "Bearer test_key")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response.to_string())
            .create_async()
            .await;

        let client = OtterSecClient::new_with_config(mock_url, "test_key".to_string());
        let result = client.get_verification_data("verified_program").await;

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.program_address, "verified_program");
        assert!(data.is_verified);
        assert_eq!(data.verification_level, Some("Full Audit".to_string()));
        assert_eq!(data.audit_date, Some("2024-01-15".to_string()));
        assert!(data.source_available);
    }

    #[tokio::test]
    async fn test_get_verification_data_not_verified() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let mock_response = serde_json::json!({
            "is_verified": false
        });

        let _mock = server
            .mock("GET", "/status/unverified_program")
            .match_header("Authorization", "Bearer test_key")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response.to_string())
            .create_async()
            .await;

        let client = OtterSecClient::new_with_config(mock_url, "test_key".to_string());
        let result = client.get_verification_data("unverified_program").await;

        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(!data.is_verified);
        assert_eq!(data.verification_level, None);
        assert!(!data.source_available);
    }

    #[tokio::test]
    async fn test_get_verification_data_rate_limited() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let _mock = server
            .mock("GET", "/status/test_program")
            .match_header("Authorization", "Bearer test_key")
            .with_status(429)
            .with_body("Too Many Requests")
            .create_async()
            .await;

        let client = OtterSecClient::new_with_config(mock_url, "test_key".to_string());
        let result = client.get_verification_data("test_program").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("rate limited"));
    }

    #[tokio::test]
    async fn test_get_verification_data_api_error() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let _mock = server
            .mock("GET", "/status/error_program")
            .match_header("Authorization", "Bearer test_key")
            .with_status(404)
            .with_body("Not Found")
            .create_async()
            .await;

        let client = OtterSecClient::new_with_config(mock_url, "test_key".to_string());
        let result = client.get_verification_data("error_program").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("API error"));
    }
}
