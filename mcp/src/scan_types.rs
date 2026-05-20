//! Structured scan API types (REST + MCP JSON).

use crate::scan_coverage::{RulesSnapshot, ScanCoverage};
use parapet_core::rules::RuleAction;
use parapet_scanner::ScanReport;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionScanResult {
    pub scan_id: String,
    pub signature: String,
    pub decision: DecisionSummary,
    pub programs: Vec<String>,
    pub analysis_time_ms: u64,
    pub scan_coverage: ScanCoverage,
    pub rules_snapshot: RulesSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analyzer_fields: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionSummary {
    pub action: String,
    pub risk_score: u8,
    pub matched: bool,
    pub matched_rules: Vec<MatchedRuleSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedRuleSummary {
    pub rule_id: String,
    pub rule_name: String,
    pub action: String,
    pub weight: u8,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletScanResult {
    pub scan_id: String,
    pub report: ScanReport,
    pub scan_coverage: ScanCoverage,
    pub rules_snapshot: RulesSnapshot,
}

impl DecisionSummary {
    pub fn from_decision(decision: &parapet_core::rules::types::RuleDecision) -> Self {
        Self {
            action: decision.action.to_string(),
            risk_score: decision.total_risk,
            matched: decision.matched,
            matched_rules: decision
                .matched_rules
                .iter()
                .map(|m| MatchedRuleSummary {
                    rule_id: m.rule_id.clone(),
                    rule_name: m.rule_name.clone(),
                    action: m.action.to_string(),
                    weight: m.weight,
                    message: m.message.clone(),
                })
                .collect(),
        }
    }
}

pub fn action_str(a: RuleAction) -> &'static str {
    match a {
        RuleAction::Block => "block",
        RuleAction::Alert => "alert",
        RuleAction::Pass => "pass",
    }
}
