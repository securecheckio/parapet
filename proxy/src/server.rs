use anyhow::Result;
use axum::{routing::post, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::{compression::CompressionLayer, cors::CorsLayer};

use crate::{auth, cache, output, rpc_handler, types::AppState, upstream, usage_tracker};
use parapet_core::rules;

/// Authentication mode for the RPC proxy
#[derive(Clone)]
pub enum AuthMode {
    /// No authentication (default, backwards compatible)
    None,
    /// Simple API key authentication from environment
    ApiKey,
    /// Wallet allowlist from environment
    WalletAllowlist,
    /// Custom auth provider
    Custom(Arc<dyn auth::AuthProvider>),
}

pub struct ServerConfig {
    pub port: u16,
    pub upstream_url: String,
    pub redis_url: Option<String>,
    pub bind_address: [u8; 4],

    /// Authentication mode
    pub auth_mode: AuthMode,

    /// Usage tracking (can be used alongside auth_mode)
    pub enable_usage_tracking: bool,
    pub default_requests_per_month: u64,

    /// Wallet allowlist (can be used alongside auth_mode)
    pub allowed_wallets: Option<Vec<String>>,

    pub blocked_programs: Option<Vec<String>>,
    pub rules_path: Option<String>,
    pub rule_action_override: Option<String>,

    /// WASM analyzers directory (optional)
    pub wasm_analyzers_path: Option<String>,

    /// Optional output manager for forensic audit trails
    /// If provided, this will be used instead of loading from environment
    pub output_manager: Option<Arc<output::OutputManager>>,

    /// Upstream rate limiting
    pub upstream_max_concurrent: usize,
    pub upstream_delay_ms: u64,
    pub upstream_timeout_secs: Option<u64>,
    pub upstream_max_retries: Option<usize>,
    pub upstream_retry_base_delay_ms: Option<u64>,
    pub upstream_circuit_breaker_threshold: Option<usize>,
    pub upstream_circuit_breaker_timeout_secs: Option<u64>,

    /// Default blocking threshold (0-100, default: 70)
    /// OSS: Used as the global threshold
    /// SaaS: Used as fallback when user hasn't set custom threshold
    pub default_blocking_threshold: u8,

    /// Enable escalations (requires redis_url and ESCALATION_APPROVER_WALLET env var)
    pub enable_escalations: bool,

    /// Automatic rule feed updates (community rules from multiple sources)
    pub rules_feed_enabled: bool,
    pub rules_feed_sources: Option<Vec<FeedSourceConfig>>,
    pub rules_feed_poll_interval: u64,
}

/// Feed source configuration from environment
#[derive(Debug, Clone)]
pub struct FeedSourceConfig {
    pub url: String,
    pub name: Option<String>,
    pub priority: u32,
    pub min_request_interval: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 8899,
            upstream_url: String::new(),
            redis_url: None,
            bind_address: [0, 0, 0, 0], // Bind to all interfaces by default
            auth_mode: AuthMode::None,  // No auth by default (backwards compatible)
            enable_usage_tracking: false,
            default_requests_per_month: 10_000,
            allowed_wallets: None,
            blocked_programs: None,
            rules_path: None,
            rule_action_override: None,
            wasm_analyzers_path: Some("./analyzers".to_string()),
            output_manager: None,
            upstream_max_concurrent: 10,
            upstream_delay_ms: 100,
            upstream_timeout_secs: Some(30),
            upstream_max_retries: Some(3),
            upstream_retry_base_delay_ms: Some(100),
            upstream_circuit_breaker_threshold: Some(5),
            upstream_circuit_breaker_timeout_secs: Some(60),
            default_blocking_threshold: 70,
            enable_escalations: false,
            rules_feed_enabled: false,
            rules_feed_sources: None,
            rules_feed_poll_interval: 3600,
        }
    }
}

/// Create router with given state (useful for tests and custom deployments)
#[allow(dead_code)]
pub fn create_router_with_state(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", post(rpc_handler::handle_rpc))
        .route("/health", axum::routing::get(health_check))
        .layer(CorsLayer::permissive())
        .layer(CompressionLayer::new())
        .with_state(state)
}

