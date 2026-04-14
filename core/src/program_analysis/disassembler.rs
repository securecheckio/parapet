// BPF disassembler - ported from txfilter
// Analyzes program bytecode for suspicious patterns

use anyhow::Result;
use goblin::elf::Elf;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisassemblyResult {
    pub total_instructions: usize,
    pub suspicious_instruction_count: usize,
    pub suspicious_patterns: Vec<String>,
    pub instruction_categories: HashMap<String, usize>,
    pub entropy_score: f64,
    pub complexity_score: f64,
    pub control_flow_node_count: usize,
    pub control_flow_edge_count: usize,
    pub has_account_write: bool,
    pub account_write_count: usize,
    pub has_cpi_call: bool,
    pub cpi_call_count: usize,
    pub reads_account_data: bool,
    pub account_read_count: usize,
    pub has_signer_check: bool,
    pub has_owner_check: bool,
    pub has_key_check: bool,
    pub checked_account_count: usize,
    pub unchecked_account_count: usize,
    pub missing_signer_check: bool,
    pub missing_owner_check: bool,
    pub arbitrary_cpi: bool,
    pub spl_token_related: bool,
    pub token_2022_related: bool,
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub address: u64,
    pub mnemonic: String,
    pub op_str: String,
    pub bytes: Vec<u8>,
    pub opcode: u8,
    pub src_reg: u8,
    pub dst_reg: u8,
    pub offset: i16,
    pub immediate: i32,
}

pub struct ProgramDisassembler {
    // Simple BPF bytecode analyzer (without capstone for now)
}

