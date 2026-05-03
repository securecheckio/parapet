use crate::{auth, output, upstream, usage_tracker};
use parapet_core::rules;
use std::collections::HashSet;
use std::sync::Arc;

pub struct AppState {
    pub upstream_provider: Arc<dyn upstream::UpstreamProvider>,
    /// Rule engine wrapped in RwLock for live updates
    pub rule_engine: Arc<tokio::sync::RwLock<rules::RuleEngine>>,

    /// Optional authentication provider
    pub auth_provider: Option<Arc<dyn auth::AuthProvider>>,

    /// Usage tracker (can be used alongside or instead of auth_provider)
    pub usage_tracker: Option<Arc<usage_tracker::UsageTracker>>,

    /// Wallet allowlist (can be used alongside or instead of auth_provider)
    pub allowed_wallets: Option<HashSet<String>>,

    /// Output manager for forensic audit trail
    pub output_manager: Option<Arc<output::OutputManager>>,

    /// Default blocking threshold for OSS deployments (0-100)
    /// In SaaS mode, per-user thresholds from auth_context override this
    pub default_blocking_threshold: u8,

    /// Simulation analyzer registry for analyzing simulation responses
    pub simulation_registry: Arc<rules::analyzers::simulation::SimulationAnalyzerRegistry>,

    /// Escalation configuration (when Redis is enabled)
    pub escalation_config: Option<EscalationConfig>,

    /// Activity feed configuration (when Redis is enabled)
    pub activity_feed_config: Option<ActivityFeedConfig>,

    /// When non-empty, only these JSON-RPC methods are accepted.
    pub rpc_allowed_methods: Vec<String>,
    /// Methods always rejected (checked first).
    pub rpc_blocked_methods: Vec<String>,
}

impl AppState {
    pub fn is_method_allowed(&self, method: &str) -> bool {
        if self
            .rpc_blocked_methods
            .iter()
            .any(|m| m.as_str() == method)
        {
            return false;
        }
        if !self.rpc_allowed_methods.is_empty()
            && !self
                .rpc_allowed_methods
                .iter()
                .any(|m| m.as_str() == method)
        {
            return false;
        }
        true
    }
}

#[derive(Clone)]
pub struct EscalationConfig {
    pub redis_url: String,
    pub approver_wallet: String,
}

#[derive(Clone)]
pub struct ActivityFeedConfig {
    pub redis_url: String,
    pub min_risk_score: u8,
    pub max_events_per_wallet: usize,
    pub ttl_seconds: u64,
    pub network: String,
}
