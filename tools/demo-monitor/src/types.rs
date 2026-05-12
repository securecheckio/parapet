use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemoStage {
    pub id: u32,
    pub name: String,
    pub date: String,
    pub description: String,
    pub tx_signature: Option<String>,
    pub expected_result: StageResult,
    pub talking_points: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StageResult {
    Pass,
    Alert,
    Block,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimulateRequest {
    pub stage_id: u32,
    pub transaction: Option<String>, // Optional: can use stage's default tx
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimulateResponse {
    pub stage: DemoStage,
    pub parapet_result: ParapetResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParapetResult {
    pub action: String, // "pass", "alert"
    pub risk_score: u32,
    pub rules_triggered: Vec<String>,
    pub message: String,
    pub analysis_time_ms: f64,
    #[serde(default)]
    pub analyzer_fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FireEventRequest {
    pub stage_id: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FireEventResponse {
    pub stage: DemoStage,
    pub baseline_result: BaselineResult,
    pub parapet_result: ParapetResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineResult {
    pub status: String,   // "executed", "signed"
    pub loss_amount: u64, // in millions
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorAlert {
    pub stage_id: u32,
    pub alert_type: String, // "attack_detected", "rapid_actions"
    pub message: String,
    pub risk_score: u32,
    pub timestamp: u64,
}
