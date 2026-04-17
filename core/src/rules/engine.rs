use super::analyzer::AnalyzerRegistry;
use super::dynamic::DynamicRuleStore;
use super::flowstate::FlowStateManager;
use super::performance::PerformanceTracker;
use super::types::*;
use anyhow::{anyhow, Result};
use serde_json::Value;
use solana_sdk::transaction::{Transaction, VersionedTransaction};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[cfg(feature = "reqwest")]
use crate::enrichment::{EnrichmentData, EnrichmentService};

pub struct RuleEngine {
    rules: Vec<RuleDefinition>,
    registry: AnalyzerRegistry,
    /// Cached set of all fields required by active rules
    required_fields: std::collections::HashSet<String>,
    /// Optional action override for bulk rule modification
    action_override: Option<super::types::ActionOverride>,
    /// Current Solana network (mainnet-beta, devnet, testnet)
    current_network: String,
    /// Dynamic rules store (optional, for runtime rule injection)
    dynamic_rules: Option<Arc<DynamicRuleStore>>,
    /// Enrichment service for off-chain data (optional)
    #[cfg(feature = "reqwest")]
    enrichment: Option<Arc<EnrichmentService>>,
    /// Enrichment data cache for current transaction batch
    #[cfg(feature = "reqwest")]
    enrichment_cache: Arc<tokio::sync::RwLock<HashMap<String, EnrichmentData>>>,
    /// Performance tracker for rule and analyzer timing
    performance_tracker: PerformanceTracker,
    /// FlowState manager (optional, for multi-transaction attack detection)
    flowstate: Option<Arc<Mutex<FlowStateManager>>>,
}

/// Detect Solana network from RPC URL
fn detect_network_from_url(url: &str) -> String {
    let url_lower = url.to_lowercase();
    if url_lower.contains("devnet") {
        "devnet".to_string()
    } else if url_lower.contains("testnet") {
        "testnet".to_string()
    } else {
        "mainnet-beta".to_string()
    }
}

impl RuleEngine {
    pub fn new(registry: AnalyzerRegistry) -> Self {
        // Try to auto-detect from UPSTREAM_RPC_URL first, fall back to SOLANA_NETWORK env var
        let current_network = if let Ok(rpc_url) = std::env::var("UPSTREAM_RPC_URL") {
            let detected = detect_network_from_url(&rpc_url);
            // Allow explicit override if someone really wants it
            std::env::var("SOLANA_NETWORK").unwrap_or(detected)
        } else {
            std::env::var("SOLANA_NETWORK").unwrap_or_else(|_| "mainnet-beta".to_string())
        };

        // Check if performance tracking is enabled via env var
        let perf_enabled = std::env::var("RULE_ENGINE_PERFORMANCE_TRACKING")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);

        if perf_enabled {
            log::info!("📊 Performance tracking enabled for rules and analyzers");
        }

        log::info!(
            "🌐 Network: {} (rules will be filtered by network)",
            current_network
        );

        let mut engine = Self {
            rules: Vec::new(),
            registry,
            required_fields: std::collections::HashSet::new(),
            action_override: None,
            current_network,
            dynamic_rules: None,
            #[cfg(feature = "reqwest")]
            enrichment: None,
            #[cfg(feature = "reqwest")]
            enrichment_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            performance_tracker: PerformanceTracker::new(perf_enabled),
            flowstate: None, // DISABLED by default
        };

