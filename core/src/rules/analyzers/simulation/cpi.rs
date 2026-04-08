use super::SimulationAnalyzer;
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Analyzes CPI (Cross-Program Invocation) calls from simulation results
pub struct SimulationCpiAnalyzer;

impl SimulationCpiAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn calculate_max_depth(inner_instructions: &[Value]) -> u32 {
        // Each inner instruction has an index and a list of instructions
        // Depth is determined by parsing the invoke/success log pairs
        // For simplicity, we'll count the number of nested inner instruction levels
        inner_instructions.len() as u32
    }

    fn extract_program_ids(inner_instructions: &[Value]) -> Vec<String> {
        let mut program_ids = Vec::new();

        for inner_inst in inner_instructions {
            if let Some(instructions) = inner_inst.get("instructions").and_then(|v| v.as_array()) {
                for inst in instructions {
                    // Each instruction has a programIdIndex
                    // We'd need the account keys to resolve this, but for now we'll note the presence
                    if let Some(program_idx) = inst.get("programIdIndex") {
                        program_ids.push(program_idx.to_string());
                    }
                }
            }
        }

        program_ids
    }
}

#[async_trait::async_trait]
impl SimulationAnalyzer for SimulationCpiAnalyzer {
    fn name(&self) -> &str {
        "simulation_cpi"
    }

    fn fields(&self) -> Vec<String> {
        vec![
            "has_cpi_calls".to_string(),
            "cpi_instruction_count".to_string(),
            "cpi_depth".to_string(),
            "cpi_program_indexes".to_string(),
        ]
    }

    async fn analyze(&self, simulation_result: &Value) -> Result<HashMap<String, Value>> {
        let mut fields = HashMap::new();

        // Extract innerInstructions array
        let inner_instructions = simulation_result
            .get("innerInstructions")
            .and_then(|v| v.as_array());

        if let Some(inner_insts) = inner_instructions {
            let has_cpi = !inner_insts.is_empty();
            let cpi_count = inner_insts.len() as u32;
            let max_depth = Self::calculate_max_depth(inner_insts);
            let program_indexes = Self::extract_program_ids(inner_insts);

            fields.insert("has_cpi_calls".to_string(), json!(has_cpi));
            fields.insert("cpi_instruction_count".to_string(), json!(cpi_count));
            fields.insert("cpi_depth".to_string(), json!(max_depth));
            fields.insert("cpi_program_indexes".to_string(), json!(program_indexes));
        } else {
            // No CPI calls
            fields.insert("has_cpi_calls".to_string(), json!(false));
            fields.insert("cpi_instruction_count".to_string(), json!(0));
            fields.insert("cpi_depth".to_string(), json!(0));
            fields.insert(
                "cpi_program_indexes".to_string(),
                json!(Vec::<String>::new()),
            );
        }

        Ok(fields)
    }

    fn estimated_latency_ms(&self) -> u64 {
        1
    }
}

impl Default for SimulationCpiAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