impl ProgramDisassembler {
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }

    pub fn disassemble(&self, program_data: &[u8]) -> Result<DisassemblyResult> {
        info!(
            "Starting program disassembly, data size: {} bytes",
            program_data.len()
        );

        let instructions = self.analyze_bytecode(program_data)?;
        let entropy_score = self.calculate_entropy(program_data);
        let complexity_score = self.calculate_complexity(program_data);
        let mut suspicious_patterns = self.detect_suspicious_patterns(program_data);
        let cfg = self.build_control_flow_graph(&instructions);
        let behavior = self.detect_behavioral_patterns(program_data, &instructions);
        suspicious_patterns.extend(behavior.suspicious_patterns.clone());

        Ok(DisassemblyResult {
            total_instructions: instructions.len(),
            suspicious_instruction_count: behavior.suspicious_instruction_count,
            suspicious_patterns,
            instruction_categories: behavior.instruction_categories,
            entropy_score,
            complexity_score,
            control_flow_node_count: cfg.node_count,
            control_flow_edge_count: cfg.edge_count,
            has_account_write: behavior.account_write_count > 0,
            account_write_count: behavior.account_write_count,
            has_cpi_call: behavior.cpi_call_count > 0,
            cpi_call_count: behavior.cpi_call_count,
            reads_account_data: behavior.account_read_count > 0,
            account_read_count: behavior.account_read_count,
            has_signer_check: behavior.has_signer_check,
            has_owner_check: behavior.has_owner_check,
            has_key_check: behavior.has_key_check,
            checked_account_count: behavior.checked_account_count,
            unchecked_account_count: behavior.unchecked_account_count,
            missing_signer_check: behavior.missing_signer_check,
            missing_owner_check: behavior.missing_owner_check,
            arbitrary_cpi: behavior.arbitrary_cpi,
            spl_token_related: behavior.spl_token_related,
            token_2022_related: behavior.token_2022_related,
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

        let opcode = bytes[0];
        let regs = bytes[1];
        let dst_reg = regs & 0x0f;
        let src_reg = (regs >> 4) & 0x0f;
        let offset = i16::from_le_bytes([bytes[2], bytes[3]]);
        let immediate = i32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);

        let class = opcode & 0x07;
        let mnemonic = match class {
            0x00 => "ld",
            0x01 => "ldx",
            0x02 => "st",
            0x03 => "stx",
            0x04 => "alu",
            0x05 => "jmp",
            0x06 => "jmp32",
            0x07 => "alu64",
            _ => "invalid",
        }
        .to_string();

        Some(Instruction {
            address,
            mnemonic,
            op_str: format!(
                "op=0x{opcode:02x} dst=r{dst_reg} src=r{src_reg} off={offset} imm={immediate}"
            ),
            bytes: bytes.to_vec(),
            opcode,
            src_reg,
            dst_reg,
            offset,
            immediate,
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

    fn detect_behavioral_patterns(&self, data: &[u8], instructions: &[Instruction]) -> BehavioralPatternResult {
        let mut instruction_categories = HashMap::new();

        let mut account_write_count = 0usize;
        let mut cpi_call_count = 0usize;
        let mut account_read_count = 0usize;
        let mut has_signer_check = false;
        let mut has_owner_check = false;
        let mut has_key_check = false;
        let mut checked_account_count = 0usize;
        let mut suspicious_instruction_count = 0usize;
        let mut suspicious_patterns = Vec::new();

        // Heuristics are symbol/string based because Solana programs usually preserve syscall labels.
        let marker_count = |needle: &[u8]| data.windows(needle.len()).filter(|w| *w == needle).count();
        let has_marker = |needle: &[u8]| data.windows(needle.len()).any(|w| w == needle);

        let write_markers = [b"try_borrow_mut_data".as_slice(), b"set_lamports", b"transfer"];
        for marker in write_markers {
            account_write_count += marker_count(marker);
        }
        if account_write_count > 0 {
            instruction_categories.insert("account_write".to_string(), account_write_count);
        }

        let cpi_markers = [b"invoke_signed".as_slice(), b"invoke".as_slice(), b"sol_invoke"];
        for marker in cpi_markers {
            cpi_call_count += marker_count(marker);
        }
        if cpi_call_count > 0 {
            instruction_categories.insert("cpi_call".to_string(), cpi_call_count);
        }

        let read_markers = [b"try_borrow_data".as_slice(), b"account.data", b"AccountInfo"];
        for marker in read_markers {
            account_read_count += marker_count(marker);
        }
        if account_read_count > 0 {
            instruction_categories.insert("account_read".to_string(), account_read_count);
        }

        if has_marker(b"is_signer") || has_marker(b"Signer") || has_marker(b"MissingRequiredSignature") {
            has_signer_check = true;
            checked_account_count += 1;
        }
        if has_marker(b"owner") || has_marker(b"IncorrectProgramId") || has_marker(b"ProgramOwner") {
            has_owner_check = true;
            checked_account_count += 1;
        }
        if has_marker(b"Pubkey::eq") || has_marker(b"account.key") || has_marker(b"InvalidAccountData") {
            has_key_check = true;
            checked_account_count += 1;
        }

        let mut branch_count = 0usize;
        for instruction in instructions {
            if instruction.mnemonic == "jmp" || instruction.mnemonic == "jmp32" {
                branch_count += 1;
                if instruction.offset != 0 {
                    suspicious_instruction_count += 1;
                }
            }
            if instruction.mnemonic == "st" || instruction.mnemonic == "stx" {
                suspicious_instruction_count += 1;
            }
        }
        instruction_categories.insert("branch".to_string(), branch_count);

        let unchecked_account_count = account_write_count
            .saturating_add(account_read_count)
            .saturating_sub(checked_account_count);

        let missing_signer_check = account_write_count > 0 && !has_signer_check;
        if missing_signer_check {
            suspicious_patterns.push("Missing signer check pattern (MSC)".to_string());
        }
        let missing_owner_check = account_read_count > 0 && !has_owner_check;
        if missing_owner_check {
            suspicious_patterns.push("Missing owner check pattern (MOC)".to_string());
        }
        let arbitrary_cpi = cpi_call_count > 0 && !(has_owner_check || has_key_check);
        if arbitrary_cpi {
            suspicious_patterns.push("Arbitrary CPI target pattern (ACPI)".to_string());
        }

        let spl_token_related = has_marker(b"spl_token")
            || has_marker(b"TokenInstruction")
            || has_marker(b"InitializeMint")
            || has_marker(b"SetAuthority");
        let token_2022_related = has_marker(b"spl_token_2022")
            || has_marker(b"transfer_checked")
            || has_marker(b"InterestBearingMint")
            || has_marker(b"TransferHook");

        BehavioralPatternResult {
            instruction_categories,
            suspicious_instruction_count,
            suspicious_patterns,
            account_write_count,
            cpi_call_count,
            account_read_count,
            has_signer_check,
            has_owner_check,
            has_key_check,
            checked_account_count,
            unchecked_account_count,
            missing_signer_check,
            missing_owner_check,
            arbitrary_cpi,
            spl_token_related,
            token_2022_related,
        }
    }

    fn build_control_flow_graph(&self, instructions: &[Instruction]) -> ControlFlowGraphSummary {
        let mut edges: HashSet<(u64, u64)> = HashSet::new();
        let mut nodes: HashSet<u64> = HashSet::new();

        for (idx, instr) in instructions.iter().enumerate() {
            nodes.insert(instr.address);
            if let Some(next) = instructions.get(idx + 1) {
                edges.insert((instr.address, next.address));
            }

            if instr.mnemonic == "jmp" || instr.mnemonic == "jmp32" {
                let target = if instr.offset >= 0 {
                    instr.address + ((instr.offset as u64 + 1) * 8)
                } else {
                    instr.address.saturating_sub((instr.offset.unsigned_abs() as u64) * 8)
                };
                edges.insert((instr.address, target));
                nodes.insert(target);
            }
        }

        ControlFlowGraphSummary {
            node_count: nodes.len(),
            edge_count: edges.len(),
        }
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
                analysis.insert(
                    "entry_point".to_string(),
                    format!("0x{:x}", elf.header.e_entry),
                );
                analysis.insert(
                    "sections".to_string(),
                    elf.section_headers.len().to_string(),
                );

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
                    analysis.insert(
                        "suspicious_sections".to_string(),
                        suspicious_sections.join(", "),
                    );
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

#[derive(Debug, Clone)]
struct BehavioralPatternResult {
    instruction_categories: HashMap<String, usize>,
    suspicious_instruction_count: usize,
    suspicious_patterns: Vec<String>,
    account_write_count: usize,
    cpi_call_count: usize,
    account_read_count: usize,
    has_signer_check: bool,
    has_owner_check: bool,
    has_key_check: bool,
    checked_account_count: usize,
    unchecked_account_count: usize,
    missing_signer_check: bool,
    missing_owner_check: bool,
    arbitrary_cpi: bool,
    spl_token_related: bool,
    token_2022_related: bool,
}

#[derive(Debug, Clone)]
struct ControlFlowGraphSummary {
    node_count: usize,
    edge_count: usize,
}

#[cfg(test)]
mod tests {
    use super::ProgramDisassembler;

    #[test]
    fn detects_msc_and_acpi_patterns() {
        let disassembler = ProgramDisassembler::new().expect("disassembler");
        // two 8-byte instructions + symbol payload markers used by heuristic detection
        let mut data = vec![0x05, 0x10, 0x01, 0x00, 0, 0, 0, 0, 0x03, 0x00, 0, 0, 0, 0, 0, 0];
        data.extend_from_slice(b"try_borrow_mut_data invoke_signed");

        let result = disassembler.disassemble(&data).expect("disassemble");
        assert!(result.has_account_write);
        assert!(result.has_cpi_call);
        assert!(result.missing_signer_check);
        assert!(result.arbitrary_cpi);
    }

    #[test]
    fn detects_token_2022_markers() {
        let disassembler = ProgramDisassembler::new().expect("disassembler");
        let mut data = vec![0x05, 0x10, 0x00, 0x00, 0, 0, 0, 0];
        data.extend_from_slice(b"spl_token_2022 TransferHook");

        let result = disassembler.disassemble(&data).expect("disassemble");
        assert!(result.token_2022_related);
        assert!(!result.spl_token_related || result.token_2022_related);
    }
}
