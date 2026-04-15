use anyhow::{anyhow, Result};
use reqwest::header::{ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use std::time::Instant;
use tokio::sync::RwLock;

use crate::rules::analyzers::BlockedHash;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedHashFeedEntry {
    pub program_id: String,
    pub hash: String,
    pub reason: Option<String>,
    pub severity: Option<String>,
    pub detected_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlocklistFeed {
    pub version: String,
    pub updated_at: String,
    #[serde(default)]
    pub blocked_programs: Vec<String>,
    #[serde(default)]
    pub blocked_hashes: Vec<BlockedHashFeedEntry>,
}

#[derive(Debug, Clone, Default)]
pub struct ProgramBlocklistState {
    pub blocked_programs: HashSet<String>,
    pub blocked_hashes: Vec<BlockedHash>,
}

pub struct FeedPoller {
    feeds: Vec<String>,
    state: std::sync::Arc<RwLock<ProgramBlocklistState>>,
    interval: Duration,
    client: reqwest::Client,
    states: std::sync::Arc<RwLock<HashMap<String, FeedState>>>,
}

impl FeedPoller {
    pub fn new(
        feeds: Vec<String>,
        state: std::sync::Arc<RwLock<ProgramBlocklistState>>,
        interval: Duration,
    ) -> Self {
        Self {
            feeds,
            state,
            interval,
            client: reqwest::Client::new(),
            states: std::sync::Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start(&self) {
        loop {
            if let Err(err) = self.poll_once().await {
                log::warn!("Program blocklist feed poll failed: {err}");
            }
            tokio::time::sleep(self.interval).await;
        }
    }

    pub async fn poll_once(&self) -> Result<()> {
        let mut merged_programs = HashSet::new();
        let mut merged_hashes: Vec<BlockedHash> = Vec::new();

        for feed_url in &self.feeds {
            if !self.is_ready_for_request(feed_url).await {
                log::debug!("Skipping feed due to backoff window: {feed_url}");
                continue;
            }

            match self.fetch_feed(feed_url).await {
                Ok(Some(feed)) => {
                    self.mark_success(feed_url).await;
                    merged_programs.extend(feed.blocked_programs);
                    merged_hashes.extend(feed.blocked_hashes.into_iter().map(|entry| {
                        BlockedHash {
                            program_id: entry.program_id,
                            hash: entry.hash,
                        }
                    }));
                }
                Ok(None) => {
                    self.mark_success(feed_url).await;
                    log::debug!("Feed not modified since last fetch: {feed_url}");
                }
                Err(err) => {
                    if err.retryable {
                        self.mark_retryable_error(feed_url).await;
                    } else {
                        self.mark_non_retryable_error(feed_url).await;
                    }
                    log::warn!("Failed to fetch feed {feed_url}: {}", err.message);
                }
            }
        }

        let mut state = self.state.write().await;
        state.blocked_programs = merged_programs;
        state.blocked_hashes = merged_hashes;
        Ok(())
    }

    async fn fetch_feed(
        &self,
        url: &str,
    ) -> std::result::Result<Option<BlocklistFeed>, FeedFetchError> {
        let mut request = self.client.get(url).timeout(Duration::from_secs(10));

        if let Some(state) = self.states.read().await.get(url).cloned() {
            if let Some(etag) = state.validator.etag {
                request = request.header(IF_NONE_MATCH, etag);
            }
            if let Some(last_modified) = state.validator.last_modified {
                request = request.header(IF_MODIFIED_SINCE, last_modified);
            }
        }

        let response = request.send().await.map_err(|err| FeedFetchError {
            message: err.to_string(),
            retryable: true,
        })?;

        if response.status() == reqwest::StatusCode::NOT_MODIFIED {
            return Ok(None);
        }

        if !response.status().is_success() {
            let status = response.status();
            let retryable =
                status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error();
            return Err(FeedFetchError {
                message: format!("feed returned HTTP {}", status),
                retryable,
            });
        }

        let etag = response
            .headers()
            .get(ETAG)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let last_modified = response
            .headers()
            .get(LAST_MODIFIED)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);

        let feed: BlocklistFeed = response.json().await.map_err(|err| FeedFetchError {
            message: err.to_string(),
            retryable: false,
        })?;
        if feed.version.trim().is_empty() {
            return Err(FeedFetchError {
                message: anyhow!("feed missing version").to_string(),
                retryable: false,
            });
        }

        {
            let mut states = self.states.write().await;
            let state = states.entry(url.to_string()).or_default();
            state.validator.etag = etag;
            state.validator.last_modified = last_modified;
        }

        Ok(Some(feed))
    }

    async fn is_ready_for_request(&self, url: &str) -> bool {
        let states = self.states.read().await;
        match states.get(url) {
            Some(state) => state.next_allowed_attempt <= Instant::now(),
            None => true,
        }
    }

    async fn mark_success(&self, url: &str) {
        let mut states = self.states.write().await;
        let state = states.entry(url.to_string()).or_default();
        state.failures = 0;
        state.next_allowed_attempt = Instant::now();
    }

    async fn mark_retryable_error(&self, url: &str) {
        let mut states = self.states.write().await;
        let state = states.entry(url.to_string()).or_default();
        state.failures = state.failures.saturating_add(1);
        let backoff = compute_backoff_with_jitter(self.interval, state.failures, url);
        state.next_allowed_attempt = Instant::now() + backoff;
    }

    async fn mark_non_retryable_error(&self, url: &str) {
        let mut states = self.states.write().await;
        let state = states.entry(url.to_string()).or_default();
        state.failures = 0;
        state.next_allowed_attempt = Instant::now() + self.interval;
    }
}

#[derive(Debug, Clone, Default)]
struct FeedValidatorState {
    etag: Option<String>,
    last_modified: Option<String>,
}

#[derive(Debug, Clone)]
struct FeedFetchError {
    message: String,
    retryable: bool,
}

#[derive(Debug, Clone)]
struct FeedState {
    validator: FeedValidatorState,
    failures: u32,
    next_allowed_attempt: Instant,
}

impl Default for FeedState {
    fn default() -> Self {
        Self {
            validator: FeedValidatorState::default(),
            failures: 0,
            next_allowed_attempt: Instant::now(),
        }
    }
}

fn compute_backoff_with_jitter(base_interval: Duration, failures: u32, url: &str) -> Duration {
    let exponent = failures.min(6);
    let multiplier = 1u64 << exponent;
    let capped = base_interval
        .as_millis()
        .saturating_mul(multiplier as u128)
        .min(Duration::from_secs(3600).as_millis()) as u64;

    let jitter_window_ms = (capped / 5).max(250); // 20% jitter, min 250ms
    let mut hash = 1469598103934665603u64;
    for b in url.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    let jitter_ms = hash % jitter_window_ms;
    Duration::from_millis(capped + jitter_ms)
}