/// Build the Axum router for the RPC proxy (used by `start_server` and in-process tooling such as `rpc-perf`).
pub async fn build_app_router(config: ServerConfig) -> Result<Router> {
    // Initialize cache (side effect: connects Redis or allocates in-memory store)
    if let Some(redis_url) = &config.redis_url {
        log::info!("💾 Connecting to Redis: {}", redis_url);
        cache::Cache::new(redis_url).await?;
    } else {
        log::info!("💾 Using in-memory cache (no Redis configured)");
        cache::Cache::new_in_memory()?;
    };
    log::info!("✅ Cache initialized");

    // Initialize rule engine (always required)
    let rule_engine = initialize_rule_engine(
        config.rules_path.as_deref(),
        config.rule_action_override.as_deref(),
    )?;

    // Wrap rule engine in Arc<RwLock> for live updates
    let rule_engine = Arc::new(tokio::sync::RwLock::new(rule_engine));
    
    // Start automatic rule feed updater if enabled
    if config.rules_feed_enabled {
        if let Some(feed_sources) = config.rules_feed_sources {
            let feed_config = rules::FeedConfig {
                feed_sources: feed_sources
                    .into_iter()
                    .map(|src| rules::FeedSource {
                        url: src.url,
                        name: src.name,
                        priority: src.priority,
                        min_request_interval: src.min_request_interval,
                    })
                    .collect(),
                poll_interval: config.rules_feed_poll_interval,
                enabled: true,
            };
            
            let updater = rules::FeedUpdater::new(feed_config);
            let engine_clone = rule_engine.clone();
            
            updater.start_polling(move |merged| {
                let engine = engine_clone.clone();
                tokio::spawn(async move {
                    log::info!("🔄 Applying {} rule updates from {} sources", 
                        merged.rules.len(), merged.sources.len());
                    
                    let mut engine = engine.write().await;
                    
                    // Load new rules (merges with existing)
                    if let Err(e) = engine.load_rules(merged.rules) {
                        log::error!("Failed to load updated rules: {}", e);
                        return;
                    }
                    
                    // TODO: Remove deprecated rules
                    // engine.remove_rules(&merged.deprecated_rule_ids)?;
                    
                    log::info!("✅ Rule engine updated: {} total rules", engine.rule_count());
                });
                Ok(())
            }).await;
        } else {
            log::warn!("⚠️  RULES_FEED_ENABLED=true but no RULES_FEED_SOURCES provided");
        }
    }

    // Initialize upstream client
    let upstream_config = upstream::UpstreamConfig {
        max_concurrent: config.upstream_max_concurrent,
        delay_ms: config.upstream_delay_ms,
        timeout_secs: config.upstream_timeout_secs.unwrap_or(30),
        max_retries: config.upstream_max_retries.unwrap_or(3),
        retry_base_delay_ms: config.upstream_retry_base_delay_ms.unwrap_or(100),
        circuit_breaker_threshold: config.upstream_circuit_breaker_threshold.unwrap_or(5),
        circuit_breaker_timeout_secs: config.upstream_circuit_breaker_timeout_secs.unwrap_or(60),
    };

    let upstream_client = upstream::UpstreamClient::new_with_config(
        config.upstream_url.clone(),
        upstream_config,
    );
    log::info!("✅ Upstream client initialized");

    // Initialize usage tracker if enabled
    let usage_tracker = if config.enable_usage_tracking {
        if let Some(redis_url) = &config.redis_url {
            match usage_tracker::UsageTracker::new(redis_url, config.default_requests_per_month) {
                Ok(tracker) => {
                    log::info!(
                        "✅ Usage tracking enabled ({} requests/month per wallet)",
                        config.default_requests_per_month
                    );
                    Some(Arc::new(tracker))
                }
                Err(e) => {
                    log::error!("❌ Failed to initialize usage tracker: {}", e);
                    None
                }
            }
        } else {
            log::warn!("⚠️  Usage tracking requires Redis, disabling");
            None
        }
    } else {
        None
    };

    // NEW: Setup authentication provider
    let auth_provider: Option<Arc<dyn auth::AuthProvider>> = match config.auth_mode {
        AuthMode::None => {
            log::info!("🔓 Authentication: DISABLED");
            None
        }
        AuthMode::ApiKey => match auth::providers::ApiKeyAuth::from_env() {
            Ok(auth) => {
                let key_count = auth.key_count();
                log::info!(
                    "🔑 Authentication: API Keys ({} keys configured)",
                    key_count
                );
                Some(Arc::new(auth))
            }
            Err(e) => {
                log::error!("❌ Failed to setup API key auth: {}", e);
                None
            }
        },
        AuthMode::WalletAllowlist => match auth::providers::WalletAllowlist::from_env() {
            Ok(auth) => {
                let wallet_count = auth.wallet_count();
                log::info!(
                    "👛 Authentication: Wallet Allowlist ({} wallets)",
                    wallet_count
                );
                Some(Arc::new(auth))
            }
            Err(e) => {
                log::error!("❌ Failed to setup wallet allowlist auth: {}", e);
                None
            }
        },
        AuthMode::Custom(provider) => {
            log::info!("🔧 Authentication: Custom ({})", provider.name());
            Some(provider)
        }
    };

    // Process wallet allowlist
    let allowed_wallets = config.allowed_wallets.map(|wallets| {
        let wallet_set: std::collections::HashSet<String> = wallets.into_iter().collect();
        log::info!("✅ Wallet allowlist enabled ({} wallets)", wallet_set.len());
        wallet_set
    });

    if allowed_wallets.is_none() && auth_provider.is_none() {
        log::info!("ℹ️  No authentication or wallet allowlist (all requests allowed)");
    }

    // Initialize output manager for forensic audit trail
    let output_manager = if let Some(manager) = config.output_manager {
        // Use provided output manager
        let count = manager.pipeline_count();
        if count > 0 {
            log::info!("📊 Output manager: {} pipelines enabled (provided)", count);
            Some(manager)
        } else {
            log::info!("📊 Output manager: No pipelines (provided but empty)");
            None
        }
    } else {
        // Load from environment as fallback
        output::load_from_env()
            .ok()
            .and_then(|manager: crate::output::OutputManager| {
                let count = manager.pipeline_count();
                if count > 0 {
                    log::info!("📊 Output manager: {} pipelines enabled (from env)", count);
                    Some(Arc::new(manager))
                } else {
                    log::info!("📊 Output manager: No formatters configured");
                    None
                }
            })
    };

    // Initialize simulation analyzer registry
    let mut simulation_registry = parapet_core::rules::analyzers::simulation::SimulationAnalyzerRegistry::new();
    simulation_registry.register(Box::new(
        parapet_core::rules::analyzers::SimulationBalanceAnalyzer::new(),
    ));
    simulation_registry.register(Box::new(
        parapet_core::rules::analyzers::SimulationTokenAnalyzer::new(),
    ));
    simulation_registry.register(Box::new(
        parapet_core::rules::analyzers::SimulationLogAnalyzer::new(),
    ));
    simulation_registry.register(Box::new(
        parapet_core::rules::analyzers::SimulationCpiAnalyzer::new(),
    ));
    simulation_registry.register(Box::new(
        parapet_core::rules::analyzers::SimulationFailureAnalyzer::new(),
    ));
    simulation_registry.register(Box::new(
        parapet_core::rules::analyzers::SimulationComputeAnalyzer::new(),
    ));
    log::info!("✅ Simulation analyzers registered");

    // Initialize escalation config if enabled
    let escalation_config = if config.enable_escalations {
        if let Some(redis_url) = &config.redis_url {
            if let Ok(approver_wallet) = std::env::var("ESCALATION_APPROVER_WALLET") {
                log::info!("🚨 Escalations enabled (approver: {})", approver_wallet);
                Some(crate::types::EscalationConfig {
                    redis_url: redis_url.clone(),
                    approver_wallet,
                })
            } else {
                log::warn!("⚠️  Escalations enabled but ESCALATION_APPROVER_WALLET not set");
                None
            }
        } else {
            log::warn!("⚠️  Escalations enabled but Redis URL not configured");
            None
        }
    } else {
        None
    };

    // Create app state
    let state = Arc::new(AppState {
        upstream_client,
        rule_engine,
        auth_provider,
        usage_tracker,
        allowed_wallets,
        output_manager,
        default_blocking_threshold: config.default_blocking_threshold,
        simulation_registry: Arc::new(simulation_registry),
        escalation_config,
    });

    Ok(Router::new()
        .route("/", post(rpc_handler::handle_rpc))
        .route("/health", axum::routing::get(health_check))
        .layer(CorsLayer::permissive())
        .layer(CompressionLayer::new())
        .with_state(state))
}

