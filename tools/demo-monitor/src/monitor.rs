use crate::types::{BaselineResult, MonitorAlert, ParapetResult};
use std::time::{SystemTime, UNIX_EPOCH};

/// Simulate baseline behavior (no Parapet protection)
pub fn get_baseline_result(stage_id: u32) -> BaselineResult {
    match stage_id {
        1 => BaselineResult {
            status: "Signed ✓".to_string(),
            loss_amount: 0,
        },
        2 => BaselineResult {
            status: "Executed ✓".to_string(),
            loss_amount: 0,
        },
        3 => BaselineResult {
            status: "Executed ✓".to_string(),
            loss_amount: 0,
        },
        _ => BaselineResult {
            status: "Unknown".to_string(),
            loss_amount: 0,
        },
    }
}

/// Generate monitor alert for WebSocket broadcast
pub fn generate_monitor_alert(stage_id: u32, parapet_result: &ParapetResult) -> MonitorAlert {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let (alert_type, message) = match stage_id {
        1 => (
            "nonce_creation_detected".to_string(),
            format!("Durable nonce creation detected. Security Council member creating delayed execution infrastructure. {} Review and verify authorization.", parapet_result.message),
        ),
        2 => (
            "attack_detected".to_string(),
            format!("ATTACK UNDERWAY: Admin transfer via durable nonce executing on-chain. {} Initiate incident response protocols immediately.", parapet_result.message),
        ),
        3 => (
            "coordinated_attack".to_string(),
            format!("CRITICAL: Coordinated attack confirmed. Sequential admin actions detected. {} Protocol compromise in progress.", parapet_result.message),
        ),
        _ => (
            "unknown".to_string(),
            "Unknown event".to_string(),
        ),
    };

    MonitorAlert {
        stage_id,
        alert_type,
        message,
        risk_score: parapet_result.risk_score,
        timestamp,
    }
}