        // Auto-enable from environment variable
        if std::env::var("PARAPET_FLOWSTATE_ENABLED")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false)
        {
            let max_wallets = std::env::var("PARAPET_FLOWSTATE_MAX_WALLETS")
                .ok()
                .and_then(|v| v.parse().ok());
            engine = engine.with_flowstate(max_wallets);
        }

        engine
    }

    /// Enable flowstate for multi-transaction attack detection
    pub fn with_flowstate(mut self, max_wallets: Option<usize>) -> Self {
        log::info!("✅ RuleEngine: FlowState enabled - multi-transaction attack detection active");
        if let Some(limit) = max_wallets {
            log::info!("   Memory limit: {} wallets max", limit);
        }
        self.flowstate = Some(Arc::new(Mutex::new(FlowStateManager::new(max_wallets))));
        self
    }

    /// Check if flowstate is enabled
    pub fn has_flowstate(&self) -> bool {
        self.flowstate.is_some()
    }

    /// Enable enrichment service for off-chain data access in rules
    #[cfg(feature = "reqwest")]
    pub fn with_enrichment(mut self, service: Arc<EnrichmentService>) -> Self {
        log::info!("✅ RuleEngine: Enrichment service enabled - rules can access off-chain data");
        self.enrichment = Some(service);
        self
    }

    /// Set enrichment cache for current transaction batch (called by scanner)
    #[cfg(feature = "reqwest")]
    pub async fn set_enrichment_cache(&self, cache: HashMap<String, EnrichmentData>) {
        let cache_size = cache.len();
        let mut write_lock = self.enrichment_cache.write().await;
        *write_lock = cache;
        log::debug!("✅ Enrichment cache populated with {} entries", cache_size);
    }

    /// Clear enrichment cache (call after batch processing)
    #[cfg(feature = "reqwest")]
    pub async fn clear_enrichment_cache(&self) {
        let mut write_lock = self.enrichment_cache.write().await;
        write_lock.clear();
    }

    /// Enable dynamic rules with optional Redis URL for multi-instance sync
    pub fn with_dynamic_rules(mut self, redis_url: Option<String>) -> Self {
        self.dynamic_rules = Some(Arc::new(DynamicRuleStore::new(redis_url)));
        log::info!("✅ Dynamic rules enabled");
        self
    }

    /// Get dynamic rules store (if enabled)
    pub fn dynamic_rules(&self) -> Option<Arc<DynamicRuleStore>> {
        self.dynamic_rules.clone()
    }

    /// Get performance tracker
    pub fn performance_tracker(&self) -> &PerformanceTracker {
        &self.performance_tracker
    }

    /// Get performance metrics snapshot
    pub async fn get_performance_metrics(&self) -> super::performance::EnginePerformanceMetrics {
        self.performance_tracker.get_metrics().await
    }

    /// Get formatted performance report
    pub async fn get_performance_report(&self) -> String {
        self.performance_tracker.get_report().await
    }

    /// Reset performance metrics
    pub async fn reset_performance_metrics(&self) {
        self.performance_tracker.reset().await
    }

    /// Check if a rule should be loaded based on network filtering
    fn is_rule_applicable(&self, rule: &RuleDefinition) -> bool {
        // Check if rule has network restriction in metadata
        if let Some(networks_value) = rule.metadata.get("networks") {
            // Support both single string and array of strings
            let applicable = if let Some(network_str) = networks_value.as_str() {
                // Single network string
                network_str == self.current_network || network_str == "all"
            } else if let Some(network_array) = networks_value.as_array() {
                // Array of networks
                network_array.iter().any(|n| {
                    n.as_str()
                        .map_or(false, |s| s == self.current_network || s == "all")
                })
            } else {
                // Invalid format, skip network filtering
                true
            };

            if !applicable {
                log::debug!(
                    "  ⏭️  Skipping rule '{}': network '{}' not in {:?}",
                    rule.name,
                    self.current_network,
                    networks_value
                );
            }

            applicable
        } else {
            // No network restriction - defaults to mainnet-only for safety
            // Rules without network metadata are assumed to be mainnet rules
            let is_mainnet =
                self.current_network == "mainnet-beta" || self.current_network == "mainnet";

            if !is_mainnet {
                log::debug!(
                    "  ⏭️  Skipping rule '{}': no network specified, defaulting to mainnet-only",
                    rule.name
                );
            }

            is_mainnet
        }
    }

    /// Set action override for bulk rule modification
    pub fn with_action_override(mut self, override_config: super::types::ActionOverride) -> Self {
        self.action_override = Some(override_config);
        self
    }

    /// Apply action override if configured
    fn apply_action_override(
        &self,
        rule_def: &RuleDefinition,
        original_action: super::types::RuleAction,
    ) -> super::types::RuleAction {
        match &self.action_override {
            None => original_action,
            Some(override_config) => {
                let new_action = override_config.apply(original_action);
                if new_action != original_action {
                    log::debug!(
                        "🔄 Overriding rule '{}' action: {} -> {}",
                        rule_def.id,
                        original_action,
                        new_action
                    );
                }
                new_action
            }
        }
    }

    pub fn load_rules(&mut self, mut rules: Vec<RuleDefinition>) -> Result<()> {
        log::info!("📜 Loading {} rules", rules.len());

        // Run pass (tracker) rules before block/alert so flowstate counters increment before
        // dependent rules read them. Within each tier, preserve input order via stable sort.
        rules.sort_by(|a, b| {
            let priority_a = match a.rule.action {
                super::types::RuleAction::Pass => 0,
                super::types::RuleAction::Alert => 1,
                super::types::RuleAction::Block => 2,
            };
            let priority_b = match b.rule.action {
                super::types::RuleAction::Pass => 0,
                super::types::RuleAction::Alert => 1,
                super::types::RuleAction::Block => 2,
            };
            priority_a.cmp(&priority_b)
        });

        log::debug!("✅ Rules sorted by priority (pass → alert → block)");

        // Log action override if configured
        if let Some(override_config) = &self.action_override {
            match override_config {
                super::types::ActionOverride::All(action) => {
                    log::warn!(
                        "⚠️  Action Override: ALL rule actions will be overridden to '{}'",
                        action
                    );
                }
                super::types::ActionOverride::Specific(map) => {
                    log::warn!("⚠️  Action Override: Specific actions will be overridden:");
                    for (original, replacement) in map {
                        log::warn!("    {} -> {}", original, replacement);
                    }
                }
            }
        }

        // Validate rules before loading
        let mut required_fields = std::collections::HashSet::new();

        for rule in &rules {
            if !rule.enabled {
                log::debug!("  ⏭️  Skipping disabled rule: {}", rule.name);
                continue;
            }

            // Check if rule applies to current network
            if !self.is_rule_applicable(rule) {
                continue;
            }

            // Validate that all fields referenced in the rule have corresponding analyzers
            if let Err(e) = self.validate_rule(rule) {
                log::error!("  ❌ Rule validation failed for '{}': {}", rule.name, e);
                return Err(anyhow!("Rule '{}' validation failed: {}", rule.name, e));
            }

            // Extract required fields from this rule
            self.extract_required_fields(&rule.rule.conditions, &mut required_fields);

            log::info!("  ✓ Loaded rule: {} ({})", rule.name, rule.id);
        }

        // Check for flowstate rules and warn if flowstate disabled
        let mut flowstate_rules_count = 0;
        for rule in &rules {
            if rule.enabled && self.rule_uses_flowstate(&rule.rule) {
                flowstate_rules_count += 1;
            }
        }

        if flowstate_rules_count > 0 && self.flowstate.is_none() {
            log::warn!(
                "⚠️  {} rules use flowstate but flowstate is DISABLED. \
                These rules will have reduced effectiveness. \
                Enable with PARAPET_FLOWSTATE_ENABLED=true",
                flowstate_rules_count
            );
        }

        self.rules = rules;
        self.required_fields = required_fields;

        log::info!(
            "📊 Rules require {} unique fields from analyzers",
            self.required_fields.len()
        );

        Ok(())
    }

    fn rule_uses_flowstate(&self, rule: &Rule) -> bool {
        rule.flowstate.is_some() || self.condition_uses_flowstate(&rule.conditions)
    }

    fn condition_uses_flowstate(&self, condition: &RuleCondition) -> bool {
        match condition {
            RuleCondition::FlowState(_) => true,
            RuleCondition::Compound(compound) => {
                compound
                    .all
                    .as_ref()
                    .map(|conds| conds.iter().any(|c| self.condition_uses_flowstate(c)))
                    .unwrap_or(false)
                    || compound
                        .any
                        .as_ref()
                        .map(|conds| conds.iter().any(|c| self.condition_uses_flowstate(c)))
                        .unwrap_or(false)
                    || compound
                        .not
                        .as_ref()
                        .map(|c| self.condition_uses_flowstate(c))
                        .unwrap_or(false)
            }
            _ => false,
        }
    }

    fn validate_rule(&self, rule: &RuleDefinition) -> Result<()> {
        let available_fields = self.get_available_fields();
        self.validate_condition(&rule.rule.conditions, &available_fields)
    }

    fn validate_condition(
        &self,
        condition: &RuleCondition,
        available_fields: &std::collections::HashSet<String>,
    ) -> Result<()> {
        match condition {
            RuleCondition::Simple(simple) => {
                // FlowState counter/flag fields are engine-managed, not analyzer outputs
                if simple.field.starts_with("flowstate:")
                    || simple.field.starts_with("flowstate_global:")
                {
                    self.validate_operator_value_compatibility(&simple.operator, &simple.value)?;
                    return Ok(());
                }
                // Check if field exists in any analyzer
                if !available_fields.contains(&simple.field) {
                    return Err(anyhow!(
                        "Field '{}' not provided by any registered analyzer. Available fields: {}",
                        simple.field,
                        available_fields
                            .iter()
                            .take(10)
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }

                // Validate operator/value type compatibility
                self.validate_operator_value_compatibility(&simple.operator, &simple.value)?;

                Ok(())
            }
            RuleCondition::FlowState(_) => {
                // FlowState conditions are always valid (they don't depend on analyzers)
                Ok(())
            }
            RuleCondition::Compound(compound) => {
                // Validate compound condition has at least one condition
                let has_conditions =
                    compound.all.is_some() || compound.any.is_some() || compound.not.is_some();
                if !has_conditions {
                    return Err(anyhow!(
                        "Compound condition must have at least one of: 'all', 'any', or 'not'"
                    ));
                }

                if let Some(all_conditions) = &compound.all {
                    if all_conditions.is_empty() {
                        return Err(anyhow!("'all' condition cannot be empty"));
                    }
                    for cond in all_conditions {
                        self.validate_condition(cond, available_fields)?;
                    }
                }
                if let Some(any_conditions) = &compound.any {
                    if any_conditions.is_empty() {
                        return Err(anyhow!("'any' condition cannot be empty"));
                    }
                    for cond in any_conditions {
                        self.validate_condition(cond, available_fields)?;
                    }
                }
                if let Some(not_condition) = &compound.not {
                    self.validate_condition(not_condition, available_fields)?;
                }
                Ok(())
            }
        }
    }

    fn validate_operator_value_compatibility(
        &self,
        operator: &ComparisonOperator,
        value: &Value,
    ) -> Result<()> {
        use ComparisonOperator::*;

        match operator {
            ComparisonOperator::IsNotSet | ComparisonOperator::Exists => Ok(()),
            GreaterThan | LessThan | GreaterThanOrEqual | LessThanOrEqual => {
                // Numeric operators require numeric values
                match value {
                    Value::Number(_) => Ok(()),
                    Value::String(s) => {
                        // Allow string numbers that can be parsed
                        s.parse::<f64>()
                            .map(|_| ())
                            .map_err(|_| anyhow!("Operator '{:?}' requires a numeric value, got unparseable string: '{}'", operator, s))
                    }
                    _ => Err(anyhow!(
                        "Operator '{:?}' requires a numeric value, got {:?}",
                        operator,
                        value
                    )),
                }
            }
            In | NotIn => {
                // In/NotIn operators require array values
                if !value.is_array() {
                    return Err(anyhow!(
                        "Operator '{:?}' requires an array value, got {:?}",
                        operator,
                        value
                    ));
                }
                Ok(())
            }
            Contains => {
                // Contains operator requires array or string haystack
                if !value.is_array() && !value.is_string() {
                    return Err(anyhow!(
                        "Operator 'contains' requires an array or string value, got {:?}",
                        value
                    ));
                }
                Ok(())
            }
            Equals | NotEquals => {
                // Equals/NotEquals work with any type
                Ok(())
            }
        }
    }

    fn get_available_fields(&self) -> std::collections::HashSet<String> {
        let mut fields = std::collections::HashSet::new();

        // Get fields from registry
        for analyzer_name in self.registry.list_all() {
            if let Some(analyzer) = self.registry.get(&analyzer_name) {
                // Add unprefixed fields
                for field in analyzer.fields() {
                    fields.insert(field.clone());
                    // Also add prefixed version
                    fields.insert(format!("{}:{}", analyzer_name, field));
                }
            }
        }

        fields
    }

    /// Extract all fields referenced in a condition tree
    fn extract_required_fields(
        &self,
        condition: &RuleCondition,
        fields: &mut std::collections::HashSet<String>,
    ) {
        match condition {
            RuleCondition::Simple(simple) => {
                fields.insert(simple.field.clone());
            }
            RuleCondition::FlowState(_) => {
                // FlowState conditions don't require analyzer fields
            }
            RuleCondition::Compound(compound) => {
                if let Some(all_conditions) = &compound.all {
                    for cond in all_conditions {
                        self.extract_required_fields(cond, fields);
                    }
                }
                if let Some(any_conditions) = &compound.any {
                    for cond in any_conditions {
                        self.extract_required_fields(cond, fields);
                    }
                }
                if let Some(not_condition) = &compound.not {
                    self.extract_required_fields(not_condition, fields);
                }
            }
        }
    }

    /// Get the set of analyzers needed to evaluate active rules
    pub fn get_required_analyzers(&self) -> Vec<String> {
        let mut required_analyzers = std::collections::HashSet::new();

        for field in &self.required_fields {
            if field.starts_with("flowstate:") || field.starts_with("flowstate_global:") {
                continue;
            }
            // Check if field is prefixed (analyzer:field)
            if let Some((analyzer_name, _)) = field.split_once(':') {
                required_analyzers.insert(analyzer_name.to_string());
            } else {
                // Unprefixed field - need to find which analyzer provides it
                for analyzer_name in self.registry.list_all() {
                    if let Some(analyzer) = self.registry.get(&analyzer_name) {
                        if analyzer.fields().contains(&field.to_string()) {
                            required_analyzers.insert(analyzer_name);
                            break; // Found it, move to next field
                        }
                    }
                }
            }
        }

        required_analyzers.into_iter().collect()
    }

    pub fn load_rules_from_json(&mut self, json: &str) -> Result<()> {
        let rule_def: RuleDefinition = serde_json::from_str(json)?;
        self.rules.push(rule_def);
        Ok(())
    }

    pub fn load_rules_from_file(&mut self, path: &str) -> Result<()> {
        let metadata = std::fs::metadata(path)?;

        // If it's a directory, load all JSON files from it
        if metadata.is_dir() {
            return self.load_rules_from_dir(path);
        }

        // Single file
        let content = std::fs::read_to_string(path)?;

        // Try to parse as single rule first
        if let Ok(rule) = serde_json::from_str::<RuleDefinition>(&content) {
            self.rules.push(rule);
            return Ok(());
        }

        // Try to parse as array of rules
        if let Ok(rules) = serde_json::from_str::<Vec<RuleDefinition>>(&content) {
            self.load_rules(rules)?;
            return Ok(());
        }

        Err(anyhow!("Failed to parse rules from file: {}", path))
    }

    pub fn load_rules_from_dir(&mut self, dir_path: &str) -> Result<()> {
        log::info!("📂 Loading all rules from directory: {}", dir_path);

        let mut loaded_count = 0;
        let entries = std::fs::read_dir(dir_path)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            // Only process .json files
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            match self.load_rules_from_file(path.to_str().unwrap()) {
                Ok(_) => {
                    loaded_count += 1;
                    log::debug!("  ✓ Loaded: {}", path.display());
                }
                Err(e) => {
                    log::warn!("  ⚠️  Failed to load {}: {}", path.display(), e);
                }
            }
        }

        if loaded_count == 0 {
            return Err(anyhow!(
                "No valid rule files found in directory: {}",
                dir_path
            ));
        }

        log::info!(
            "✅ Loaded rules from {} files in {}",
            loaded_count,
            dir_path
        );
        Ok(())
    }

    pub async fn evaluate(&self, tx: &Transaction) -> Result<RuleDecision> {
        self.evaluate_with_threshold(tx, 70).await
    }

    pub async fn evaluate_with_threshold(
        &self,
        tx: &Transaction,
        threshold: u8,
    ) -> Result<RuleDecision> {
        // STEP 1: Check dynamic rules first (highest priority)
        if let Some(dynamic_store) = &self.dynamic_rules {
            // Get canonical hash for matching
            let canonical_hash = if let Some(analyzer) = self.registry.get("canonical_tx") {
                let result = analyzer.analyze(tx).await?;
                result
                    .get("canonical_transaction_hash")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            } else {
                None
            };

            let dynamic_rules = dynamic_store
                .get_matching_rules(canonical_hash.as_deref())
                .await;

            if !dynamic_rules.is_empty() {
                log::debug!("🔍 Checking {} dynamic rules", dynamic_rules.len());

                for dyn_rule in &dynamic_rules {
                    if !dyn_rule.rule.enabled {
                        continue;
                    }

                    // Analyze transaction for dynamic rule evaluation
                    #[cfg(feature = "reqwest")]
                    let fields = {
                        let mut fields = self.registry.analyze_all(tx).await?;
                        self.add_enrichment_to_fields(&mut fields).await;
                        fields
                    };
                    #[cfg(not(feature = "reqwest"))]
                    let fields = self.registry.analyze_all(tx).await?;

                    let wallet = self.extract_wallet_from_fields(&fields);

                    if self
                        .evaluate_condition(&dyn_rule.rule.rule.conditions, &fields, &wallet)
                        .await?
                    {
                        let action = dyn_rule.rule.rule.action;

                        // Dynamic rule matched - allow transaction
                        if action == super::types::RuleAction::Pass {
                            log::info!(
                                "✅ Dynamic rule '{}' allowed transaction",
                                dyn_rule.rule.name
                            );

                            // Increment use count
                            if let Err(e) =
                                dynamic_store.increment_use_count(&dyn_rule.rule.id).await
                            {
                                log::warn!("Failed to increment use count: {}", e);
                            }

                            return Ok(RuleDecision {
                                action: super::types::RuleAction::Pass,
                                rule_id: dyn_rule.rule.id.clone(),
                                rule_name: dyn_rule.rule.name.clone(),
                                message: format!(
                                    "✓ Allowed by dynamic rule: {}",
                                    dyn_rule.rule.name
                                ),
                                matched: true,
                                total_risk: 0,
                                matched_rules: vec![super::types::MatchedRule {
                                    rule_id: dyn_rule.rule.id.clone(),
                                    rule_name: dyn_rule.rule.name.clone(),
                                    action,
                                    weight: 0,
                                    message: dyn_rule.rule.rule.message.clone(),
                                }],
                                structural_risk: None,
                                simulation_risk: None,
                                is_simulation: false,
                            });
                        }
                    }
                }
            }
        }

        // STEP 2: No dynamic rule matched - evaluate static security rules
        // Get list of required analyzers (lazy evaluation optimization)
        let required_analyzers = self.get_required_analyzers();

        log::debug!(
            "Running {} analyzers for rule evaluation (threshold: {})",
            required_analyzers.len(),
            threshold
        );

        // Analyze transaction with only required analyzers (parallel execution)
        #[cfg(feature = "reqwest")]
        let fields = {
            let mut fields = self
                .registry
                .analyze_selected(tx, &required_analyzers)
                .await?;
            self.add_enrichment_to_fields(&mut fields).await;
            fields
        };
        #[cfg(not(feature = "reqwest"))]
        let fields = self
            .registry
            .analyze_selected(tx, &required_analyzers)
            .await?;

        self.evaluate_fields(fields, threshold).await
    }

    /// Shared static rule evaluation loop over a pre-computed fields map.
    async fn evaluate_fields(
        &self,
        fields: HashMap<String, Value>,
        threshold: u8,
    ) -> Result<RuleDecision> {
        let mut total_risk = 0u8;
        let mut matched_alerts = Vec::new();

        // Extract wallet address for flowstate tracking
        let wallet = self.extract_wallet_from_fields(&fields);

        // Evaluate each static rule in order
        for rule_def in &self.rules {
            if !rule_def.enabled {
                continue;
            }

            // Check for missing field behavior override in rule metadata
            let missing_field_override = rule_def
                .metadata
                .get("missing_field_behavior")
                .and_then(|v| v.as_str());

            if self
                .evaluate_condition_with_override(
                    &rule_def.rule.conditions,
                    &fields,
                    missing_field_override,
                    &wallet,
                )
                .await?
            {
                // Apply flowstate actions if rule matched
                if let Some(ref flowstate) = rule_def.rule.flowstate {
                    self.apply_flowstate_actions(&wallet, flowstate, &fields)
                        .await;
                }
                let action = self.apply_action_override(rule_def, rule_def.rule.action);

                // SIGNATURE-BASED: action="block" → stop immediately
                if action == super::types::RuleAction::Block {
                    log::info!("🚨 Rule '{}' triggered immediate block", rule_def.name);

                    // Auto-increment blocked_transaction_count flowstate for tracking repeated blocks
                    if let Some(state) = &self.flowstate {
                        let mut state_lock = state.lock().await;
                        state_lock.increment(
                            &wallet,
                            "blocked_transaction_count",
                            Some(Duration::from_secs(3600)),
                        );
                    }
                    return Ok(RuleDecision {
                        action: super::types::RuleAction::Block,
                        rule_id: rule_def.id.clone(),
                        rule_name: rule_def.name.clone(),
                        message: rule_def.rule.message.clone(),
                        matched: true,
                        total_risk: 100,
                        matched_rules: vec![super::types::MatchedRule {
                            rule_id: rule_def.id.clone(),
                            rule_name: rule_def.name.clone(),
                            action,
                            weight: 100,
                            message: rule_def.rule.message.clone(),
                        }],
                        structural_risk: None,
                        simulation_risk: None,
                        is_simulation: false,
                    });
                }

                // HEURISTIC: action="alert" → accumulate weight
                if action == super::types::RuleAction::Alert {
                    let weight = rule_def
                        .metadata
                        .get("weight")
                        .and_then(|w| w.as_u64())
                        .unwrap_or(20) as u8;

                    total_risk = total_risk.saturating_add(weight);

                    matched_alerts.push(super::types::MatchedRule {
                        rule_id: rule_def.id.clone(),
                        rule_name: rule_def.name.clone(),
                        action,
                        weight,
                        message: rule_def.rule.message.clone(),
                    });

                    log::debug!(
                        "⚠️  Alert: '{}' (+{}, total: {})",
                        rule_def.name,
                        weight,
                        total_risk
                    );

                    // EARLY STOP: if accumulated risk hits threshold, block immediately
                    if total_risk >= threshold {
                        log::info!(
                            "🚨 Risk threshold reached ({} >= {}), stopping evaluation",
                            total_risk,
                            threshold
                        );

                        // Auto-increment blocked_transaction_count flowstate for tracking repeated blocks
                        if let Some(state) = &self.flowstate {
                            let mut state_lock = state.lock().await;
                            state_lock.increment(
                                &wallet,
                                "blocked_transaction_count",
                                Some(Duration::from_secs(3600)),
                            );
                        }
                        return Ok(RuleDecision {
                            action: super::types::RuleAction::Block,
                            rule_id: "composite".to_string(),
                            rule_name: "Multiple Risk Factors".to_string(),
                            message: format!("🚨 Transaction blocked: {} risk factors detected ({}/100 risk weight, threshold: {}/100)", 
                                matched_alerts.len(), total_risk, threshold),
                            matched: true,
                            total_risk,
                            matched_rules: matched_alerts,
                            structural_risk: None,
                            simulation_risk: None,
                            is_simulation: false,
                        });
                    }
                }
            }
        }
        // Determine final action based on accumulated risk
        let (final_action, message) = if total_risk >= threshold {
            // Auto-increment blocked_transaction_count flowstate for tracking repeated blocks
            if let Some(state) = &self.flowstate {
                let mut state_lock = state.lock().await;
                state_lock.increment(
                    &wallet,
                    "blocked_transaction_count",
                    Some(Duration::from_secs(3600)),
                );
            }

            (
                super::types::RuleAction::Block,
                format!("🚨 Transaction blocked: {} risk factors detected ({}/100 risk weight, threshold: {}/100)", 
                    matched_alerts.len(), total_risk, threshold)
            )
        } else if total_risk > 0 {
            (
                super::types::RuleAction::Alert,
                format!(
                    "⚠️  {} suspicious patterns detected ({}/100 risk weight, threshold: {}/100)",
                    matched_alerts.len(),
                    total_risk,
                    threshold
                ),
            )
        } else {
            (
                super::types::RuleAction::Pass,
                "✓ No suspicious patterns detected".to_string(),
            )
        };

        Ok(RuleDecision {
            action: final_action,
            rule_id: if !matched_alerts.is_empty() {
                "composite".to_string()
            } else {
                String::new()
            },
            rule_name: if !matched_alerts.is_empty() {
                "Multiple Risk Factors".to_string()
            } else {
                String::new()
            },
            message,
            matched: !matched_alerts.is_empty() || total_risk > 0,
            total_risk,
            matched_rules: matched_alerts,
            structural_risk: None,
            simulation_risk: None,
            is_simulation: false,
        })
    }

    pub async fn evaluate_versioned(&self, tx: &VersionedTransaction) -> Result<RuleDecision> {
        self.evaluate_versioned_with_threshold(tx, 70).await
    }

    pub async fn evaluate_versioned_with_threshold(
        &self,
        tx: &VersionedTransaction,
        threshold: u8,
    ) -> Result<RuleDecision> {
        // Try to convert to legacy first - if possible, use regular evaluation
        if let Some(legacy_tx) = tx.clone().into_legacy_transaction() {
            return self.evaluate_with_threshold(&legacy_tx, threshold).await;
        }

        // For v0 transactions, we have limited analysis capability
        // For now, just pass them through with a warning in the logs
        log::warn!("⚠️  Limited rule evaluation for v0 transaction (ALTs not resolved)");
        Ok(RuleDecision::no_match())
    }

    /// Evaluate with confirmed transaction metadata (logs + inner/CPI instructions).
    pub async fn evaluate_with_metadata(
        &self,
        tx: &Transaction,
        metadata: &super::analyzer::ConfirmedTransactionMetadata,
    ) -> Result<RuleDecision> {
        self.evaluate_with_metadata_and_threshold(tx, metadata, 70)
            .await
    }

    pub async fn evaluate_with_metadata_and_threshold(
        &self,
        tx: &Transaction,
        metadata: &super::analyzer::ConfirmedTransactionMetadata,
        threshold: u8,
    ) -> Result<RuleDecision> {
        let required_analyzers = self.get_required_analyzers();

        #[cfg(feature = "reqwest")]
        let fields = {
            let mut fields = self
                .registry
                .analyze_selected_with_metadata(tx, &required_analyzers, metadata)
                .await?;
            self.add_enrichment_to_fields(&mut fields).await;
            fields
        };
        #[cfg(not(feature = "reqwest"))]
        let fields = self
            .registry
            .analyze_selected_with_metadata(tx, &required_analyzers, metadata)
            .await?;

        self.evaluate_fields(fields, threshold).await
    }

    /// Convenience wrapper — evaluate with logs only (no inner instructions).
    pub async fn evaluate_with_logs(
        &self,
        tx: &Transaction,
        logs: &[String],
    ) -> Result<RuleDecision> {
        self.evaluate_with_logs_and_threshold(tx, logs, 70).await
    }

    pub async fn evaluate_with_logs_and_threshold(
        &self,
        tx: &Transaction,
        logs: &[String],
        threshold: u8,
    ) -> Result<RuleDecision> {
        let metadata = super::analyzer::ConfirmedTransactionMetadata {
            logs: logs.to_vec(),
            inner_instructions: vec![],
        };
        self.evaluate_with_metadata_and_threshold(tx, &metadata, threshold)
            .await
    }

    async fn evaluate_condition(
        &self,
        condition: &RuleCondition,
        fields: &HashMap<String, Value>,
        wallet: &solana_sdk::pubkey::Pubkey,
    ) -> Result<bool> {
        self.evaluate_condition_with_override(condition, fields, None, wallet)
            .await
    }

    fn evaluate_condition_with_override<'a>(
        &'a self,
        condition: &'a RuleCondition,
        fields: &'a HashMap<String, Value>,
        missing_field_override: Option<&'a str>,
        wallet: &'a solana_sdk::pubkey::Pubkey,
    ) -> Pin<Box<dyn Future<Output = Result<bool>> + 'a + Send>> {
        Box::pin(async move {
            match condition {
                RuleCondition::Simple(simple) => Ok(self
                    .evaluate_simple_with_flowstate(simple, fields, missing_field_override, wallet)
                    .await?),
                RuleCondition::FlowState(flowstate) => {
                    self.evaluate_flowstate(flowstate, wallet).await
                }
                RuleCondition::Compound(compound) => {
                    self.evaluate_compound_with_override(
                        compound,
                        fields,
                        missing_field_override,
                        wallet,
                    )
                    .await
                }
            }
        })
    }

    async fn evaluate_flowstate(
        &self,
        condition: &FlowStateCondition,
        wallet: &solana_sdk::pubkey::Pubkey,
    ) -> Result<bool> {
        // GRACEFUL DEGRADATION: Return false if flowstate disabled
        if let Some(state) = &self.flowstate {
            let state_lock = state.lock().await;

            if let (Some(op_str), Some(val)) = (&condition.count_operator, condition.count_value) {
                // Counter comparison - parse operator string
                let operator = match op_str.as_str() {
                    "equals" | "==" => ComparisonOperator::Equals,
                    "not_equals" | "!=" => ComparisonOperator::NotEquals,
                    "greater_than" | ">" => ComparisonOperator::GreaterThan,
                    "less_than" | "<" => ComparisonOperator::LessThan,
                    "greater_than_or_equal" | ">=" => ComparisonOperator::GreaterThanOrEqual,
                    "less_than_or_equal" | "<=" => ComparisonOperator::LessThanOrEqual,
                    _ => return Err(anyhow!("Invalid flowstate counter operator: {}", op_str)),
                };

                let count = state_lock.get_counter(wallet, &condition.flowstate);
                self.compare(&Value::from(count), &operator, &Value::from(val))
            } else if let Some(within_secs) = condition.within_seconds {
                // Boolean check with time window
                Ok(state_lock.is_set_within(wallet, &condition.flowstate, within_secs))
            } else {
                // Boolean check (atomic - checks expiration only)
                Ok(state_lock.is_set(wallet, &condition.flowstate))
            }
        } else {
            log::debug!(
                "FlowState condition '{}' skipped (flowstate disabled)",
                condition.flowstate
            );
            Ok(false)
        }
    }

    async fn evaluate_simple_with_flowstate(
        &self,
        condition: &SimpleCondition,
        fields: &HashMap<String, Value>,
        missing_field_override: Option<&str>,
        wallet: &solana_sdk::pubkey::Pubkey,
    ) -> Result<bool> {
        if condition.field.starts_with("flowstate:") {
            let rest = &condition.field["flowstate:".len()..];
            return self
                .evaluate_flowstate_counter_field(false, rest, condition, fields, wallet)
                .await;
        }
        if condition.field.starts_with("flowstate_global:") {
            let rest = &condition.field["flowstate_global:".len()..];
            return self
                .evaluate_flowstate_counter_field(true, rest, condition, fields, wallet)
                .await;
        }
        self.evaluate_simple(condition, fields, missing_field_override)
    }

    /// Resolve `flowstate:` / `flowstate_global:` simple fields against flowstate (counters and flags).
    async fn evaluate_flowstate_counter_field(
        &self,
        global: bool,
        name_template: &str,
        condition: &SimpleCondition,
        fields: &HashMap<String, Value>,
        wallet: &solana_sdk::pubkey::Pubkey,
    ) -> Result<bool> {
        if condition.operator == ComparisonOperator::IsNotSet {
            let Some(state) = &self.flowstate else {
                return Ok(true);
            };
            let name = match self.interpolate_flowstate_name(name_template, fields) {
                Ok(Some(n)) => n,
                _ => return Ok(true),
            };
            let lock = state.lock().await;
            let is_set = if global {
                lock.is_set_global(&name)
            } else {
                lock.is_set(wallet, &name)
            };
            return Ok(!is_set);
        }

        let Some(state) = &self.flowstate else {
            let v = Value::from(0u64);
            return self.compare(&v, &condition.operator, &condition.value);
        };

        let name = match self.interpolate_flowstate_name(name_template, fields) {
            Ok(Some(n)) => n,
            _ => {
                let v = Value::Null;
                return self.compare(&v, &condition.operator, &condition.value);
            }
        };

        let lock = state.lock().await;
        let count = if global {
            lock.get_counter_global(&name)
        } else {
            lock.get_counter(wallet, &name)
        };
        let v = Value::from(count);
        self.compare(&v, &condition.operator, &condition.value)
    }

    fn evaluate_simple(
        &self,
        condition: &SimpleCondition,
        fields: &HashMap<String, Value>,
        missing_field_override: Option<&str>,
    ) -> Result<bool> {
        if condition.operator == ComparisonOperator::IsNotSet {
            let present = fields.get(&condition.field).filter(|v| !v.is_null());
            return Ok(present.is_none());
        }
        if condition.operator == ComparisonOperator::Exists {
            return Ok(fields
                .get(&condition.field)
                .map(|v| !v.is_null())
                .unwrap_or(false));
        }

        let field_value = fields.get(&condition.field);

        if field_value.is_none() {
            // Field doesn't exist - check for per-rule override first
            let missing_field_result = if let Some(behavior) = missing_field_override {
                match behavior {
                    "fail" | "false" => {
                        log::debug!(
                            "Field '{}' not found, override='fail' → false",
                            condition.field
                        );
                        false
                    }
                    "pass" | "true" => {
                        log::debug!(
                            "Field '{}' not found, override='pass' → true",
                            condition.field
                        );
                        true
                    }
                    "error" | "strict" => {
                        return Err(anyhow::anyhow!(
                            "Field '{}' required by rule but analyzer unavailable (strict mode)",
                            condition.field
                        ));
                    }
                    "auto" | _ => {
                        // Fall through to smart defaults
                        self.get_smart_default_for_missing_field(condition)?
                    }
                }
            } else {
                // No override - use smart defaults
                self.get_smart_default_for_missing_field(condition)?
            };

            return Ok(missing_field_result);
        }

        self.compare(field_value.unwrap(), &condition.operator, &condition.value)
    }

    fn get_smart_default_for_missing_field(&self, condition: &SimpleCondition) -> Result<bool> {
        // Smart defaults based on operator and expected value
        let result = match (&condition.operator, &condition.value) {
            // Checking if field equals empty/null → treat missing as empty (true)
            // Use case: "is this program unknown?" (no identity data = unknown = true)
            (ComparisonOperator::Equals, Value::Array(arr)) if arr.is_empty() => true,
            (ComparisonOperator::Equals, Value::String(s)) if s.is_empty() => true,
            (ComparisonOperator::Equals, Value::Null) => true,
            (ComparisonOperator::Equals, Value::Bool(false)) => true,

            // Checking if field contains something → missing can't contain (false)
            // Use case: "does identity contain 'scammer'?" (no data = not a known scammer)
            (ComparisonOperator::Contains, _) => false,

            // Checking if field is IN array → missing is not in anything (false)
            (ComparisonOperator::In, _) => false,

            // Checking if field NOT equals something → depends on value
            (ComparisonOperator::NotEquals, Value::Array(arr)) if arr.is_empty() => false,
            (ComparisonOperator::NotEquals, Value::Null) => false,
            (ComparisonOperator::NotEquals, _) => true,

            // All other cases → treat as null and let compare() handle it
            _ => {
                let null_value = Value::Null;
                return self.compare(&null_value, &condition.operator, &condition.value);
            }
        };

        log::debug!(
            "Field '{}' not found, operator {:?}, expected {:?} → {} (smart default)",
            condition.field,
            condition.operator,
            condition.value,
            result
        );

        Ok(result)
    }

    fn evaluate_compound_with_override<'a>(
        &'a self,
        compound: &'a CompoundCondition,
        fields: &'a HashMap<String, Value>,
        missing_field_override: Option<&'a str>,
        wallet: &'a solana_sdk::pubkey::Pubkey,
    ) -> Pin<Box<dyn Future<Output = Result<bool>> + 'a + Send>> {
        Box::pin(async move {
            if let Some(all_conditions) = &compound.all {
                for cond in all_conditions {
                    if !self
                        .evaluate_condition_with_override(
                            cond,
                            fields,
                            missing_field_override,
                            wallet,
                        )
                        .await?
                    {
                        return Ok(false);
                    }
                }
                return Ok(true);
            }

            if let Some(any_conditions) = &compound.any {
                for cond in any_conditions {
                    if self
                        .evaluate_condition_with_override(
                            cond,
                            fields,
                            missing_field_override,
                            wallet,
                        )
                        .await?
                    {
                        return Ok(true);
                    }
                }
                return Ok(false);
            }

            if let Some(not_condition) = &compound.not {
                return Ok(!self
                    .evaluate_condition_with_override(
                        not_condition,
                        fields,
                        missing_field_override,
                        wallet,
                    )
                    .await?);
            }

            Ok(false)
        })
    }

    fn compare(
        &self,
        actual: &Value,
        operator: &ComparisonOperator,
        expected: &Value,
    ) -> Result<bool> {
        match operator {
            ComparisonOperator::Equals => {
                // Special handling for null values to support graceful degradation
                // Treat null as equivalent to empty array/empty string/false for equals comparison
                if actual.is_null() {
                    match expected {
                        Value::Array(arr) => Ok(arr.is_empty()),
                        Value::String(s) => Ok(s.is_empty()),
                        Value::Bool(b) => Ok(!b), // null equals false
                        Value::Null => Ok(true),
                        _ => Ok(false),
                    }
                } else {
                    Ok(actual == expected)
                }
            }
            ComparisonOperator::NotEquals => {
                // Inverse of Equals logic
                Ok(!self.compare(actual, &ComparisonOperator::Equals, expected)?)
            }
            ComparisonOperator::GreaterThan => self.compare_numbers(actual, expected, |a, b| a > b),
            ComparisonOperator::LessThan => self.compare_numbers(actual, expected, |a, b| a < b),
            ComparisonOperator::GreaterThanOrEqual => {
                self.compare_numbers(actual, expected, |a, b| a >= b)
            }
            ComparisonOperator::LessThanOrEqual => {
                self.compare_numbers(actual, expected, |a, b| a <= b)
            }
            ComparisonOperator::In => self.in_array(actual, expected),
            ComparisonOperator::NotIn => Ok(!self.in_array(actual, expected)?),
            ComparisonOperator::Contains => self.contains(actual, expected),
            ComparisonOperator::IsNotSet => Ok(actual.is_null()),
            ComparisonOperator::Exists => Ok(!actual.is_null()),
        }
    }

    fn compare_numbers<F>(&self, actual: &Value, expected: &Value, comparator: F) -> Result<bool>
    where
        F: Fn(f64, f64) -> bool,
    {
        let a = self.to_number(actual)?;
        let b = self.to_number(expected)?;
        Ok(comparator(a, b))
    }

    fn to_number(&self, value: &Value) -> Result<f64> {
        match value {
            Value::Number(n) => n.as_f64().ok_or_else(|| anyhow!("Invalid number")),
            Value::String(s) => s.parse::<f64>().map_err(|e| anyhow!("Parse error: {}", e)),
            Value::Null => Ok(0.0), // Treat null as zero for numeric comparisons
            _ => Err(anyhow!("Value is not a number")),
        }
    }

    fn in_array(&self, needle: &Value, haystack: &Value) -> Result<bool> {
        match haystack {
            Value::Array(arr) => Ok(arr.contains(needle)),
            _ => Err(anyhow!("Expected array for 'in' operator")),
        }
    }

    fn contains(&self, haystack: &Value, needle: &Value) -> Result<bool> {
        match haystack {
            Value::Null => {
                // Null haystack contains nothing (graceful degradation)
                Ok(false)
            }
            Value::Array(arr) => Ok(arr.contains(needle)),
            Value::String(s) => {
                if let Value::String(n) = needle {
                    Ok(s.contains(n))
                } else {
                    Err(anyhow!("Expected string needle for contains"))
                }
            }
            _ => Err(anyhow!("Contains operator requires array or string")),
        }
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    pub fn enabled_rule_count(&self) -> usize {
        self.rules.iter().filter(|r| r.enabled).count()
    }

    /// Evaluate transaction with simulation data
    /// Combines structural analysis with simulation response analysis
    /// Access control rules (blocklist, etc.) are downgraded for simulations
    pub async fn evaluate_for_simulation(
        &self,
        tx: &VersionedTransaction,
        simulation_result: &Value,
        simulation_registry: &super::analyzers::simulation::SimulationAnalyzerRegistry,
        threshold: u8,
    ) -> Result<RuleDecision> {
        // Phase 1: Structural analysis
        let required_analyzers = self.get_required_analyzers();
        log::debug!(
            "Running {} structural analyzers for simulation",
            required_analyzers.len()
        );

        // Convert to legacy transaction if possible for analysis
        let structural_fields = if let Some(legacy_tx) = tx.clone().into_legacy_transaction() {
            self.registry
                .analyze_selected(&legacy_tx, &required_analyzers)
                .await?
        } else {
            log::warn!("⚠️  Limited structural analysis for v0 transaction");
            HashMap::new()
        };

        // Phase 2: Simulation analysis
        log::debug!("Running simulation analyzers");
        let simulation_fields = simulation_registry.analyze_all(simulation_result).await?;

        // Phase 3: Merge fields (simulation fields can override structural fields)
        let mut combined_fields = structural_fields.clone();
        combined_fields.extend(simulation_fields);

        // Phase 4: Evaluate rules with combined fields
        let mut total_risk = 0u8;
        let mut structural_risk_only = 0u8;
        let mut simulation_risk_only = 0u8;
        let mut matched_alerts = Vec::new();

        for rule_def in &self.rules {
            if !rule_def.enabled {
                continue;
            }

            // Check if rule should skip simulation (access control rules)
            let simulation_exempt = rule_def
                .metadata
                .get("simulation_exempt")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if simulation_exempt {
                log::debug!("  ⏭️  Skipping simulation-exempt rule: {}", rule_def.name);
                continue;
            }

            let missing_field_override = rule_def
                .metadata
                .get("missing_field_behavior")
                .and_then(|v| v.as_str());

            let wallet = self.extract_wallet_from_fields(&combined_fields);

            if self
                .evaluate_condition_with_override(
                    &rule_def.rule.conditions,
                    &combined_fields,
                    missing_field_override,
                    &wallet,
                )
                .await?
            {
                let mut action = self.apply_action_override(rule_def, rule_def.rule.action);

                // Downgrade BLOCK to ALERT for simulations (except access control)
                let is_access_control = rule_def
                    .tags
                    .iter()
                    .any(|tag| tag == "blocklist" || tag == "access-control");

                if action == super::types::RuleAction::Block && !is_access_control {
                    log::debug!(
                        "  🔽 Downgrading BLOCK -> ALERT for simulation: {}",
                        rule_def.name
                    );
                    action = super::types::RuleAction::Alert;
                }

                // Track risk by stage
                let weight = rule_def
                    .metadata
                    .get("weight")
                    .and_then(|w| w.as_u64())
                    .unwrap_or(20) as u8;

                let stage = rule_def
                    .metadata
                    .get("stage")
                    .and_then(|s| s.as_str())
                    .unwrap_or("structural");

                match stage {
                    "simulation" => {
                        simulation_risk_only = simulation_risk_only.saturating_add(weight)
                    }
                    _ => structural_risk_only = structural_risk_only.saturating_add(weight),
                }

                total_risk = total_risk.saturating_add(weight);

                matched_alerts.push(super::types::MatchedRule {
                    rule_id: rule_def.id.clone(),
                    rule_name: rule_def.name.clone(),
                    action,
                    weight,
                    message: rule_def.rule.message.clone(),
                });

                log::debug!(
                    "⚠️  Alert: '{}' (+{}, total: {}, stage: {})",
                    rule_def.name,
                    weight,
                    total_risk,
                    stage
                );
            }
        }

        // Determine final action based on accumulated risk
        let (final_action, message) = if total_risk >= threshold {
            (
                super::types::RuleAction::Alert, // Always ALERT for simulations (never block)
                format!(
                    "⚠️  This transaction WOULD BE BLOCKED if sent: {} risk factors detected ({}/100 risk, threshold: {}/100)",
                    matched_alerts.len(),
                    total_risk,
                    threshold
                ),
            )
        } else if total_risk > 0 {
            (
                super::types::RuleAction::Alert,
                format!(
                    "⚠️  {} suspicious patterns detected ({}/100 risk, threshold: {}/100)",
                    matched_alerts.len(),
                    total_risk,
                    threshold
                ),
            )
        } else {
            (
                super::types::RuleAction::Pass,
                "✓ No suspicious patterns detected".to_string(),
            )
        };

        Ok(RuleDecision {
            action: final_action,
            rule_id: if !matched_alerts.is_empty() {
                "composite".to_string()
            } else {
                String::new()
            },
            rule_name: if !matched_alerts.is_empty() {
                "Multiple Risk Factors".to_string()
            } else {
                String::new()
            },
            message,
            matched: !matched_alerts.is_empty() || total_risk > 0,
            total_risk,
            matched_rules: matched_alerts,
            structural_risk: Some(structural_risk_only),
            simulation_risk: Some(simulation_risk_only),
            is_simulation: true,
        })
    }

    /// Add enrichment data to fields for rule evaluation
    #[cfg(feature = "reqwest")]
    async fn add_enrichment_to_fields(&self, fields: &mut HashMap<String, Value>) {
        // Read from enrichment cache
        let cache = self.enrichment_cache.read().await;

        if cache.is_empty() {
            return;
        }

        // Extract token addresses from transaction data (if available)
        let mut token_addresses = Vec::new();

        // Look for token addresses in various fields
        if let Some(token_mint) = fields.get("token_mint").and_then(|v| v.as_str()) {
            token_addresses.push(token_mint.to_string());
        }
        if let Some(token_accounts) = fields.get("token_accounts").and_then(|v| v.as_array()) {
            for account in token_accounts {
                if let Some(mint) = account.as_str() {
                    token_addresses.push(mint.to_string());
                }
            }
        }

        // For each token found, add its enrichment data organized by source
        for token_addr in token_addresses {
            if let Some(enrichment) = cache.get(&token_addr) {
                // Create rugcheck namespace (includes all RugCheck API data)
                let mut rugcheck_json = serde_json::Map::new();

                // Add base Rugcheck score
                if let Some(ref rc) = enrichment.rugcheck {
                    rugcheck_json.insert("score".to_string(), Value::from(rc.score));
                    rugcheck_json
                        .insert("risk_level".to_string(), Value::from(rc.risk_level.clone()));
                    if let Some(mc) = rc.market_cap {
                        rugcheck_json.insert("market_cap".to_string(), Value::from(mc));
                    }
                    if let Some(liq) = rc.liquidity {
                        rugcheck_json.insert("liquidity".to_string(), Value::from(liq));
                    }
                }

                // Add insider analysis (from Rugcheck API)
                if let Some(ref insider) = enrichment.insider_analysis {
                    let mut insider_json = serde_json::Map::new();
                    insider_json.insert("risk_score".to_string(), Value::from(insider.risk_score));
                    insider_json.insert(
                        "risk_level".to_string(),
                        Value::from(insider.risk_level.clone()),
                    );
                    insider_json.insert(
                        "trade_networks".to_string(),
                        Value::from(insider.trade_networks),
                    );
                    insider_json.insert(
                        "transfer_networks".to_string(),
                        Value::from(insider.transfer_networks),
                    );
                    insider_json.insert(
                        "total_insiders".to_string(),
                        Value::from(insider.total_insiders),
                    );
                    insider_json.insert(
                        "insider_concentration".to_string(),
                        Value::from(insider.insider_concentration),
                    );
                    rugcheck_json
                        .insert("insider_analysis".to_string(), Value::Object(insider_json));
                }

                // Add vault analysis (from Rugcheck API)
                if let Some(ref vault) = enrichment.vault_analysis {
                    let mut vault_json = serde_json::Map::new();
                    vault_json.insert(
                        "has_locked_liquidity".to_string(),
                        Value::from(vault.has_locked_liquidity),
                    );
                    vault_json.insert(
                        "locked_percentage".to_string(),
                        Value::from(vault.locked_percentage),
                    );
                    vault_json.insert(
                        "rugpull_risk".to_string(),
                        Value::from(vault.rugpull_risk.clone()),
                    );
                    rugcheck_json.insert("vault_analysis".to_string(), Value::Object(vault_json));
                }

                // Add domain registration (from Rugcheck API)
                if let Some(ref domain) = enrichment.domain_registration {
                    let mut domain_json = serde_json::Map::new();
                    domain_json.insert("domain".to_string(), Value::from(domain.domain.clone()));
                    domain_json.insert("verified".to_string(), Value::from(domain.verified));
                    rugcheck_json.insert("domain".to_string(), Value::Object(domain_json));
                }

                // Add rugcheck namespace to fields
                if !rugcheck_json.is_empty() {
                    fields.insert("rugcheck".to_string(), Value::Object(rugcheck_json.clone()));
                    fields.insert(
                        format!("rugcheck_{}", token_addr),
                        Value::Object(rugcheck_json),
                    );
                }

                // Add Jupiter data if available
                if let Some(ref jupiter) = enrichment.jupiter {
                    let mut jupiter_json = serde_json::Map::new();
                    if let Some(price) = jupiter.price_usd {
                        jupiter_json.insert("price_usd".to_string(), Value::from(price));
                    }
                    if let Some(vol) = jupiter.volume_24h {
                        jupiter_json.insert("volume_24h".to_string(), Value::from(vol));
                    }
                    if let Some(score) = jupiter.organic_score {
                        jupiter_json.insert("organic_score".to_string(), Value::from(score));
                    }
                    jupiter_json.insert(
                        "has_rugpull_indicators".to_string(),
                        Value::from(jupiter.has_rugpull_indicators),
                    );

                    if !jupiter_json.is_empty() {
                        fields.insert("jupiter".to_string(), Value::Object(jupiter_json));
                    }
                }

                log::debug!(
                    "✅ Added rugcheck data for token {} to rule context",
                    token_addr
                );
            }
        }
    }

    /// Extract wallet address from fields (fee payer)
    fn extract_wallet_from_fields(
        &self,
        fields: &HashMap<String, Value>,
    ) -> solana_sdk::pubkey::Pubkey {
        // Try to get fee payer from basic:fee_payer field
        if let Some(Value::String(fee_payer)) = fields.get("basic:fee_payer") {
            if let Ok(pubkey) = fee_payer.parse() {
                return pubkey;
            }
        }

        // Fallback to default (should not happen in practice)
        solana_sdk::pubkey::Pubkey::default()
    }

    /// Interpolate variables in flowstate names
    /// Example: "transfers_to:{system:sol_recipients[0]}" + fields["system:sol_recipients"][0] -> "transfers_to:7xK...9mP"
    ///
    /// Format: {analyzer:field_name} or {analyzer:field_name[index]}
    /// - {system:sol_recipients[0]} -> first SOL recipient
    /// - {token_instructions:mints[0]} -> first token mint
    /// - {basic:fee_payer} -> transaction fee payer
    /// - {basic:instruction_count} -> number of instructions (numeric fields converted to string)
    fn interpolate_flowstate_name(
        &self,
        template: &str,
        fields: &HashMap<String, Value>,
    ) -> Result<Option<String>> {
        if !template.contains('{') {
            // No variables to interpolate
            return Ok(Some(template.to_string()));
        }

        let mut result = template.to_string();

        // Find all {variable} patterns
        let re = regex::Regex::new(r"\{([^}]+)\}").unwrap();

        for cap in re.captures_iter(template) {
            let var_spec = &cap[1];

            // Parse variable specification: "analyzer:field_name" or "analyzer:field_name[index]"
            if !var_spec.contains(':') {
                log::warn!("Invalid variable format '{}' - use 'analyzer:field_name' or 'analyzer:field_name[index]'", var_spec);
                return Ok(None);
            }

            let (field_path, array_index) = if let Some(bracket_pos) = var_spec.find('[') {
                // Array field with index: "system:sol_recipients[0]"
                let field = &var_spec[..bracket_pos];
                let index_str = &var_spec[bracket_pos + 1..var_spec.len() - 1];
                let index: usize = index_str.parse().unwrap_or(0);
                (field.to_string(), Some(index))
            } else {
                // Direct field: "basic:fee_payer"
                (var_spec.to_string(), None)
            };

            // Get the field value
            let value = if let Some(field_value) = fields.get(&field_path) {
                match field_value {
                    Value::Array(arr) => {
                        if arr.is_empty() {
                            // Array is empty - skip this flowstate operation
                            return Ok(None);
                        }
                        // Use specified index or default to 0
                        let idx = array_index.unwrap_or(0);
                        if let Some(Value::String(s)) = arr.get(idx) {
                            s.clone()
                        } else {
                            log::warn!(
                                "Array element at index {} is not a string for field {}",
                                idx,
                                field_path
                            );
                            return Ok(None);
                        }
                    }
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => {
                        log::warn!(
                            "Field {} has unsupported type for interpolation",
                            field_path
                        );
                        return Ok(None);
                    }
                }
            } else {
                log::warn!("Field {} not found for variable interpolation", field_path);
                return Ok(None);
            };

            // Replace the variable
            result = result.replace(&format!("{{{}}}", var_spec), &value);
        }

        Ok(Some(result))
    }

    /// Apply flowstate actions after a rule matches
    async fn apply_flowstate_actions(
        &self,
        wallet: &solana_sdk::pubkey::Pubkey,
        flowstate: &FlowStateActions,
        fields: &HashMap<String, Value>,
    ) {
        if let Some(state) = &self.flowstate {
            let mut state_lock = state.lock().await;

            // Convert ttl_seconds to Duration
            let ttl = flowstate.ttl_seconds.map(Duration::from_secs);

            for name_template in &flowstate.set {
                if let Ok(Some(name)) = self.interpolate_flowstate_name(name_template, fields) {
                    match flowstate.scope {
                        FlowStateScope::PerWallet => state_lock.set(wallet, &name, ttl),
                        FlowStateScope::Global => state_lock.set_global(&name, ttl),
                    }
                }
            }

            for name_template in &flowstate.increment {
                if let Ok(Some(name)) = self.interpolate_flowstate_name(name_template, fields) {
                    match flowstate.scope {
                        FlowStateScope::PerWallet => state_lock.increment(wallet, &name, ttl),
                        FlowStateScope::Global => state_lock.increment_global(&name, ttl),
                    }
                }
            }

            for name_template in &flowstate.unset {
                if let Ok(Some(name)) = self.interpolate_flowstate_name(name_template, fields) {
                    match flowstate.scope {
                        FlowStateScope::PerWallet => state_lock.unset(wallet, &name),
                        FlowStateScope::Global => state_lock.unset_global(&name),
                    }
                }
            }
        }
    }
}
