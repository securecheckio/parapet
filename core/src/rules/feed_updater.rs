use anyhow::Result;
use serde::{Deserialize, Serialize};
#[cfg(feature = "reqwest")]
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
#[cfg(feature = "reqwest")]
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(feature = "reqwest")]
use tokio::sync::RwLock;
use tokio::time::sleep;

/// Configuration for automatic rule feed updates
#[derive(Debug, Clone)]
pub struct FeedConfig {
    /// Multiple feed sources in priority order (first = highest priority)
    pub feed_sources: Vec<FeedSource>,
    /// How often to poll for updates (seconds)
    pub poll_interval: u64,
    /// Whether to enable automatic updates
    pub enabled: bool,
}

/// A single feed source with caching metadata
#[derive(Debug, Clone)]
pub struct FeedSource {
    /// URL to fetch rules from
    pub url: String,
    /// Optional name for logging
    pub name: Option<String>,
    /// Priority (lower = higher priority, used for conflict resolution)
    pub priority: u32,
    /// Minimum seconds between requests to this source (rate limiting)
    pub min_request_interval: u64,
}

/// Cache entry for a feed source
#[cfg(feature = "reqwest")]
#[derive(Debug, Clone)]
struct FeedCacheEntry {
    etag: Option<String>,
    last_modified: Option<String>,
    last_fetch: u64, // Unix timestamp
    cached_feed: Option<RuleFeed>,
}

/// Rule feed response format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleFeed {
    /// Feed format version
    pub version: String,
    /// When this feed was published
    pub published_at: String,
    /// New or updated rules
    #[serde(default)]
    pub rules: Vec<super::types::RuleDefinition>,
    /// Rule IDs that should be removed/deprecated
    #[serde(default)]
    pub deprecated_rule_ids: Vec<String>,
    /// Optional source identifier
    #[serde(default)]
    pub source: Option<String>,
    /// Optional: Nested feed sources for meta-feed composition
    /// Note: Currently for documentation only - use config.toml with multiple [[rule_feeds.sources]] for composition
    #[serde(default)]
    pub feeds: Vec<serde_json::Value>,
}

/// Merged rules from multiple sources
#[derive(Debug, Clone)]
pub struct MergedRuleFeed {
    pub rules: Vec<super::types::RuleDefinition>,
    pub deprecated_rule_ids: Vec<String>,
    pub sources: Vec<String>,
}

/// Fetches rules from a remote feed with caching support (feed updater internal).
#[cfg(feature = "reqwest")]
async fn fetch_rules_from_feed_cached(
    url: &str,
    cache_entry: Option<&FeedCacheEntry>,
) -> Result<FetchResult> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let mut request = client.get(url).header("User-Agent", "parapet-proxy/1.0");

    // Add caching headers if we have cache
    if let Some(cache) = cache_entry {
        if let Some(etag) = &cache.etag {
            request = request.header("If-None-Match", etag);
        }
        if let Some(last_mod) = &cache.last_modified {
            request = request.header("If-Modified-Since", last_mod);
        }
    }

    let response = request.send().await?;

    // 304 Not Modified - use cached version
    if response.status() == 304 {
        if let Some(cache) = cache_entry {
            if let Some(cached_feed) = &cache.cached_feed {
                log::debug!("📦 Using cached feed (304 Not Modified)");
                return Ok(FetchResult::NotModified(cached_feed.clone()));
            }
        }
        anyhow::bail!("Got 304 but no cached feed available");
    }

    if !response.status().is_success() {
        anyhow::bail!("Feed request failed: {}", response.status());
    }

    // Extract caching headers
    let etag = response
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let last_modified = response
        .headers()
        .get("last-modified")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let feed: RuleFeed = response.json().await?;
    log::info!(
        "📥 Fetched rule feed v{} with {} rules",
        feed.version,
        feed.rules.len()
    );

    Ok(FetchResult::Updated {
        feed,
        etag,
        last_modified,
    })
}

/// Result of a feed fetch
pub enum FetchResult {
    /// Feed was updated
    Updated {
        feed: RuleFeed,
        etag: Option<String>,
        last_modified: Option<String>,
    },
    /// Feed not modified (304), use cached version
    NotModified(RuleFeed),
}

/// Multi-source feed updater with caching and rate limiting
pub struct FeedUpdater {
    config: FeedConfig,
    #[cfg(feature = "reqwest")]
    cache: Arc<RwLock<HashMap<String, FeedCacheEntry>>>,
}

