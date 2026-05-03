use anyhow::Result;
use parapet_upstream::parse_upstream_urls_list;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub upstream: UpstreamConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub usage: UsageConfig,
    #[serde(default)]
    pub redis: RedisConfig,
    #[serde(default)]
    pub wasm: WasmConfig,
    #[serde(default)]
    pub escalations: EscalationsConfig,
    #[serde(default)]
    pub rule_feeds: RuleFeedsConfig,
    #[serde(default)]
    pub activity_feed: ActivityFeedConfig,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
}

#[derive(Debug, Deserialize)]
pub struct UpstreamConfig {
    /// Single RPC URL (mutually exclusive with `endpoint` when non-empty).
    #[serde(default)]
    pub url: String,
    /// Multiple RPC endpoints with priority (lower `priority` tried first).
    #[serde(default)]
    pub endpoint: Vec<UpstreamEndpoint>,
    /// Optional routing strategy: `failover` (default) or `smart` (latency / slot hints).
    #[serde(default)]
    pub strategy: Option<String>,
    #[serde(default = "default_upstream_smart_max_slot_lag")]
    pub smart_max_slot_lag: u64,
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,
    #[serde(default = "default_delay_ms")]
    pub delay_ms: u64,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "default_max_retries")]
    pub max_retries: usize,
    #[serde(default = "default_retry_base_delay_ms")]
    pub retry_base_delay_ms: u64,
    #[serde(default = "default_circuit_breaker_threshold")]
    pub circuit_breaker_threshold: usize,
    #[serde(default = "default_circuit_breaker_timeout_secs")]
    pub circuit_breaker_timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpstreamEndpoint {
    pub url: String,
    #[serde(default)]
    pub priority: u32,
    pub max_concurrent: Option<usize>,
    pub delay_ms: Option<u64>,
    pub timeout_secs: Option<u64>,
    pub max_retries: Option<usize>,
    pub retry_base_delay_ms: Option<u64>,
    pub circuit_breaker_threshold: Option<usize>,
    pub circuit_breaker_timeout_secs: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct NetworkConfig {
    #[serde(default = "default_network")]
    pub network: String,
    /// Address Lookup Tables to pre-fetch on startup (improves v0 transaction performance)
    #[serde(default)]
    pub prefetch_alts: Vec<String>,
    /// ALT cache TTL in seconds (default: 3600 = 1 hour)
    #[serde(default = "default_alt_cache_ttl")]
    pub alt_cache_ttl_secs: u64,
}

#[derive(Debug, Deserialize)]
pub struct SecurityConfig {
    #[serde(default = "default_blocking_threshold")]
    pub default_blocking_threshold: u8,
    pub rules_path: Option<String>,
    pub rule_action_override: Option<String>,
    pub blocked_programs: Option<Vec<String>>,
    pub blocked_hashes: Option<Vec<BlockedHash>>,
    pub blocked_program_feeds: Option<Vec<String>>,
    #[serde(default = "default_feed_poll_interval")]
    pub feed_poll_interval_secs: u64,
    /// When non-empty, only these JSON-RPC methods are accepted (public allowlist).
    #[serde(default)]
    pub allowed_methods: Vec<String>,
    /// Methods always rejected (applied before `allowed_methods`).
    #[serde(default)]
    pub blocked_methods: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlockedHash {
    pub program_id: String,
    pub hash: String,
    pub reason: Option<String>,
    pub added_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AuthConfig {
    #[serde(default)]
    pub mode: String, // "none", "api_key", "wallet_allowlist"
    pub api_keys: Option<String>,
    pub allowed_wallets: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UsageConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_requests_per_month")]
    pub default_requests_per_month: u64,
}

#[derive(Debug, Deserialize)]
pub struct RedisConfig {
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WasmConfig {
    pub analyzers_path: Option<String>,
    pub analyzer_config: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EscalationsConfig {
    #[serde(default)]
    pub enabled: bool,
    pub approver_wallet: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RuleFeedsConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_poll_interval")]
    pub poll_interval: u64,
    #[serde(default = "default_min_interval")]
    pub default_min_interval: u64,
    #[serde(default)]
    pub sources: Vec<FeedSourceConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ActivityFeedConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_min_risk_score")]
    pub min_risk_score: u8,
    #[serde(default = "default_max_events_per_wallet")]
    pub max_events_per_wallet: usize,
    #[serde(default = "default_ttl_seconds")]
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeedSourceConfig {
    pub url: String,
    pub name: Option<String>,
    #[serde(default)]
    pub priority: u32,
    pub min_interval: Option<u64>,
}

// Defaults
fn default_port() -> u16 {
    8899
}
fn default_bind_address() -> String {
    "0.0.0.0".to_string()
}
fn default_max_concurrent() -> usize {
    10
}
fn default_delay_ms() -> u64 {
    100
}
fn default_timeout_secs() -> u64 {
    30
}
fn default_max_retries() -> usize {
    3
}
fn default_retry_base_delay_ms() -> u64 {
    100
}
fn default_circuit_breaker_threshold() -> usize {
    5
}
fn default_circuit_breaker_timeout_secs() -> u64 {
    60
}
fn default_network() -> String {
    "mainnet-beta".to_string()
}
fn default_alt_cache_ttl() -> u64 {
    3600 // 1 hour
}
fn default_blocking_threshold() -> u8 {
    70
}
fn default_requests_per_month() -> u64 {
    10_000
}
fn default_poll_interval() -> u64 {
    3600
}
fn default_feed_poll_interval() -> u64 {
    3600
}
fn default_min_interval() -> u64 {
    300
}
fn default_min_risk_score() -> u8 {
    40
}
fn default_max_events_per_wallet() -> usize {
    100
}
fn default_ttl_seconds() -> u64 {
    86400 // 24 hours
}

fn default_upstream_smart_max_slot_lag() -> u64 {
    20
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            bind_address: default_bind_address(),
        }
    }
}

impl Default for UpstreamConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            endpoint: Vec::new(),
            strategy: None,
            smart_max_slot_lag: default_upstream_smart_max_slot_lag(),
            max_concurrent: default_max_concurrent(),
            delay_ms: default_delay_ms(),
            timeout_secs: default_timeout_secs(),
            max_retries: default_max_retries(),
            retry_base_delay_ms: default_retry_base_delay_ms(),
            circuit_breaker_threshold: default_circuit_breaker_threshold(),
            circuit_breaker_timeout_secs: default_circuit_breaker_timeout_secs(),
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            network: default_network(),
            prefetch_alts: Vec::new(), // Empty by default
            alt_cache_ttl_secs: default_alt_cache_ttl(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            default_blocking_threshold: default_blocking_threshold(),
            rules_path: None,
            rule_action_override: None,
            blocked_programs: None,
            blocked_hashes: None,
            blocked_program_feeds: None,
            feed_poll_interval_secs: default_feed_poll_interval(),
            allowed_methods: Vec::new(),
            blocked_methods: Vec::new(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            mode: "none".to_string(),
            api_keys: None,
            allowed_wallets: None,
        }
    }
}

impl Default for UsageConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_requests_per_month: default_requests_per_month(),
        }
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self { url: None }
    }
}

