use crate::circuit::{CircuitBreaker, CircuitState};
use crate::json::{JsonRpcRequest, JsonRpcResponse};
use anyhow::Result;
use async_trait::async_trait;
use base64::Engine;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{sleep, Duration};

use crate::UpstreamProvider;

/// Per-endpoint HTTP tuning (timeouts, retries, circuit breaker).
#[derive(Debug, Clone)]
pub struct UpstreamHttpSettings {
    pub max_concurrent: usize,
    pub delay_ms: u64,
    pub timeout_secs: u64,
    pub max_retries: usize,
    pub retry_base_delay_ms: u64,
    pub circuit_breaker_threshold: usize,
    pub circuit_breaker_timeout_secs: u64,
}

impl Default for UpstreamHttpSettings {
    fn default() -> Self {
        Self {
            max_concurrent: 10,
            delay_ms: 100,
            timeout_secs: 30,
            max_retries: 3,
            retry_base_delay_ms: 100,
            circuit_breaker_threshold: 5,
            circuit_breaker_timeout_secs: 60,
        }
    }
}

/// Single upstream JSON-RPC over HTTP (building block for failover strategies).
#[derive(Clone)]
pub struct UpstreamClient {
    client: Client,
    pub upstream_url: String,
    concurrency_limiter: Arc<Semaphore>,
    request_delay_ms: u64,
    circuit_breaker: CircuitBreaker,
    max_retries: usize,
    retry_base_delay_ms: u64,
}

impl UpstreamClient {
    pub fn new(upstream_url: String) -> Self {
        Self::new_with_config(upstream_url, UpstreamHttpSettings::default())
    }

    pub fn new_with_limits(upstream_url: String, max_concurrent: usize, delay_ms: u64) -> Self {
        Self::new_with_config(
            upstream_url,
            UpstreamHttpSettings {
                max_concurrent,
                delay_ms,
                ..Default::default()
            },
        )
    }

