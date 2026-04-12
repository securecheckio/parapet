// Shared types for program analysis

use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

/// Complete program analysis result from any tier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramAnalysisResult {
    pub program_id: String,
    pub bytecode_hash: Option<String>,
    pub tier_used: String,

    // Risk assessment
    pub risk_score: f64,
    pub risk_level: RiskLevel,
    pub is_safe: bool,

    // Tier 1: Superficial
    pub helius_identity: Option<serde_json::Value>,
    pub ottersec_verified: bool,

    // Tier 2: Deep
    pub bytecode_analysis: Option<BytecodeAnalysis>,
    pub suspicious_patterns: Vec<String>,

    // Tier 3: AI
    pub ai_analysis: Option<AiAnalysis>,
    pub vulnerabilities: Vec<Vulnerability>,
    pub recommendations: Vec<String>,

    // Worker attribution
    pub worker_wallet_address: Option<String>,
    pub worker_signature: Option<String>,

    // Metadata
    pub cached: bool,
    pub analysis_time_ms: u64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    VeryLow,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BytecodeAnalysis {
    pub total_instructions: usize,
    pub suspicious_instruction_count: usize,
    pub control_flow_graph: Option<String>, // JSON representation
    pub complexity_score: f64,
    pub entropy_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAnalysis {
    pub model_used: String,
    pub behavioral_analysis: String,
    pub code_quality_assessment: String,
    pub confidence_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub severity: String,
    pub category: String,
    pub description: String,
    pub location: Option<String>,
}

/// Program data fetched from on-chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramData {
    pub address: Pubkey,
    pub executable_data: Vec<u8>,
    pub is_executable: bool,
    pub is_upgradeable: bool,
    pub authority: Option<Pubkey>,
    pub owner: Pubkey,
    pub lamports: u64,
}
