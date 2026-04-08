use super::SimulationAnalyzer;
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Analyzes compute unit consumption from simulation results
pub struct SimulationComputeAnalyzer;

impl SimulationComputeAnalyzer {
    pub fn new() -> Self {
        Self
    }

    // Solana compute unit limits
    const DEFAULT_COMPUTE_LIMIT: u64 = 200_000;
    const MAX_COMPUTE_LIMIT: u64 = 1_400_000;

    /// Determine if compute usage is excessive for the instruction count
    /// Rule of thumb: ~5,000 CU per simple instruction is normal
    /// Complex instructions can use 50,000+ CU
    fn is_excessive_compute(units_consumed: u64, instruction_count: u64) -> bool {
        if instruction_count == 0 {
            return false;
        }

        let avg_per_instruction = units_consumed / instruction_count;
        // Flag if average is > 100,000 CU per instruction (very high)
        avg_per_instruction > 100_000
    }
}

#[async_trait::async_trait]
impl SimulationAnalyzer for SimulationComputeAnalyzer {
    fn name(&self) -> &str {
        "simulation_compute"
    }

    fn fields(&self) -> Vec<String> {
        vec![
            "compute_units_used".to_string(),
            "compute_usage_percent".to_string(),
            "excessive_compute".to_string(),
            "near_compute_limit".to_string(),
        ]
    }

    async fn analyze(&self, simulation_result: &Value) -> Result<HashMap<String, Value>> {
        let mut fields = HashMap::new();

        // Extract unitsConsumed
        let units_consumed = simulation_result
            .get("unitsConsumed")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        // Calculate percentage of default limit
        let usage_percent = (units_consumed as f64 / Self::DEFAULT_COMPUTE_LIMIT as f64) * 100.0;

        // Check if near the max limit (>90% of 1.4M)
        let near_limit = units_consumed > (Self::MAX_COMPUTE_LIMIT * 9 / 10);

        // Try to get instruction count from logs (count "invoke" entries)
        let instruction_count =
            if let Some(logs) = simulation_result.get("logs").and_then(|v| v.as_array()) {
                logs.iter()
                    .filter(|log| {
                        log.as_str()
                            .map(|s| s.contains(" invoke [1]"))
                            .unwrap_or(false)
                    })
                    .count() as u64
            } else {
                1 // Default to 1 if we can't determine
            };

        let excessive = Self::is_excessive_compute(units_consumed, instruction_count);

        fields.insert("compute_units_used".to_string(), json!(units_consumed));
        fields.insert("compute_usage_percent".to_string(), json!(usage_percent));
        fields.insert("excessive_compute".to_string(), json!(excessive));
        fields.insert("near_compute_limit".to_string(), json!(near_limit));

        Ok(fields)
    }

    fn estimated_latency_ms(&self) -> u64 {
        1
    }
}

impl Default for SimulationComputeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