    pub fn new_with_config(upstream_url: String, config: UpstreamHttpSettings) -> Self {
        log::info!(
            "Upstream HTTP: max {} concurrent, {}ms delay, timeout {}s, retries {}, circuit breaker threshold {}",
            config.max_concurrent,
            config.delay_ms,
            config.timeout_secs,
            config.max_retries,
            config.circuit_breaker_threshold
        );

        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            upstream_url,
            concurrency_limiter: Arc::new(Semaphore::new(config.max_concurrent)),
            request_delay_ms: config.delay_ms,
            circuit_breaker: CircuitBreaker::new(
                config.circuit_breaker_threshold,
                config.circuit_breaker_timeout_secs,
            ),
            max_retries: config.max_retries,
            retry_base_delay_ms: config.retry_base_delay_ms,
        }
    }

    /// Whether this endpoint may accept a new request (circuit breaker gate).
    pub async fn circuit_call_permitted(&self) -> bool {
        self.circuit_breaker.call_permitted().await
    }

    pub async fn forward(&self, request: &JsonRpcRequest) -> Result<JsonRpcResponse> {
        log::debug!("Forwarding to upstream: method={}", request.method);

        if !self.circuit_breaker.call_permitted().await {
            let state = self.circuit_breaker.get_state().await;
            return Err(anyhow::anyhow!(
                "Circuit breaker is {:?} - upstream service is unavailable",
                state
            ));
        }

        let _permit = self.concurrency_limiter.acquire().await?;
        log::debug!("Acquired upstream request permit");

        if self.request_delay_ms > 0 {
            sleep(Duration::from_millis(self.request_delay_ms)).await;
        }

        let mut attempt = 0;
        let mut last_error = None;

        while attempt <= self.max_retries {
            if attempt > 0 {
                let backoff_ms = self.retry_base_delay_ms * 2u64.pow(attempt as u32 - 1);
                log::debug!("Retry attempt {} after {}ms backoff", attempt, backoff_ms);
                sleep(Duration::from_millis(backoff_ms)).await;
            }

            match self.try_request(request).await {
                Ok(response) => {
                    self.circuit_breaker.record_success().await;
                    return Ok(response);
                }
                Err(e) => {
                    last_error = Some(e);
                    attempt += 1;

                    if let Some(err) = &last_error {
                        if !Self::is_retryable_error(err) {
                            log::debug!("Non-retryable error, not retrying");
                            break;
                        }
                    }
                }
            }
        }

        self.circuit_breaker.record_failure().await;

        Err(last_error
            .unwrap_or_else(|| anyhow::anyhow!("Request failed after {} attempts", attempt)))
    }

    async fn try_request(&self, request: &JsonRpcRequest) -> Result<JsonRpcResponse> {
        let response = self
            .client
            .post(&self.upstream_url)
            .json(request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Upstream returned error: status={}, body={}",
                status,
                body
            ));
        }

        let rpc_response: JsonRpcResponse = response.json().await?;
        log::debug!("Received response from upstream");

        Ok(rpc_response)
    }

    pub fn is_retryable_error(error: &anyhow::Error) -> bool {
        let error_str = error.to_string().to_lowercase();

        if error_str.contains("connection")
            || error_str.contains("timeout")
            || error_str.contains("network")
            || error_str.contains("dns")
        {
            return true;
        }

        if error_str.contains("status=5") {
            return true;
        }

        if error_str.contains("status=429") {
            return true;
        }

        false
    }

    pub async fn get_circuit_state(&self) -> CircuitState {
        self.circuit_breaker.get_state().await
    }

    pub async fn get_account(&self, pubkey: &str) -> Result<Option<Vec<u8>>> {
        use serde_json::json;

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: serde_json::Value::Number(1.into()),
            method: "getAccountInfo".to_string(),
            params: vec![
                serde_json::Value::String(pubkey.to_string()),
                json!({
                    "encoding": "base64"
                }),
            ],
        };

        let response = self.forward(&request).await?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!(
                "RPC error fetching account: {}",
                error.message
            ));
        }

        if let Some(result) = response.result {
            if result.is_null() {
                return Ok(None);
            }

            let data = result
                .get("value")
                .and_then(|v| v.get("data"))
                .and_then(|d| d.get(0))
                .and_then(|s| s.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid account data format"))?;

            let decoded = base64::engine::general_purpose::STANDARD
                .decode(data)
                .map_err(|e| anyhow::anyhow!("Failed to decode account data: {}", e))?;

            Ok(Some(decoded))
        } else {
            Ok(None)
        }
    }

    pub async fn get_multiple_accounts(&self, pubkeys: &[String]) -> Result<Vec<Option<Vec<u8>>>> {
        use serde_json::json;

        if pubkeys.is_empty() {
            return Ok(vec![]);
        }

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: serde_json::Value::Number(1.into()),
            method: "getMultipleAccounts".to_string(),
            params: vec![
                serde_json::Value::Array(
                    pubkeys
                        .iter()
                        .map(|pk| serde_json::Value::String(pk.clone()))
                        .collect(),
                ),
                json!({
                    "encoding": "base64"
                }),
            ],
        };

        let response = self.forward(&request).await?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!(
                "RPC error fetching accounts: {}",
                error.message
            ));
        }

        let result = response
            .result
            .ok_or_else(|| anyhow::anyhow!("No result in response"))?;
        let values = result
            .get("value")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Invalid response format"))?;

        let mut accounts = Vec::new();
        for value in values {
            if value.is_null() {
                accounts.push(None);
                continue;
            }

            let data = value
                .get("data")
                .and_then(|d| d.get(0))
                .and_then(|s| s.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid account data format"))?;

            let decoded = base64::engine::general_purpose::STANDARD
                .decode(data)
                .map_err(|e| anyhow::anyhow!("Failed to decode account data: {}", e))?;

            accounts.push(Some(decoded));
        }

        Ok(accounts)
    }
}

#[async_trait]
impl UpstreamProvider for UpstreamClient {
    async fn forward(&self, request: &JsonRpcRequest) -> Result<JsonRpcResponse> {
        UpstreamClient::forward(self, request).await
    }

    async fn get_account(&self, pubkey: &str) -> Result<Option<Vec<u8>>> {
        UpstreamClient::get_account(self, pubkey).await
    }

    async fn get_multiple_accounts(&self, pubkeys: &[String]) -> Result<Vec<Option<Vec<u8>>>> {
        UpstreamClient::get_multiple_accounts(self, pubkeys).await
    }

    fn get_upstream_url(&self) -> String {
        self.upstream_url.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retryable_error_classification() {
        assert!(UpstreamClient::is_retryable_error(&anyhow::anyhow!(
            "status=500"
        )));
        assert!(UpstreamClient::is_retryable_error(&anyhow::anyhow!(
            "status=429"
        )));
        assert!(UpstreamClient::is_retryable_error(&anyhow::anyhow!(
            "connection reset"
        )));
        assert!(!UpstreamClient::is_retryable_error(&anyhow::anyhow!(
            "status=400"
        )));
    }
}
