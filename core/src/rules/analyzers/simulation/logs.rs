use super::SimulationAnalyzer;
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Analyzes program logs from simulation results
pub struct SimulationLogAnalyzer;

impl SimulationLogAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Suspicious keywords that might indicate malicious behavior
    const SUSPICIOUS_KEYWORDS: &'static [&'static str] = &[
        "drain",
        "exploit",
        "steal",
        "hack",
        "rug",
        "scam",
        "malicious",
        "unauthorized",
        "backdoor",
        "honeypot",
    ];

    /// Warning keywords in program logs
    const WARNING_KEYWORDS: &'static [&'static str] = &[
        "warning",
        "warn",
        "error",
        "failed",
        "rejected",
        "invalid",
        "unauthorized",
        "insufficient",
    ];

    fn contains_suspicious_keywords(log: &str) -> bool {
        let log_lower = log.to_lowercase();
        Self::SUSPICIOUS_KEYWORDS
            .iter()
            .any(|&keyword| log_lower.contains(keyword))
    }

    fn contains_warning_keywords(log: &str) -> bool {
        let log_lower = log.to_lowercase();
        Self::WARNING_KEYWORDS
            .iter()
            .any(|&keyword| log_lower.contains(keyword))
    }

    fn extract_program_from_log(log: &str) -> Option<String> {
        // Parse log format: "Program PROGRAM_ID invoke [depth]"
        if log.starts_with("Program ") && log.contains(" invoke [") {
            let parts: Vec<&str> = log.split_whitespace().collect();
            if parts.len() >= 2 {
                return Some(parts[1].to_string());
            }
        }
        None
    }

    /// Extract instruction name from "Program log: Instruction: X"
    fn extract_instruction_name(log: &str) -> Option<String> {
        let prefix = "Program log: Instruction: ";
        log.strip_prefix(prefix)
            .map(|stripped| stripped.trim().to_string())
    }
}

#[async_trait::async_trait]
impl SimulationAnalyzer for SimulationLogAnalyzer {
    fn name(&self) -> &str {
        "simulation_logs"
    }

    fn fields(&self) -> Vec<String> {
        vec![
            "log_count".to_string(),
            "error_messages".to_string(),
            "suspicious_keywords".to_string(),
            "program_warnings".to_string(),
            "programs_logged".to_string(),
            "has_error_logs".to_string(),
            "instruction_names".to_string(),
            "program_invocations".to_string(),
        ]
    }

    async fn analyze(&self, simulation_result: &Value) -> Result<HashMap<String, Value>> {
        let mut fields = HashMap::new();

        // Extract logs array
        let logs = simulation_result.get("logs").and_then(|v| v.as_array());

        if let Some(logs) = logs {
            let mut error_messages = Vec::new();
            let mut program_warnings = Vec::new();
            let mut programs_logged = std::collections::HashSet::new();
            let mut instruction_names = Vec::new();
            let mut program_invocations: Vec<String> = Vec::new();
            let mut seen_programs = std::collections::HashSet::new();
            let mut has_suspicious = false;
            let mut has_errors = false;

            for log_entry in logs {
                if let Some(log_str) = log_entry.as_str() {
                    // Extract program ID from log
                    if let Some(program) = Self::extract_program_from_log(log_str) {
                        programs_logged.insert(program.clone());
                        if seen_programs.insert(program.clone()) {
                            program_invocations.push(program);
                        }
                    }

                    // Extract instruction names from "Program log: Instruction: X"
                    if let Some(name) = Self::extract_instruction_name(log_str) {
                        instruction_names.push(name);
                    }

                    // Check for suspicious keywords
                    if Self::contains_suspicious_keywords(log_str) {
                        has_suspicious = true;
                        error_messages.push(log_str.to_string());
                    }

                    // Check for warnings/errors
                    if Self::contains_warning_keywords(log_str) {
                        has_errors = true;
                        program_warnings.push(log_str.to_string());
                    }

                    // Check for explicit error logs (format: "Program log: Error: ...")
                    if log_str.contains("Program log: Error")
                        || log_str.contains("failed")
                        || log_str.contains("Error:")
                    {
                        has_errors = true;
                        error_messages.push(log_str.to_string());
                    }
                }
            }

            fields.insert("log_count".to_string(), json!(logs.len()));
            fields.insert("error_messages".to_string(), json!(error_messages));
            fields.insert("suspicious_keywords".to_string(), json!(has_suspicious));
            fields.insert("program_warnings".to_string(), json!(program_warnings));
            fields.insert("programs_logged".to_string(), json!(programs_logged.len()));
            fields.insert("has_error_logs".to_string(), json!(has_errors));
            fields.insert("instruction_names".to_string(), json!(instruction_names));
            fields.insert(
                "program_invocations".to_string(),
                json!(program_invocations),
            );
        } else {
            // No logs available
            fields.insert("log_count".to_string(), json!(0));
            fields.insert("error_messages".to_string(), json!(Vec::<String>::new()));
            fields.insert("suspicious_keywords".to_string(), json!(false));
            fields.insert("program_warnings".to_string(), json!(Vec::<String>::new()));
            fields.insert("programs_logged".to_string(), json!(0));
            fields.insert("has_error_logs".to_string(), json!(false));
            fields.insert("instruction_names".to_string(), json!(Vec::<String>::new()));
            fields.insert(
                "program_invocations".to_string(),
                json!(Vec::<String>::new()),
            );
        }

        Ok(fields)
    }

    fn estimated_latency_ms(&self) -> u64 {
        1
    }
}

impl Default for SimulationLogAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
