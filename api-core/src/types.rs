use serde::{Deserialize, Serialize};

// ============================================================================
// Rule Management Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRuleRequest {
    pub wallet: String,
    pub rule: DynamicRuleDefinition,
    pub signed_rule: String,  // Base64 encoded rule JSON
    pub signature: String,    // Base58 signature
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicRuleDefinition {
    pub id: String,
    pub name: String,
    pub priority: u32,
    pub conditions: serde_json::Value,
    pub action: String,
    pub recurring: bool,
    pub use_count_limit: Option<u32>,
    pub expires_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRulesRequest {
    pub wallet: String,
    pub signature: String,
    pub message: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRuleRequest {
    pub wallet: String,
    pub signature: String,
    pub message: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRulesRequest {
    pub wallet: String,
    pub signature: String,
    pub message: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRulesRequest {
    pub wallet: String,
    pub rules: Vec<DynamicRuleDefinition>,
    pub signature: String,
    pub message: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRuleResponse {
    pub rule_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRulesResponse {
    pub rules: Vec<DynamicRuleDefinition>,
}

// ============================================================================
// Authentication Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonceRequest {
    pub wallet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonceResponse {
    pub nonce: String,
    pub expires_at: u64,
}

// ============================================================================
// Escalation Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Escalation {
    pub escalation_id: String,
    pub canonical_hash: String,
    pub requester_wallet: String,
    pub approver_wallet: String,
    pub risk_score: u32,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedInstruction {
    pub instruction_type: String,
    pub human_readable: String,
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedRule {
    pub rule_type: String,
    pub name: String,
    pub description: String,
    pub conditions: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveEscalationRequest {
    pub approver_wallet: String,
    pub signature: String,
    pub message: String,
    pub nonce: String,
    pub timestamp: u64,
    pub rule: DynamicRuleDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DenyEscalationRequest {
    pub wallet: String,
    pub signature: String,
    pub message: String,
    pub nonce: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ApprovalResponse {
    TransactionForwarded {
        signature: String,
        fast_path: bool,
        message: String,
    },
    RuleCreated {
        rule_id: String,
        fast_path: bool,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationStatusResponse {
    pub status: EscalationStatus,
    pub rule_id: Option<String>,
    pub transaction_signature: Option<String>,
    pub fast_path: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPendingRequest {
    pub wallet: String,
    pub signature: String,
    pub message: String,
    pub timestamp: u64,
}

// ============================================================================
// WebSocket Event Types
// ============================================================================

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
        rule: DynamicRuleDefinition,
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
