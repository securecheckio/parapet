use crate::client::{UpstreamClient, UpstreamHttpSettings};
use crate::json::{JsonRpcRequest, JsonRpcResponse};
use crate::UpstreamProvider;
use anyhow::Result;
use async_trait::async_trait;

/// Priority-ordered failover across multiple [`UpstreamClient`] instances.
#[derive(Clone)]
pub struct FailoverUpstreamProvider {
    upstreams: Vec<UpstreamClient>,
}

impl FailoverUpstreamProvider {
    pub fn new(configs: Vec<(String, UpstreamHttpSettings)>) -> Self {
        let upstreams = configs
            .into_iter()
            .map(|(url, config)| UpstreamClient::new_with_config(url, config))
            .collect();
        Self { upstreams }
    }

    async fn try_forward(&self, request: &JsonRpcRequest) -> Result<JsonRpcResponse> {
        let mut last_error = None;

        for (idx, client) in self.upstreams.iter().enumerate() {
            if !client.circuit_call_permitted().await {
                log::debug!("Skipping upstream {} — circuit breaker open", idx);
                continue;
            }

            log::debug!("Trying upstream {} ({})", idx, client.upstream_url);

            match client.forward(request).await {
                Ok(response) => {
                    if idx > 0 {
                        log::info!("Succeeded on fallback upstream {}", idx);
                    }
                    return Ok(response);
                }
                Err(e) => {
                    log::warn!("Upstream {} failed: {}", idx, e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All upstreams unavailable")))
    }

    async fn try_get_account(&self, pubkey: &str) -> Result<Option<Vec<u8>>> {
        let mut last_error = None;

        for client in &self.upstreams {
            if !client.circuit_call_permitted().await {
                continue;
            }

            match client.get_account(pubkey).await {
                Ok(account) => return Ok(account),
                Err(e) => last_error = Some(e),
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All upstreams unavailable")))
    }

    async fn try_get_multiple_accounts(&self, pubkeys: &[String]) -> Result<Vec<Option<Vec<u8>>>> {
        let mut last_error = None;

        for client in &self.upstreams {
            if !client.circuit_call_permitted().await {
                continue;
            }

            match client.get_multiple_accounts(pubkeys).await {
                Ok(accounts) => return Ok(accounts),
                Err(e) => last_error = Some(e),
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All upstreams unavailable")))
    }
}

#[async_trait]
impl UpstreamProvider for FailoverUpstreamProvider {
    async fn forward(&self, request: &JsonRpcRequest) -> Result<JsonRpcResponse> {
        self.try_forward(request).await
    }

    async fn get_account(&self, pubkey: &str) -> Result<Option<Vec<u8>>> {
        self.try_get_account(pubkey).await
    }

    async fn get_multiple_accounts(&self, pubkeys: &[String]) -> Result<Vec<Option<Vec<u8>>>> {
        self.try_get_multiple_accounts(pubkeys).await
    }

    fn get_upstream_url(&self) -> String {
        self.upstreams
            .first()
            .map(|c| c.upstream_url.clone())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::{JsonRpcRequest, JsonRpcResponse};
    use crate::UpstreamProvider;

    fn sample_request() -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: serde_json::json!(1),
            method: "getHealth".to_string(),
            params: vec![],
        }
    }

    #[tokio::test]
    async fn failover_reaches_second_when_first_unreachable() {
        let settings = UpstreamHttpSettings {
            max_concurrent: 2,
            delay_ms: 0,
            timeout_secs: 2,
            max_retries: 0,
            retry_base_delay_ms: 10,
            circuit_breaker_threshold: 1,
            circuit_breaker_timeout_secs: 60,
        };
        let bad = "http://127.0.0.1:9";
        let good = "https://api.devnet.solana.com";
        let fb = FailoverUpstreamProvider::new(vec![
            (bad.to_string(), settings.clone()),
            (good.to_string(), settings),
        ]);
        let resp: JsonRpcResponse = fb
            .forward(&sample_request())
            .await
            .expect("devnet getHealth");
        assert!(resp.error.is_none());
    }
}
