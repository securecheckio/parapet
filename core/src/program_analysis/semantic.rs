// Semantic analyzer - simplified version for tier 2 analysis
// Analyzes control flow, data flow, and syscalls

use anyhow::Result;
use log::info;
use serde::{Deserialize, Serialize};

use super::disassembler::DisassemblyResult;
use super::types::ProgramData;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticAnalysisResult {
    pub program_id: String,
    pub control_flow_complexity: f64,
    pub data_flow_risks: Vec<String>,
    pub syscall_patterns: Vec<String>,
    pub confidence_score: f64,
}

pub struct SemanticAnalyzer {}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self {}
    }

    /// Perform semantic analysis on program bytecode
    pub async fn analyze_program(
        &self,
        program_data: &ProgramData,
        disassembly: Option<&DisassemblyResult>,
    ) -> Result<SemanticAnalysisResult> {
        info!(
            "Starting semantic analysis for program: {}",
            program_data.address
        );

        let control_flow_complexity = self.analyze_control_flow(program_data, disassembly)?;
        let data_flow_risks = self.analyze_data_flow(program_data)?;
        let syscall_patterns = self.detect_syscall_patterns(program_data)?;

        // Calculate confidence based on analysis completeness
        let confidence = if disassembly.is_some() { 0.8 } else { 0.5 };

        Ok(SemanticAnalysisResult {
            program_id: program_data.address.to_string(),
            control_flow_complexity,
            data_flow_risks,
            syscall_patterns,
            confidence_score: confidence,
        })
    }

    fn analyze_control_flow(
        &self,
        _program_data: &ProgramData,
        disassembly: Option<&DisassemblyResult>,
    ) -> Result<f64> {
        if let Some(disasm) = disassembly {
            // Use complexity from disassembly
            Ok(disasm.complexity_score)
        } else {
            // Fallback: basic complexity estimation
            Ok(0.5)
        }
    }

    fn analyze_data_flow(&self, program_data: &ProgramData) -> Result<Vec<String>> {
        let mut risks = Vec::new();

        // Check for suspicious data patterns
        if program_data.is_upgradeable && program_data.authority.is_none() {
            risks.push("Upgradeable program with no authority set".to_string());
        }

        Ok(risks)
    }

    fn detect_syscall_patterns(&self, program_data: &ProgramData) -> Result<Vec<String>> {
        let mut patterns = Vec::new();

        // Detect common syscall signatures in bytecode
        let data = &program_data.executable_data;

        if Self::contains_pattern(data, b"sol_invoke") {
            patterns.push("Cross-program invocation detected".to_string());
        }

        if Self::contains_pattern(data, b"sol_log") {
            patterns.push("Logging syscall detected".to_string());
        }

        Ok(patterns)
    }

    fn contains_pattern(data: &[u8], pattern: &[u8]) -> bool {
        data.windows(pattern.len()).any(|window| window == pattern)
    }
}
