use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Escalation {
    pub escalation_id: String,
    pub canonical_hash: String,
    pub requester_wallet: String,
    pub approver_wallet: String,
    pub risk_score: u8,
    pub warnings: Vec<String>,
    pub decoded_instructions: Vec<DecodedInstruction>,
    pub suggested_rules: Vec<SuggestedRule>,
    pub status: EscalationStatus,
    pub created_at: u64,
    pub expires_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscalationStatus {
    Pending,
    Approved,
    ApprovedFastPath,
    ApprovedSlowPath,
    Forwarded,
    Denied,
    Expired,
}

// Re-export from decoder module to avoid duplication
pub use super::decoder::DecodedInstruction;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedRule {
    pub rule_type: String,
    pub name: String,
    pub description: String,
    pub conditions: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EscalationEvent {
    #[serde(rename = "escalation_created")]
    Created { escalation: Escalation },

    #[serde(rename = "escalation_approved")]
    Approved {
        escalation_id: String,
        approved_by: String,
        approved_at: u64,
        rule: serde_json::Value,
    },

    #[serde(rename = "escalation_denied")]
    Denied {
        escalation_id: String,
        denied_by: String,
        denied_at: u64,
        reason: Option<String>,
    },

    #[serde(rename = "escalation_expired")]
    Expired {
        escalation_id: String,
        expired_at: u64,
    },

    #[serde(rename = "escalation_forwarded")]
    Forwarded {
        escalation_id: String,
        signature: String,
        forwarded_at: u64,
    },
}
