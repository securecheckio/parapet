use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Core transaction event for forensic audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionEvent {
    pub event_id: String,
    pub timestamp: DateTime<Utc>,

    // Attribution (who/what)
    pub user_id: Option<String>,
    pub identity: Option<String>,
    pub wallet: String,
    pub ip_address: Option<String>,
    pub tier: Option<String>,
    pub scopes: Vec<String>,

    // Intent (what they tried to do)
    pub method: String,
    pub expected_action: Option<String>,
    pub destination: Option<String>,
    pub programs: Vec<String>,
    pub program_names: Vec<String>,

    // Analysis (what we found)
    pub risk_score: u32,
    pub risk_level: RiskLevel,
    pub issues: Vec<String>,
    pub rule_matches: Vec<RuleMatch>,
    pub analyzer_fields: HashMap<String, serde_json::Value>,
    pub analyzers_used: Vec<String>,

    // Outcome (what happened)
    pub outcome: TransactionOutcome,
    pub signature: Option<String>,
    pub slot: Option<u64>,
    pub block_reason: Option<String>,

    // Point-in-time state (compliance proof)
    pub simulation_success: Option<bool>,
    pub simulation_error: Option<String>,
    pub compute_units: Option<u64>,
    pub rules_version: Option<String>,
    pub engine_version: String,

    // Human-readable translation
    pub summary: String,
    pub description: Option<String>,
    pub action_type: Option<String>,
    pub protocol: Option<String>,
    pub amount: Option<String>,
    pub tokens: Vec<String>,
    pub risk_explanation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionOutcome {
    Allowed,
    Blocked,
    Failed,
    RequiresApproval,
    Simulation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::Critical => "CRITICAL",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleMatch {
    pub rule_id: String,
    pub rule_name: String,
    pub action: String,
    pub reason: String,
    pub matched_fields: HashMap<String, String>,
}

impl TransactionEvent {
    pub fn new(wallet: String, method: String) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            user_id: None,
            identity: None,
            wallet,
            ip_address: None,
            tier: None,
            scopes: Vec::new(),
            method,
            expected_action: None,
            destination: None,
            programs: Vec::new(),
            program_names: Vec::new(),
            risk_score: 0,
            risk_level: RiskLevel::Low,
            issues: Vec::new(),
            rule_matches: Vec::new(),
            analyzer_fields: HashMap::new(),
            analyzers_used: Vec::new(),
            outcome: TransactionOutcome::Allowed,
            signature: None,
            slot: None,
            block_reason: None,
            simulation_success: None,
            simulation_error: None,
            compute_units: None,
            rules_version: None,
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            summary: String::new(),
            description: None,
            action_type: None,
            protocol: None,
            amount: None,
            tokens: Vec::new(),
            risk_explanation: None,
        }
    }
}