impl Default for WasmConfig {
    fn default() -> Self {
        Self {
            analyzers_path: Some("./analyzers".to_string()),
            analyzer_config: None,
        }
    }
}

impl Default for EscalationsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            approver_wallet: None,
        }
    }
}

impl Default for RuleFeedsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            poll_interval: default_poll_interval(),
            default_min_interval: default_min_interval(),
            sources: Vec::new(),
        }
    }
}

impl Default for ActivityFeedConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            min_risk_score: default_min_risk_score(),
            max_events_per_wallet: default_max_events_per_wallet(),
            ttl_seconds: default_ttl_seconds(),
        }
    }
}

impl UpstreamConfig {
    pub fn validate(&self) -> Result<()> {
        let has_url = !self.url.trim().is_empty();
        let has_ep = !self.endpoint.is_empty();
        if has_url && has_ep {
            anyhow::bail!("upstream: specify either `url` or `[[upstream.endpoint]]`, not both");
        }
        if !has_url && !has_ep {
            anyhow::bail!("upstream: set `url` or at least one `[[upstream.endpoint]]`");
        }
        Ok(())
    }

    /// First URL in priority order (for ProgramAnalyzer and similar).
    pub fn primary_url(&self) -> String {
        self.ordered_rpc_urls()
            .into_iter()
            .next()
            .unwrap_or_default()
    }

    pub fn ordered_rpc_urls(&self) -> Vec<String> {
        if !self.endpoint.is_empty() {
            let mut eps = self.endpoint.clone();
            eps.sort_by(|a, b| a.priority.cmp(&b.priority));
            eps.into_iter().map(|e| e.url).collect()
        } else {
            vec![self.url.clone()]
                .into_iter()
                .filter(|u| !u.trim().is_empty())
                .collect()
        }
    }