impl FeedUpdater {
    pub fn new(config: FeedConfig) -> Self {
        Self {
            config,
            #[cfg(feature = "reqwest")]
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Fetch from all sources and merge (`reqwest` required for HTTP).
    #[cfg(feature = "reqwest")]
    pub async fn fetch_all_sources(&self) -> Result<MergedRuleFeed> {
        let mut all_rules: HashMap<String, (super::types::RuleDefinition, u32)> = HashMap::new();
        let mut all_deprecated = Vec::new();
        let mut sources = Vec::new();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        for source in &self.config.feed_sources {
            // Check rate limit
            let should_fetch = {
                let cache = self.cache.read().await;
                if let Some(entry) = cache.get(&source.url) {
                    let elapsed = now.saturating_sub(entry.last_fetch);
                    elapsed >= source.min_request_interval
                } else {
                    true
                }
            };

            if !should_fetch {
                log::debug!("⏳ Skipping {} (rate limited)", source.url);
                continue;
            }

            // Fetch with caching
            let cache_entry = self.cache.read().await.get(&source.url).cloned();

            let result = match fetch_rules_from_feed_cached(&source.url, cache_entry.as_ref()).await
            {
                Ok(result) => result,
                Err(e) => {
                    log::warn!("Failed to fetch from {}: {}", source.url, e);
                    continue;
                }
            };

            let (feed, etag, last_modified) = match result {
                FetchResult::Updated {
                    feed,
                    etag,
                    last_modified,
                } => (feed, etag, last_modified),
                FetchResult::NotModified(feed) => {
                    log::debug!("📦 Using cached feed from {}", source.url);
                    (feed, None, None)
                }
            };

            // Update cache
            {
                let mut cache = self.cache.write().await;
                let old_entry = cache.get(&source.url).cloned();
                cache.insert(
                    source.url.clone(),
                    FeedCacheEntry {
                        etag: etag.or_else(|| old_entry.as_ref().and_then(|e| e.etag.clone())),
                        last_modified: last_modified
                            .or_else(|| old_entry.as_ref().and_then(|e| e.last_modified.clone())),
                        last_fetch: now,
                        cached_feed: Some(feed.clone()),
                    },
                );
            }

            let source_name = source
                .name
                .as_deref()
                .or(feed.source.as_deref())
                .unwrap_or(&source.url);
            sources.push(source_name.to_string());

            // Merge rules (priority-based conflict resolution)
            for rule in feed.rules {
                let rule_id = rule.id.clone();
                match all_rules.get(&rule_id) {
                    Some((_, existing_priority)) => {
                        if source.priority < *existing_priority {
                            log::debug!(
                                "🔄 Overriding rule {} from higher priority source",
                                rule_id
                            );
                            all_rules.insert(rule_id, (rule, source.priority));
                        }
                    }
                    None => {
                        all_rules.insert(rule_id, (rule, source.priority));
                    }
                }
            }

            // Collect deprecated rules
            all_deprecated.extend(feed.deprecated_rule_ids);
        }

        // Deduplicate deprecated rules
        all_deprecated.sort();
        all_deprecated.dedup();

        let rules: Vec<_> = all_rules.into_values().map(|(rule, _)| rule).collect();

        log::info!(
            "📊 Merged {} rules from {} sources",
            rules.len(),
            sources.len()
        );

        Ok(MergedRuleFeed {
            rules,
            deprecated_rule_ids: all_deprecated,
            sources,
        })
    }

    #[cfg(not(feature = "reqwest"))]
    pub async fn fetch_all_sources(&self) -> Result<MergedRuleFeed> {
        anyhow::bail!("parapet-core was built without `reqwest`; rule feed updates are unavailable")
    }

    /// Start background polling task
    pub async fn start_polling<F>(self, on_update: F)
    where
        F: Fn(MergedRuleFeed) -> Result<()> + Send + 'static,
    {
        if !self.config.enabled {
            log::info!("📭 Rule feed updates disabled");
            return;
        }

        log::info!("📡 Starting multi-source rule feed updater");
        log::info!(
            "   {} feed sources configured",
            self.config.feed_sources.len()
        );
        log::info!("   Polling every {} seconds", self.config.poll_interval);

        for (i, source) in self.config.feed_sources.iter().enumerate() {
            let name = source.name.as_deref().unwrap_or("unnamed");
            log::info!(
                "   [{}] {} (priority: {}, rate limit: {}s)",
                i + 1,
                name,
                source.priority,
                source.min_request_interval
            );
        }

        let updater = Arc::new(self);

        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(updater.config.poll_interval)).await;

                match updater.fetch_all_sources().await {
                    Ok(merged) => {
                        log::info!(
                            "✅ Rule feed update received from {} sources",
                            merged.sources.len()
                        );
                        if let Err(e) = on_update(merged) {
                            log::error!("Failed to apply rule updates: {}", e);
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to fetch rule feeds: {}", e);
                    }
                }
            }
        });
    }
}

#[cfg(all(test, feature = "reqwest"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_multi_source_fetch() {
        let config = FeedConfig {
            feed_sources: vec![FeedSource {
                url: "https://parapet-rules.securecheck.io/community/default-protection.json"
                    .to_string(),
                name: Some("Default Protection Rules".to_string()),
                priority: 0,
                min_request_interval: 60,
            }],
            poll_interval: 3600,
            enabled: true,
        };

        let updater = FeedUpdater::new(config);

        match updater.fetch_all_sources().await {
            Ok(merged) => {
                println!(
                    "✅ Fetched {} rules from {} sources",
                    merged.rules.len(),
                    merged.sources.len()
                );
            }
            Err(e) => {
                println!("⚠️  Feed not available yet: {}", e);
            }
        }
    }
}
