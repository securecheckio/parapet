use super::SimulationAnalyzer;
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Analyzes failure states from simulation results
pub struct SimulationFailureAnalyzer;

impl SimulationFailureAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn extract_error_message(err: &Value) -> String {
        // Try to get error message from various formats
        if let Some(msg) = err.as_str() {
            return msg.to_string();
        }

        if let Some(obj) = err.as_object() {
            // Try common error message fields
            if let Some(msg) = obj.get("InstructionError").and_then(|v| v.to_string().into()) {
                return format!("InstructionError: {}", msg);
            }
            if let Some(msg) = obj.get("Custom").and_then(|v| v.as_u64()) {
                return format!("Custom error: {}", msg);
            }
        }

        format!("{:?}", err)
    }

    fn detect_partial_failure(logs: &[Value]) -> bool {
        // Check if some instructions succeeded and some failed
        let mut has_success = false;
        let mut has_failure = false;

        for log in logs {
            if let Some(log_str) = log.as_str() {
                if log_str.contains(" success") {
                    has_success = true;
                }
                if log_str.contains(" failed") || log_str.contains("Error:") {
                    has_failure = true;
                }
            }
        }

        has_success && has_failure
    }
}

#[async_trait::async_trait]
impl SimulationAnalyzer for SimulationFailureAnalyzer {
    fn name(&self) -> &str {
        "simulation_failure"
    }

    fn fields(&self) -> Vec<String> {
        vec![
            "simulation_failed".to_string(),
            "simulation_error".to_string(),
            "partial_failure".to_string(),
            "has_simulation_error".to_string(),
        ]
    }

    async fn analyze(&self, simulation_result: &Value) -> Result<HashMap<String, Value>> {
        let mut fields = HashMap::new();

        // Check for error field
        let err = simulation_result.get("err");
        let has_error = err.is_some() && !err.unwrap().is_null();

        let error_message = if has_error {
            Self::extract_error_message(err.unwrap())
        } else {
            String::new()
        };

        // Check for partial failures in logs
        let logs = simulation_result
            .get("logs")
            .and_then(|v| v.as_array())
            .map(|arr| arr.as_slice())
            .unwrap_or(&[]);

        let partial_failure = Self::detect_partial_failure(logs);

        fields.insert("simulation_failed".to_string(), json!(has_error));
        fields.insert("simulation_error".to_string(), json!(error_message));
        fields.insert("partial_failure".to_string(), json!(partial_failure));
        fields.insert("has_simulation_error".to_string(), json!(has_error));

        Ok(fields)
    }

    fn estimated_latency_ms(&self) -> u64 {
        1
    }
}

impl Default for SimulationFailureAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
