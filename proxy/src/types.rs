use crate::{auth, output, upstream, usage_tracker};
use parapet_core::rules;
use std::collections::HashSet;
use std::sync::Arc;

pub struct AppState {
    pub upstream_client: upstream::UpstreamClient,
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
}

#[derive(Clone)]
pub struct EscalationConfig {
    pub redis_url: String,
    pub approver_wallet: String,
}