    /// `(url, per-endpoint HTTP settings)` sorted by priority.
    pub fn ordered_upstream_http_settings(
        &self,
    ) -> Vec<(String, parapet_upstream::UpstreamHttpSettings)> {
        let base = parapet_upstream::UpstreamHttpSettings {
            max_concurrent: self.max_concurrent,
            delay_ms: self.delay_ms,
            timeout_secs: self.timeout_secs,
            max_retries: self.max_retries,
            retry_base_delay_ms: self.retry_base_delay_ms,
            circuit_breaker_threshold: self.circuit_breaker_threshold,
            circuit_breaker_timeout_secs: self.circuit_breaker_timeout_secs,
        };
        if !self.endpoint.is_empty() {
            let mut eps = self.endpoint.clone();
            eps.sort_by(|a, b| a.priority.cmp(&b.priority));
            eps.into_iter()
                .map(|e| {
                    let mut s = base.clone();
                    if let Some(v) = e.max_concurrent {
                        s.max_concurrent = v;
                    }
                    if let Some(v) = e.delay_ms {
                        s.delay_ms = v;
                    }
                    if let Some(v) = e.timeout_secs {
                        s.timeout_secs = v;
                    }
                    if let Some(v) = e.max_retries {
                        s.max_retries = v;
                    }
                    if let Some(v) = e.retry_base_delay_ms {
                        s.retry_base_delay_ms = v;
                    }
                    if let Some(v) = e.circuit_breaker_threshold {
                        s.circuit_breaker_threshold = v;
                    }
                    if let Some(v) = e.circuit_breaker_timeout_secs {
                        s.circuit_breaker_timeout_secs = v;
                    }
                    (e.url, s)
                })
                .collect()
        } else {
            vec![(self.url.clone(), base)]
        }
    }
}

