// BPF disassembler - ported from txfilter
// Analyzes program bytecode for suspicious patterns

use anyhow::Result;
use goblin::elf::Elf;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisassemblyResult {
    pub total_instructions: usize,
    pub suspicious_instruction_count: usize,
    pub suspicious_patterns: Vec<String>,
    pub instruction_categories: HashMap<String, usize>,
    pub entropy_score: f64,
    pub complexity_score: f64,
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub address: u64,
    pub mnemonic: String,
    pub op_str: String,
    pub bytes: Vec<u8>,
}

pub struct ProgramDisassembler {
    // Simple BPF bytecode analyzer (without capstone for now)
}

impl ProgramDisassembler {
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }

    pub fn disassemble(&self, program_data: &[u8]) -> Result<DisassemblyResult> {
        info!("Starting program disassembly, data size: {} bytes", program_data.len());
        
        let instructions = self.analyze_bytecode(program_data)?;
        let entropy_score = self.calculate_entropy(program_data);
        let complexity_score = self.calculate_complexity(program_data);
        let suspicious_patterns = self.detect_suspicious_patterns(program_data);

        Ok(DisassemblyResult {
            total_instructions: instructions.len(),
            suspicious_instruction_count: suspicious_patterns.len(),
            suspicious_patterns,
            instruction_categories: HashMap::new(),
            entropy_score,
            complexity_score,
        })
    }

    fn analyze_bytecode(&self, data: &[u8]) -> Result<Vec<Instruction>> {
        let mut instructions = Vec::new();
        
        // Simplified bytecode analysis - look for common BPF instruction patterns
        let mut offset = 0;
        while offset + 8 <= data.len() {
            let instruction_bytes = &data[offset..offset + 8];
            
            // Basic BPF instruction analysis
            if let Some(instr) = self.parse_bpf_instruction(offset as u64, instruction_bytes) {
                instructions.push(instr);
            }
            
            offset += 8; // BPF instructions are 8 bytes
        }
        
        info!("Analyzed {} potential instructions", instructions.len());
        Ok(instructions)
    }

    fn parse_bpf_instruction(&self, address: u64, bytes: &[u8]) -> Option<Instruction> {
        if bytes.len() < 8 {
            return None;
        }

        // Basic BPF instruction decoding
        let opcode = bytes[0];
        let mnemonic = match opcode & 0x07 {
            0x00 => "load",
            0x01 => "loadx",
            0x02 => "store",
            0x03 => "storex",
            0x04 => "alu",
            0x05 => "jump",
            0x06 => "unknown",
            0x07 => "alu64",
            _ => "invalid",
        }.to_string();

        Some(Instruction {
            address,
            mnemonic,
            op_str: format!("0x{:02x}", opcode),
            bytes: bytes.to_vec(),
        })
    }

    fn calculate_entropy(&self, data: &[u8]) -> f64 {
        if data.is_empty() {
            return 0.0;
        }

        let mut freq = [0u32; 256];
        for &byte in data {
            freq[byte as usize] += 1;
        }

        let len = data.len() as f64;
        let mut entropy = 0.0;

        for &count in &freq {
            if count > 0 {
                let p = count as f64 / len;
                entropy -= p * p.log2();
            }
        }

        // Normalize to 0-1 range (max entropy for byte is 8 bits)
        entropy / 8.0
    }

    fn calculate_complexity(&self, data: &[u8]) -> f64 {
        if data.len() < 16 {
            return 0.0;
        }

        // Calculate complexity based on patterns and repetition
        let mut unique_patterns = std::collections::HashSet::new();
        let window_size = 4;

        for window in data.windows(window_size) {
            unique_patterns.insert(window.to_vec());
        }

        let complexity = unique_patterns.len() as f64 / (data.len() - window_size + 1) as f64;
        complexity.min(1.0)
    }

    fn detect_suspicious_patterns(&self, data: &[u8]) -> Vec<String> {
        let mut patterns = Vec::new();

        // Look for suspicious byte patterns
        let suspicious_sequences: &[(&str, &[u8])] = &[
            ("transfer", b"transfer"),
            ("create", b"create_account"),
            ("close", b"close_account"), 
            ("invoke", b"sol_invoke"),
            ("syscall", b"sol_log"),
        ];

        for (name, pattern) in suspicious_sequences {
            if self.contains_pattern(data, pattern) {
                patterns.push(format!("{} operation detected", name));
            }
        }

        // Check for high entropy regions (possible obfuscation)
        if self.calculate_entropy(data) > 0.9 {
            patterns.push("High entropy detected - possible obfuscation".to_string());
        }

        // Check for repeated patterns (possible packing)
        if self.has_repeated_patterns(data) {
            patterns.push("Repeated patterns detected - possible packing".to_string());
        }

        patterns
    }

    fn contains_pattern(&self, data: &[u8], pattern: &[u8]) -> bool {
        data.windows(pattern.len()).any(|window| window == pattern)
    }

    fn has_repeated_patterns(&self, data: &[u8]) -> bool {
        if data.len() < 32 {
            return false;
        }

        let chunk_size = 16;
        let mut pattern_counts = HashMap::new();

        for chunk in data.chunks(chunk_size) {
            if chunk.len() == chunk_size {
                *pattern_counts.entry(chunk.to_vec()).or_insert(0) += 1;
            }
        }

        pattern_counts.values().any(|&count| count > 3)
    }

    pub fn analyze_elf_structure(&self, data: &[u8]) -> Result<HashMap<String, String>> {
        let mut analysis = HashMap::new();

        match Elf::parse(data) {
            Ok(elf) => {
                analysis.insert("format".to_string(), "ELF".to_string());
                analysis.insert("machine".to_string(), format!("{}", elf.header.e_machine));
                analysis.insert("entry_point".to_string(), format!("0x{:x}", elf.header.e_entry));
                analysis.insert("sections".to_string(), elf.section_headers.len().to_string());
                
                // Analyze sections
                let mut suspicious_sections = Vec::new();
                for section in &elf.section_headers {
                    if let Some(name) = elf.shdr_strtab.get_at(section.sh_name) {
                        if name.contains("debug") || name.contains("trace") {
                            suspicious_sections.push(name.to_string());
                        }
                    }
                }
                
                if !suspicious_sections.is_empty() {
                    analysis.insert("suspicious_sections".to_string(), suspicious_sections.join(", "));
                }
            }
            Err(_) => {
                analysis.insert("format".to_string(), "Unknown/Invalid".to_string());
                warn!("Failed to parse as ELF, analyzing as raw bytecode");
            }
        }

        Ok(analysis)
    }
}
