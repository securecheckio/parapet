use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleDefinition {
    pub version: String,
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub enabled: bool,

    #[serde(default)]
    pub tags: Vec<String>,

    pub rule: Rule,

    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub action: RuleAction,
    pub conditions: RuleCondition,
    pub message: String,

    #[serde(default)]
    #[serde(alias = "flowbits")]
    pub flowstate: Option<FlowStateActions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowStateActions {
    #[serde(default)]
    pub scope: FlowStateScope,

    #[serde(default)]
    pub set: Vec<String>,

    #[serde(default)]
    pub unset: Vec<String>,

    #[serde(default)]
    pub increment: Vec<String>,

    pub ttl_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FlowStateScope {
    #[serde(rename = "perwallet")]
    PerWallet,
    Global,
}

impl Default for FlowStateScope {
    fn default() -> Self {
        FlowStateScope::PerWallet
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    Block,
    Alert,
    Pass,
}

impl std::fmt::Display for RuleAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleAction::Block => write!(f, "block"),
            RuleAction::Alert => write!(f, "alert"),
            RuleAction::Pass => write!(f, "pass"),
        }
    }
}

impl std::str::FromStr for RuleAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "block" => Ok(RuleAction::Block),
            "alert" => Ok(RuleAction::Alert),
            "pass" => Ok(RuleAction::Pass),
            _ => Err(format!(
                "Invalid action: '{}'. Must be 'block', 'alert', or 'pass'",
                s
            )),
        }
    }
}

/// Configuration for overriding rule actions in bulk
#[derive(Debug, Clone)]
pub enum ActionOverride {
    /// Override all actions to the same action
    All(RuleAction),
    /// Override specific actions: original -> replacement
    Specific(HashMap<RuleAction, RuleAction>),
}

impl ActionOverride {
    /// Parse action override from environment variable string
    ///
    /// Formats:
    /// - "alert" -> Override all actions to alert
    /// - "block" -> Override all actions to block
    /// - "block:alert" -> Convert all "block" to "alert"
    /// - "block:alert,pass:alert" -> Convert "block" and "pass" to "alert"
    pub fn from_env_str(s: &str) -> Result<Self, String> {
        let s = s.trim();

        if s.is_empty() {
            return Err("Action override string cannot be empty".to_string());
        }

        // Check if it's a simple override (no colon)
        if !s.contains(':') {
            let action = s.parse::<RuleAction>()?;
            return Ok(ActionOverride::All(action));
        }

        // Parse specific overrides (format: "original:replacement,original:replacement")
        let mut map = HashMap::new();

        for pair in s.split(',') {
            let parts: Vec<&str> = pair.trim().split(':').collect();

            if parts.len() != 2 {
                return Err(format!(
                    "Invalid override pair: '{}'. Expected format: 'original:replacement'",
                    pair
                ));
            }

            let original = parts[0]
                .trim()
                .parse::<RuleAction>()
                .map_err(|e| format!("Invalid original action in '{}': {}", pair, e))?;
            let replacement = parts[1]
                .trim()
                .parse::<RuleAction>()
                .map_err(|e| format!("Invalid replacement action in '{}': {}", pair, e))?;

            map.insert(original, replacement);
        }

        if map.is_empty() {
            return Err("No valid action overrides found".to_string());
        }

        Ok(ActionOverride::Specific(map))
    }

    /// Apply the override to an action
    pub fn apply(&self, original: RuleAction) -> RuleAction {
        match self {
            ActionOverride::All(action) => *action,
            ActionOverride::Specific(map) => map.get(&original).copied().unwrap_or(original),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RuleCondition {
    Simple(SimpleCondition),
    FlowState(FlowStateCondition),
    Compound(CompoundCondition),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowStateCondition {
    #[serde(alias = "flowbit")]
    pub flowstate: String,

    #[serde(default)]
    pub within_seconds: Option<u64>,

    #[serde(default)]
    pub count_operator: Option<String>,

    #[serde(default)]
    pub count_value: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleCondition {
    pub field: String,
    pub operator: ComparisonOperator,
    #[serde(default)]
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompoundCondition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all: Option<Vec<RuleCondition>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub any: Option<Vec<RuleCondition>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub not: Option<Box<RuleCondition>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    In,
    NotIn,
    Contains,
    /// True when a flowstate flag is not set (used with `flowstate:` / `flowstate_global:` fields)
    #[serde(rename = "isnotset")]
    IsNotSet,
    /// True when the analyzer field is present (non-null) in the evaluated field map
    #[serde(rename = "exists")]
    Exists,
}

#[derive(Debug, Clone)]
pub struct RuleDecision {
    pub action: RuleAction,
    pub rule_id: String,
    pub rule_name: String,
    pub message: String,
    pub matched: bool,
    pub total_risk: u8,
    pub matched_rules: Vec<MatchedRule>,
    /// Risk score from structural analysis only (if applicable)
    pub structural_risk: Option<u8>,
    /// Risk score from simulation analysis only (if applicable)
    pub simulation_risk: Option<u8>,
    /// Whether this decision was made for a simulation
    pub is_simulation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedRule {
    pub rule_id: String,
    pub rule_name: String,
    pub action: RuleAction,
    pub weight: u8,
    pub message: String,
}

impl RuleDecision {
    pub fn no_match() -> Self {
        Self {
            action: RuleAction::Pass,
            rule_id: String::new(),
            rule_name: String::new(),
            message: String::new(),
            matched: false,
            total_risk: 0,
            matched_rules: Vec::new(),
            structural_risk: None,
            simulation_risk: None,
            is_simulation: false,
        }
    }
}
