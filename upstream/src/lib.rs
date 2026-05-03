//! Shared Solana JSON-RPC upstream HTTP client with pluggable routing and failover.
//!
//! Use [`UpstreamClient`] for a single endpoint, [`FailoverUpstreamProvider`] for priority failover
//! (e.g. after HTTP 429), and [`SmartUpstreamProvider`] as an example latency/slot-aware strategy.

pub mod circuit;
pub mod client;
pub mod config;
pub mod failover;
pub mod json;
pub mod strategies;

pub use circuit::{CircuitBreaker, CircuitState};
pub use client::{UpstreamClient, UpstreamHttpSettings};
pub use config::parse_upstream_urls_list;
pub use failover::FailoverUpstreamProvider;
pub use json::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
pub use strategies::SmartUpstreamProvider;

use std::sync::Arc;

/// Pluggable upstream JSON-RPC transport (async).
#[async_trait::async_trait]
pub trait UpstreamProvider: Send + Sync {
    async fn forward(&self, request: &JsonRpcRequest) -> anyhow::Result<JsonRpcResponse>;
    async fn get_account(&self, pubkey: &str) -> anyhow::Result<Option<Vec<u8>>>;
    async fn get_multiple_accounts(
        &self,
        pubkeys: &[String],
    ) -> anyhow::Result<Vec<Option<Vec<u8>>>>;
    /// Primary URL for logging or legacy single-URL consumers.
    fn get_upstream_url(&self) -> String;
}

/// Build a single client or failover stack from an ordered URL list and shared HTTP settings.
pub fn build_upstream_stack(
    urls: Vec<String>,
    settings: UpstreamHttpSettings,
) -> anyhow::Result<Arc<dyn UpstreamProvider>> {
    if urls.is_empty() {
        anyhow::bail!("At least one upstream RPC URL is required");
    }
    if urls.len() == 1 {
        return Ok(Arc::new(UpstreamClient::new_with_config(
            urls[0].clone(),
            settings,
        )));
    }
    let configs: Vec<_> = urls.into_iter().map(|u| (u, settings.clone())).collect();
    Ok(Arc::new(FailoverUpstreamProvider::new(configs)))
}

/// Same as [`build_upstream_stack`] but optionally uses [`SmartUpstreamProvider`].
pub fn build_upstream_stack_with_strategy(
    urls: Vec<String>,
    settings: UpstreamHttpSettings,
    strategy: Option<&str>,
    smart_max_slot_lag: u64,
) -> anyhow::Result<Arc<dyn UpstreamProvider>> {
    if urls.is_empty() {
        anyhow::bail!("At least one upstream RPC URL is required");
    }
    if urls.len() == 1 {
        return Ok(Arc::new(UpstreamClient::new_with_config(
            urls[0].clone(),
            settings,
        )));
    }
    let configs: Vec<_> = urls.into_iter().map(|u| (u, settings.clone())).collect();
    match strategy {
        Some("smart") => Ok(Arc::new(SmartUpstreamProvider::new(
            configs,
            smart_max_slot_lag,
        ))),
        _ => Ok(Arc::new(FailoverUpstreamProvider::new(configs))),
    }
}