pub async fn start_server(config: ServerConfig) -> Result<()> {
    log::info!("🚀 Starting Parapet RPC Proxy");
    log::info!("📡 Upstream RPC: {}", mask_api_key(&config.upstream_url));

    let port = config.port;
    let bind_address = config.bind_address;
    let addr = SocketAddr::from((bind_address, port));
    let app = build_app_router(config).await?;

    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log::info!("🎯 Parapet RPC Proxy Ready");
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log::info!("");
    log::info!("📍 Listening on:");

    log::info!("   Local:     http://localhost:{}", port);
    log::info!("   Loopback:  http://127.0.0.1:{}", port);

    if bind_address == [0, 0, 0, 0] {
        log::info!("   Network:   http://<YOUR_IP>:{}", port);
        log::info!("   (Accessible from your local network)");
    } else {
        log::info!("   Bind:      http://{}", addr);
    }

    log::info!("");
    log::info!("📋 Endpoints:");
    log::info!("   POST /        - JSON-RPC endpoint");
    log::info!("   GET  /health  - Health check");
    log::info!("");
    log::info!("✨ Ready to intercept and analyze transactions!");
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}

/// Load optional `analyzers.toml` (or `ANALYZERS_CONFIG_PATH`). The committed template is
/// `analyzers.toml.example` — copy to `analyzers.toml` to customize.
/// If no file is found: **register all analyzers** (empty `AnalyzersConfig` → every `name()` enabled).
fn load_analyzers_config() -> rules::AnalyzersConfig {
    use parapet_core::rules::AnalyzersConfig;
    use std::path::Path;

    if let Ok(p) = std::env::var("ANALYZERS_CONFIG_PATH") {
        if Path::new(&p).exists() {
            match AnalyzersConfig::from_file(&p) {
                Ok(c) => {
                    log::info!("📋 Loaded analyzer config from {} (ANALYZERS_CONFIG_PATH)", p);
                    return c;
                }
                Err(e) => log::warn!("⚠️ Failed to load ANALYZERS_CONFIG_PATH {}: {}", p, e),
            }
        } else {
            log::warn!("⚠️ ANALYZERS_CONFIG_PATH={} does not exist", p);
        }
    }

    for p in ["analyzers.toml", "proxy/analyzers.toml"] {
        if Path::new(p).exists() {
            match AnalyzersConfig::from_file(p) {
                Ok(c) => {
                    log::info!("📋 Loaded analyzer config from {}", p);
                    return c;
                }
                Err(e) => log::warn!("⚠️ Failed to load {}: {}", p, e),
            }
        }
    }

    log::info!(
        "📋 No analyzers.toml — registering all analyzers (copy analyzers.toml.example to customize)"
    );
    AnalyzersConfig::default()
}