impl Config {
    /// Load config from TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        config.upstream.validate()?;
        Ok(config)
    }

    /// Load with environment variable overrides
    pub fn from_file_with_env<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut config = Self::from_file(path)?;

        // Override with environment variables if present
        if let Ok(port) = std::env::var("PROXY_PORT") {
            if let Ok(p) = port.parse() {
                config.server.port = p;
            }
        }

        if let Ok(urls) = std::env::var("UPSTREAM_RPC_URLS") {
            config.upstream.url.clear();
            config.upstream.endpoint = parse_upstream_urls_list(&urls)
                .into_iter()
                .enumerate()
                .map(|(i, url)| UpstreamEndpoint {
                    url,
                    priority: i as u32,
                    max_concurrent: None,
                    delay_ms: None,
                    timeout_secs: None,
                    max_retries: None,
                    retry_base_delay_ms: None,
                    circuit_breaker_threshold: None,
                    circuit_breaker_timeout_secs: None,
                })
                .collect();
        } else if let Ok(url) = std::env::var("UPSTREAM_RPC_URL") {
            config.upstream.url = url;
            config.upstream.endpoint.clear();
        }

        if let Ok(allowed) = std::env::var("ALLOWED_RPC_METHODS") {
            config.security.allowed_methods = allowed
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        if let Ok(blocked) = std::env::var("BLOCKED_RPC_METHODS") {
            config.security.blocked_methods = blocked
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        if let Ok(redis_url) = std::env::var("REDIS_URL") {
            config.redis.url = Some(redis_url);
        }

        if let Ok(rules_path) = std::env::var("RULES_PATH") {
            config.security.rules_path = Some(rules_path);
        }

        config.upstream.validate()?;
        Ok(config)
    }

    /// Create from environment variables only (backwards compatible)
    pub fn from_env() -> Result<Self> {
        let config = Config {
            server: ServerConfig {
                port: std::env::var("PROXY_PORT")
                    .ok()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(8899),
                bind_address: std::env::var("BIND_ADDRESS")
                    .unwrap_or_else(|_| "0.0.0.0".to_string()),
            },
            upstream: {
                let mut u = UpstreamConfig {
                    url: String::new(),
                    endpoint: Vec::new(),
                    strategy: std::env::var("UPSTREAM_STRATEGY").ok(),
                    smart_max_slot_lag: std::env::var("UPSTREAM_SMART_MAX_SLOT_LAG")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or_else(default_upstream_smart_max_slot_lag),
                    max_concurrent: std::env::var("UPSTREAM_MAX_CONCURRENT")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(10),
                    delay_ms: std::env::var("UPSTREAM_DELAY_MS")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(100),
                    timeout_secs: std::env::var("UPSTREAM_TIMEOUT_SECS")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(30),
                    max_retries: std::env::var("UPSTREAM_MAX_RETRIES")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(3),
                    retry_base_delay_ms: std::env::var("UPSTREAM_RETRY_BASE_DELAY_MS")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(100),
                    circuit_breaker_threshold: std::env::var("UPSTREAM_CIRCUIT_BREAKER_THRESHOLD")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(5),
                    circuit_breaker_timeout_secs: std::env::var(
                        "UPSTREAM_CIRCUIT_BREAKER_TIMEOUT_SECS",
                    )
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(60),
                };
                if let Ok(urls) = std::env::var("UPSTREAM_RPC_URLS") {
                    u.endpoint = parse_upstream_urls_list(&urls)
                        .into_iter()
                        .enumerate()
                        .map(|(i, url)| UpstreamEndpoint {
                            url,
                            priority: i as u32,
                            max_concurrent: None,
                            delay_ms: None,
                            timeout_secs: None,
                            max_retries: None,
                            retry_base_delay_ms: None,
                            circuit_breaker_threshold: None,
                            circuit_breaker_timeout_secs: None,
                        })
                        .collect();
                } else {
                    u.url = std::env::var("UPSTREAM_RPC_URL")
                        .expect("UPSTREAM_RPC_URL or UPSTREAM_RPC_URLS must be set");
                }
                u
            },
            network: NetworkConfig {
                network: std::env::var("SOLANA_NETWORK")
                    .unwrap_or_else(|_| "mainnet-beta".to_string()),
                prefetch_alts: Vec::new(),
                alt_cache_ttl_secs: default_alt_cache_ttl(),
            },
            security: SecurityConfig {
                default_blocking_threshold: std::env::var("DEFAULT_BLOCKING_THRESHOLD")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(70),
                rules_path: std::env::var("RULES_PATH").ok(),
                rule_action_override: std::env::var("RULE_ACTION_OVERRIDE").ok(),
                blocked_programs: None,
                blocked_hashes: None,
                blocked_program_feeds: None,
                feed_poll_interval_secs: default_feed_poll_interval(),
                allowed_methods: std::env::var("ALLOWED_RPC_METHODS")
                    .ok()
                    .map(|s| {
                        s.split(',')
                            .map(|x| x.trim().to_string())
                            .filter(|x| !x.is_empty())
                            .collect()
                    })
                    .unwrap_or_default(),
                blocked_methods: std::env::var("BLOCKED_RPC_METHODS")
                    .ok()
                    .map(|s| {
                        s.split(',')
                            .map(|x| x.trim().to_string())
                            .filter(|x| !x.is_empty())
                            .collect()
                    })
                    .unwrap_or_default(),
            },
            auth: AuthConfig {
                mode: std::env::var("AUTH_MODE").unwrap_or_else(|_| "none".to_string()),
                api_keys: std::env::var("API_KEYS").ok(),
                allowed_wallets: std::env::var("ALLOWED_WALLETS")
                    .ok()
                    .map(|w| w.split(',').map(|s| s.trim().to_string()).collect()),
            },
            usage: UsageConfig {
                enabled: std::env::var("ENABLE_USAGE_TRACKING")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(false),
                default_requests_per_month: std::env::var("DEFAULT_REQUESTS_PER_MONTH")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(10_000),
            },
            redis: RedisConfig {
                url: std::env::var("REDIS_URL").ok(),
            },
            wasm: WasmConfig {
                analyzers_path: std::env::var("WASM_ANALYZERS_PATH").ok(),
                analyzer_config: std::env::var("WASM_ANALYZER_CONFIG").ok(),
            },
            escalations: EscalationsConfig {
                enabled: std::env::var("ENABLE_ESCALATIONS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(false),
                approver_wallet: std::env::var("ESCALATION_APPROVER_WALLET").ok(),
            },
            rule_feeds: RuleFeedsConfig {
                enabled: std::env::var("RULES_FEED_ENABLED")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(false),
                poll_interval: std::env::var("RULES_FEED_POLL_INTERVAL")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(3600),
                default_min_interval: std::env::var("RULES_FEED_MIN_INTERVAL")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(300),
                sources: parse_feed_sources_from_env(),
            },
            activity_feed: ActivityFeedConfig {
                enabled: std::env::var("ENABLE_ACTIVITY_FEED")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(false),
                min_risk_score: std::env::var("ACTIVITY_FEED_MIN_RISK_SCORE")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(40),
                max_events_per_wallet: std::env::var("ACTIVITY_FEED_MAX_EVENTS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(100),
                ttl_seconds: std::env::var("ACTIVITY_FEED_TTL_SECONDS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(86400),
            },
        };

        config.upstream.validate()?;
        Ok(config)
    }
}

/// Parse rule feed sources from RULES_FEED_URLS environment variable
/// Format: comma-separated URLs
/// Example: RULES_FEED_URLS=https://example.com/feed1.json,https://example.com/feed2.json
fn parse_feed_sources_from_env() -> Vec<FeedSourceConfig> {
    std::env::var("RULES_FEED_URLS")
        .ok()
        .map(|urls| {
            urls.split(',')
                .enumerate()
                .filter_map(|(idx, url)| {
                    let url = url.trim();
                    if url.is_empty() {
                        return None;
                    }
                    Some(FeedSourceConfig {
                        url: url.to_string(),
                        name: Some(format!("feed-{}", idx + 1)),
                        priority: 0,
                        min_interval: None,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}
