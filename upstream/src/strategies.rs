//! Example pluggable strategy: prefer lower observed latency, with optional slot-lag filtering.

use crate::client::{UpstreamClient, UpstreamHttpSettings};
use crate::json::{JsonRpcRequest, JsonRpcResponse};
use crate::UpstreamProvider;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::time::Instant;

/// Reference smart router: prefers lower observed latency, then priority index order.
///
/// Slot hints are updated via `getSlot` when forwarding non-`getSlot` calls. Not wired by default
/// in the proxy; included as a copy-paste-friendly example for custom deployments.
#[derive(Clone)]
pub struct SmartUpstreamProvider {
    upstreams: Arc<Vec<UpstreamClient>>,
    latency_sum_ms: Arc<Vec<AtomicU64>>,
    success_count: Arc<Vec<AtomicU64>>,
    last_slot: Arc<Vec<AtomicU64>>,
    max_slot_lag: u64,
}

impl SmartUpstreamProvider {
    pub fn new(configs: Vec<(String, UpstreamHttpSettings)>, max_slot_lag: u64) -> Self {
        let upstreams: Vec<_> = configs
            .into_iter()
            .map(|(url, cfg)| UpstreamClient::new_with_config(url, cfg))
            .collect();
        let n = upstreams.len();
        Self {
            upstreams: Arc::new(upstreams),
            latency_sum_ms: Arc::new((0..n).map(|_| AtomicU64::new(0)).collect()),
            success_count: Arc::new((0..n).map(|_| AtomicU64::new(0)).collect()),
            last_slot: Arc::new((0..n).map(|_| AtomicU64::new(0)).collect()),
            max_slot_lag,
        }
    }

    fn ordered_indices(&self) -> Vec<usize> {
        let n = self.upstreams.len();
        let mut idx: Vec<usize> = (0..n).collect();
        idx.sort_by(|&a, &b| self.avg_latency_ms(a).cmp(&self.avg_latency_ms(b)));
        idx
    }

    fn avg_latency_ms(&self, idx: usize) -> u64 {
        let c = self.success_count[idx].load(Ordering::Relaxed).max(1);
        self.latency_sum_ms[idx].load(Ordering::Relaxed) / c
    }

    async fn probe_slots(&self) {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: serde_json::json!(1),
            method: "getSlot".to_string(),
            params: vec![],
        };
        for (i, c) in self.upstreams.iter().enumerate() {
            if !c.circuit_call_permitted().await {
                continue;
            }
            if let Ok(resp) = c.forward(&req).await {
                if let Some(v) = resp.result.and_then(|r| r.as_u64()) {
                    self.last_slot[i].store(v, Ordering::Relaxed);
                }
            }
        }
    }

    fn slot_ok(&self, idx: usize, leader: u64) -> bool {
        let s = self.last_slot[idx].load(Ordering::Relaxed);
        if s == 0 || leader == 0 {
            return true;
        }
        leader.saturating_sub(s) <= self.max_slot_lag
    }

    fn leader_slot(&self) -> u64 {
        self.last_slot
            .iter()
            .map(|a| a.load(Ordering::Relaxed))
            .max()
            .unwrap_or(0)
    }
}

#[async_trait]
impl UpstreamProvider for SmartUpstreamProvider {
    async fn forward(&self, request: &JsonRpcRequest) -> Result<JsonRpcResponse> {
        if self.upstreams.is_empty() {
            return Err(anyhow::anyhow!("No upstreams configured"));
        }

        if request.method != "getSlot" {
            self.probe_slots().await;
        }

        let leader = self.leader_slot();
        let order = self.ordered_indices();
        let mut last_err = None;

        for &idx in &order {
            let c = &self.upstreams[idx];
            if !c.circuit_call_permitted().await {
                continue;
            }
            if request.method != "getSlot" && !self.slot_ok(idx, leader) {
                continue;
            }
            let start = Instant::now();
            match c.forward(request).await {
                Ok(resp) => {
                    let ms = start.elapsed().as_millis() as u64;
                    self.latency_sum_ms[idx].fetch_add(ms, Ordering::Relaxed);
                    self.success_count[idx].fetch_add(1, Ordering::Relaxed);
                    return Ok(resp);
                }
                Err(e) => last_err = Some(e),
            }
        }

        for (idx, c) in self.upstreams.iter().enumerate() {
            if !c.circuit_call_permitted().await {
                continue;
            }
            let start = Instant::now();
            match c.forward(request).await {
                Ok(resp) => {
                    let ms = start.elapsed().as_millis() as u64;
                    self.latency_sum_ms[idx].fetch_add(ms, Ordering::Relaxed);
                    self.success_count[idx].fetch_add(1, Ordering::Relaxed);
                    return Ok(resp);
                }
                Err(e) => last_err = Some(e),
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("All upstreams unavailable")))
    }

    async fn get_account(&self, pubkey: &str) -> Result<Option<Vec<u8>>> {
        if self.upstreams.is_empty() {
            return Err(anyhow::anyhow!("No upstreams configured"));
        }
        if self.upstreams.len() == 1 {
            return self.upstreams[0].get_account(pubkey).await;
        }

        self.probe_slots().await;
        let leader = self.leader_slot();
        let order = self.ordered_indices();
        let mut last_err = None;

        for &idx in &order {
            let c = &self.upstreams[idx];
            if !c.circuit_call_permitted().await {
                continue;
            }
            if !self.slot_ok(idx, leader) {
                continue;
            }
            match c.get_account(pubkey).await {
                Ok(v) => return Ok(v),
                Err(e) => last_err = Some(e),
            }
        }

        for c in self.upstreams.iter() {
            if !c.circuit_call_permitted().await {
                continue;
            }
            match c.get_account(pubkey).await {
                Ok(v) => return Ok(v),
                Err(e) => last_err = Some(e),
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("All upstreams unavailable")))
    }

    async fn get_multiple_accounts(&self, pubkeys: &[String]) -> Result<Vec<Option<Vec<u8>>>> {
        if self.upstreams.is_empty() {
            return Err(anyhow::anyhow!("No upstreams configured"));
        }
        if self.upstreams.len() == 1 {
            return self.upstreams[0].get_multiple_accounts(pubkeys).await;
        }

        self.probe_slots().await;
        let leader = self.leader_slot();
        let order = self.ordered_indices();
        let mut last_err = None;

        for &idx in &order {
            let c = &self.upstreams[idx];
            if !c.circuit_call_permitted().await {
                continue;
            }
            if !self.slot_ok(idx, leader) {
                continue;
            }
            match c.get_multiple_accounts(pubkeys).await {
                Ok(v) => return Ok(v),
                Err(e) => last_err = Some(e),
            }
        }

        for c in self.upstreams.iter() {
            if !c.circuit_call_permitted().await {
                continue;
            }
            match c.get_multiple_accounts(pubkeys).await {
                Ok(v) => return Ok(v),
                Err(e) => last_err = Some(e),
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("All upstreams unavailable")))
    }

    fn get_upstream_url(&self) -> String {
        self.upstreams
            .first()
            .map(|c| c.upstream_url.clone())
            .unwrap_or_default()
    }
}