fn mask_api_key(url: &str) -> String {
    if let Some(idx) = url.find("api-key=") {
        format!("{}api-key=***", &url[..idx])
    } else if let Some(idx) = url.rfind('/') {
        if idx > 0 && url[idx + 1..].len() > 10 {
            format!("{}/***", &url[..idx])
        } else {
            url.to_string()
        }
    } else {
        url.to_string()
    }
}

fn initialize_rule_engine(
    rules_path: Option<&str>,
    action_override_str: Option<&str>,
) -> Result<rules::RuleEngine> {
    use parapet_core::rules::analyzers::*;
    use parapet_core::rules::types::ActionOverride;

    let ac = load_analyzers_config();

    // Create analyzer registry
    let mut registry = rules::AnalyzerRegistry::new();

    // Register built-in analyzers
    if ac.should_register("basic") {
        registry.register(Arc::new(BasicAnalyzer::new()));
    }

    // Register core security analyzer
    if ac.should_register("core_security") {
        registry.register(Arc::new(CoreSecurityAnalyzer::new(
            std::collections::HashSet::new(),
        )));
    }

    // Register extended instruction analyzers (no external deps)
    if ac.should_register("token_instructions") {
        registry.register(Arc::new(TokenInstructionAnalyzer::new()));
    }
    if ac.should_register("system") {
        registry.register(Arc::new(SystemProgramAnalyzer::new()));
    }
    if ac.should_register("complexity") {
        registry.register(Arc::new(ProgramComplexityAnalyzer::new()));
    }
    if ac.should_register("logs") {
        registry.register(Arc::new(TransactionLogAnalyzer::new()));
    }

    // Register instruction padding analyzer (protection against padding attacks)
    if ac.should_register("padding") {
        registry.register(Arc::new(
            parapet_core::rules::analyzers::core::InstructionPaddingAnalyzer::new(),
        ));
    }

    // Register inner instruction analyzer (CPI analysis)
    if ac.should_register("inner_instruction") {
        registry.register(Arc::new(
            parapet_core::rules::analyzers::InnerInstructionAnalyzer::new(),
        ));
    }

    // Register instruction data fingerprint analyzer — loads from config file if present,
    // falls back to built-in authority-change defaults
    if ac.should_register("instruction_data") {
        // Derive fingerprint config path from rules_path (e.g. ./rules/presets/foo.json → ./rules/fingerprints/authority-change.json)
        let fingerprint_path = rules_path
            .and_then(|p| std::path::Path::new(p).parent())
            .and_then(|p| p.parent())
            .map(|base| base.join("fingerprints/authority-change.json"));

        let analyzer = match fingerprint_path.as_deref() {
            Some(path) if path.exists() => {
                match InstructionDataAnalyzer::from_config_file(path.to_str().unwrap_or("")) {
                    Ok(a) => {
                        log::info!("✅ Loaded instruction fingerprints from {}", path.display());
                        a
                    }
                    Err(e) => {
                        log::warn!("⚠️  Failed to load fingerprint config '{}': {} — using parapet-core embed", path.display(), e);
                        InstructionDataAnalyzer::with_authority_fingerprints_embedded()
                    }
                }
            }
            _ => {
                log::info!("ℹ️  No fingerprint override beside rules — using parapet-core authority-change.json");
                InstructionDataAnalyzer::with_authority_fingerprints_embedded()
            }
        };
        registry.register(Arc::new(analyzer));
    }

    // Register Helius analyzers (check HELIUS_API_KEY env var via should_register / requirements_met)
    if ac.should_register("helius_identity") {
        registry.register(Arc::new(HeliusIdentityAnalyzer::new()));
    }
    if ac.should_register("helius_transfer") {
        registry.register(Arc::new(HeliusTransferAnalyzer::new()));
    }
    if ac.should_register("helius_funding") {
        registry.register(Arc::new(HeliusFundingAnalyzer::new()));
    }

    // Register OtterSec Verified Analyzer (cryptographic source verification)
    if ac.should_register("ottersec") {
        registry.register(Arc::new(OtterSecVerifiedAnalyzer::new()));
    }

    // Rugcheck (optional enrichment API)
    if ac.should_register("rugcheck") {
        registry.register(Arc::new(RugcheckAnalyzer::new()));
    }

    // Register Jupiter Token Analyzer (token safety via Jupiter API)
    #[cfg(feature = "jupiter")]
    {
        if ac.should_register("jupiter") {
            use parapet_core::rules::analyzers::JupiterTokenAnalyzer;
            registry.register(Arc::new(JupiterTokenAnalyzer::new()));
        }
    }

    // Token Mint Analyzer requires RPC URL but not currently used in default config
    #[cfg(feature = "token-mint")]
    {
        if ac.should_register("token_mint") {
            let rpc_url = std::env::var("UPSTREAM_RPC_URL")
                .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
            registry.register(Arc::new(TokenMintAnalyzer::new(rpc_url)));
        }
    }

    // Load WASM analyzers from directory (if feature enabled and path configured)
    #[cfg(feature = "wasm-analyzers")]
    {
        if ac.should_register("wasm") {
            let wasm_config = parapet_core::rules::wasm_config::load_wasm_config_from_env();

            if let Some(wasm_path) = std::env::var("WASM_ANALYZERS_PATH").ok() {
                if wasm_path != "none" && wasm_path != "disabled" {
                    log::info!("📦 Loading WASM analyzers from: {}", wasm_path);
                    match parapet_core::rules::load_wasm_analyzers_from_dir(
                        &wasm_path,
                        wasm_config.clone(),
                    ) {
                        Ok(wasm_analyzers) => {
                            for analyzer in wasm_analyzers {
                                registry.register(analyzer);
                            }
                        }
                        Err(e) => {
                            log::warn!("⚠️ Failed to load WASM analyzers: {}", e);
                        }
                    }
                } else {
                    log::info!(
                        "📦 WASM analyzers disabled via WASM_ANALYZERS_PATH={}",
                        wasm_path
                    );
                }
            } else {
                // Try default ./analyzers directory
                let default_path = "./analyzers";
                if std::path::Path::new(default_path).exists() {
                    log::info!(
                        "📦 Loading WASM analyzers from default path: {}",
                        default_path
                    );
                    match parapet_core::rules::load_wasm_analyzers_from_dir(
                        default_path,
                        wasm_config.clone(),
                    ) {
                        Ok(wasm_analyzers) => {
                            if !wasm_analyzers.is_empty() {
                                log::info!(
                                    "📦 Loaded {} WASM analyzer(s) from default path",
                                    wasm_analyzers.len()
                                );
                                for analyzer in wasm_analyzers {
                                    registry.register(analyzer);
                                }
                            }
                        }
                        Err(e) => {
                            log::debug!("No WASM analyzers in default path: {}", e);
                        }
                    }
                }
            }
        }
    }

    #[cfg(not(feature = "wasm-analyzers"))]
    log::debug!("📦 WASM analyzers support not compiled (enable with --features wasm-analyzers)");

    // Log registered analyzers and their fields
    let all_fields = registry.get_all_fields();
    log::info!("📊 Registered {} analyzers:", all_fields.len());
    for (analyzer_name, fields) in &all_fields {
        log::info!("  • {} ({} fields)", analyzer_name, fields.len());
    }

    // Create rule engine
    let mut engine = rules::RuleEngine::new(registry);

    // Apply action override if configured
    if let Some(override_str) = action_override_str {
        match ActionOverride::from_env_str(override_str) {
            Ok(override_config) => {
                log::info!("🔄 Applying rule action override from environment");
                engine = engine.with_action_override(override_config);
            }
            Err(e) => {
                log::error!("❌ Invalid RULE_ACTION_OVERRIDE: {}", e);
                log::error!("❌ FATAL: Proxy cannot start with invalid action override");
                return Err(anyhow::anyhow!("Invalid RULE_ACTION_OVERRIDE: {}", e));
            }
        }
    }

    // Load rules from file if specified, otherwise use default protection rules
    let rules_file = rules_path.unwrap_or("./rules/presets/default-protection.json");

    engine.load_rules_from_file(rules_file)?;
    log::info!(
        "✅ Rule engine initialized with {} rules from {}",
        engine.enabled_rule_count(),
        rules_file
    );

    Ok(engine)
}
