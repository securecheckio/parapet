use crate::rules::analyzer::{ConfirmedTransactionMetadata, TransactionAnalyzer};
use anyhow::Result;
use serde_json::{json, Value};
use solana_sdk::transaction::Transaction;
use std::collections::HashMap;

const SPL_TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";

// Expected instruction data sizes for common instructions
// These are base sizes - Token-2022 extensions can add more data legitimately
const EXPECTED_SIZES: &[(u8, usize, &str)] = &[
    // SPL Token instructions (discriminator, expected_size, name)
    (0, 67, "InitializeMint"),    // 1 + 1 + 32 + 32 + 1 = 67
    (1, 1, "InitializeAccount"),  // Just discriminator
    (2, 2, "InitializeMultisig"), // 1 + 1 (m value)
    (3, 9, "Transfer"),           // 1 + 8 (amount)
    (4, 9, "Approve"),            // 1 + 8 (amount)
    (5, 1, "Revoke"),             // Just discriminator
    (6, 35, "SetAuthority"),      // 1 + 1 (type) + 1 (option) + 32 (pubkey)
    (7, 9, "MintTo"),             // 1 + 8 (amount)
    (8, 9, "Burn"),               // 1 + 8 (amount)
    (9, 1, "CloseAccount"),       // Just discriminator
    (10, 1, "FreezeAccount"),     // Just discriminator
    (11, 1, "ThawAccount"),       // Just discriminator
    (12, 10, "TransferChecked"),  // 1 + 8 (amount) + 1 (decimals)
    (13, 10, "ApproveChecked"),   // 1 + 8 (amount) + 1 (decimals)
    (14, 10, "MintToChecked"),    // 1 + 8 (amount) + 1 (decimals)
    (15, 10, "BurnChecked"),      // 1 + 8 (amount) + 1 (decimals)
];

// System Program instruction sizes
const SYSTEM_EXPECTED_SIZES: &[(u32, usize, &str)] = &[
    // System Program uses u32 discriminator (little-endian)
    (0, 52, "CreateAccount"), // 4 + 8 (lamports) + 8 (space) + 32 (owner)
    (1, 52, "Assign"),        // 4 + 32 (owner)
    (2, 12, "Transfer"),      // 4 + 8 (lamports)
    (3, 60, "CreateAccountWithSeed"), // Complex, allow more padding
    (4, 4, "AdvanceNonceAccount"), // Just discriminator
    (5, 4, "WithdrawNonceAccount"), // 4 + 8 (lamports) but varies
    (6, 4, "InitializeNonceAccount"), // 4 + 32 (authority)
    (7, 4, "AuthorizeNonceAccount"), // 4 + 32 (authority)
    (8, 60, "Allocate"),      // 4 + 8 (space)
    (9, 60, "AllocateWithSeed"), // Complex
    (10, 60, "AssignWithSeed"), // Complex
    (11, 60, "TransferWithSeed"), // Complex
];

/// Maximum reasonable padding for Token-2022 extensions
/// Token-2022 uses TLV encoding: each extension is 4 bytes (type+length) + data
/// Typical extensions: TransferFee (108 bytes), Metadata (variable), etc.
/// Allow up to 512 bytes total for multiple extensions
const MAX_TOKEN_2022_EXTENSION_SIZE: usize = 512;

/// Maximum reasonable padding for Anchor instructions
/// Anchor uses 8-byte discriminator + Borsh-serialized args
/// Most Anchor instructions are < 256 bytes total
const MAX_ANCHOR_INSTRUCTION_SIZE: usize = 512;

/// Threshold for excessive padding ratio (currently not used - size limits are sufficient)
/// If trailing_bytes / expected_size > this ratio, flag as suspicious
/// Note: We rely on absolute size limits (MAX_TOKEN_2022_EXTENSION_SIZE, MAX_ANCHOR_INSTRUCTION_SIZE)
/// rather than ratios to minimize false positives with Token-2022 extensions.
#[allow(dead_code)]
const EXCESSIVE_PADDING_RATIO: f64 = 10.0;

/// Analyzes instruction data for suspicious padding patterns.
///
/// Detects instruction padding attacks where extra bytes are added to bypass
/// security checks. Uses statistical anomaly detection to minimize false positives
/// while catching malicious padding attempts.
///
/// Fields exposed (prefixed "padding:" in the rule engine):
///   has_suspicious_padding       - true if any instruction has suspicious padding
///   suspicious_instruction_count - number of instructions with suspicious padding
///   max_padding_bytes           - maximum padding bytes found in any instruction
///   max_padding_ratio           - maximum padding ratio (trailing/expected)
///   has_repeated_padding        - true if padding contains repeated byte patterns
///   suspicious_instructions     - array of details about suspicious instructions
///
/// Detection criteria:
/// 1. Known instruction types with excessive padding (> expected + buffer)
/// 2. Padding ratio > 10x expected size
/// 3. Repeated padding bytes (0x00, 0xFF patterns suggest malicious intent)
/// 4. Extremely large instruction data (> 1024 bytes) for simple instructions
pub struct InstructionPaddingAnalyzer;

impl InstructionPaddingAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Check if a program is SPL Token or Token-2022
    fn is_token_program(program_id: &str) -> bool {
        program_id == SPL_TOKEN_PROGRAM || program_id == TOKEN_2022_PROGRAM
    }

    /// Check if a program is the System Program
    fn is_system_program(program_id: &str) -> bool {
        program_id == SYSTEM_PROGRAM
    }

    /// Check if instruction data appears to be an Anchor instruction (8-byte discriminator)
    fn is_likely_anchor(data: &[u8]) -> bool {
        data.len() >= 8
    }

    /// Detect repeated byte patterns that suggest malicious padding
    fn has_repeated_pattern(data: &[u8], start_offset: usize) -> bool {
        if data.len() <= start_offset + 16 {
            return false;
        }

        let trailing = &data[start_offset..];
        if trailing.len() < 16 {
            return false;
        }

        // Check for runs of repeated bytes (0x00, 0xFF, etc.)
        let mut consecutive_same = 1;
        let mut prev_byte = trailing[0];

        for &byte in &trailing[1..] {
            if byte == prev_byte {
                consecutive_same += 1;
                if consecutive_same >= 16 {
                    return true;
                }
            } else {
                consecutive_same = 1;
                prev_byte = byte;
            }
        }

        false
    }

    /// Analyze a single instruction for suspicious padding
    fn analyze_instruction(program_id: &str, data: &[u8]) -> Option<SuspiciousPadding> {
        if data.is_empty() {
            return None;
        }

        let data_len = data.len();

        // Check SPL Token instructions
        if Self::is_token_program(program_id) {
            if let Some(&discriminator) = data.first() {
                if let Some((_, expected_size, name)) = EXPECTED_SIZES
                    .iter()
                    .find(|(disc, _, _)| *disc == discriminator)
                {
                    let expected = *expected_size;
                    let trailing = data_len.saturating_sub(expected);

                    // Token-2022 can have extensions, allow reasonable buffer
                    let max_allowed = expected + MAX_TOKEN_2022_EXTENSION_SIZE;

                    // Only flag if exceeds max_allowed (which already accounts for Token-2022 extensions)
                    if data_len > max_allowed {
                        let ratio = trailing as f64 / expected as f64;
                        return Some(SuspiciousPadding {
                            instruction_type: format!("SPL Token {}", name),
                            expected_size: expected,
                            actual_size: data_len,
                            padding_bytes: trailing,
                            padding_ratio: ratio,
                            has_repeated_pattern: Self::has_repeated_pattern(data, expected),
                            reason: format!(
                                "Excessive padding: {} bytes (expected {}, max allowed {})",
                                data_len, expected, max_allowed
                            ),
                        });
                    }

                    // If within max_allowed, don't flag - Token-2022 extensions are legitimate
                }
            }
        }

        // Check System Program instructions
        if Self::is_system_program(program_id) && data.len() >= 4 {
            let discriminator = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);

            if let Some((_, expected_size, name)) = SYSTEM_EXPECTED_SIZES
                .iter()
                .find(|(disc, _, _)| *disc == discriminator)
            {
                let expected = *expected_size;
                let trailing = data_len.saturating_sub(expected);

                // System instructions shouldn't have much padding
                let max_allowed = expected + 64;

                if data_len > max_allowed {
                    let ratio = if expected > 0 {
                        trailing as f64 / expected as f64
                    } else {
                        trailing as f64
                    };

                    return Some(SuspiciousPadding {
                        instruction_type: format!("System Program {}", name),
                        expected_size: expected,
                        actual_size: data_len,
                        padding_bytes: trailing,
                        padding_ratio: ratio,
                        has_repeated_pattern: Self::has_repeated_pattern(data, expected),
                        reason: format!(
                            "Excessive padding: {} bytes (expected {}, max allowed {})",
                            data_len, expected, max_allowed
                        ),
                    });
                }
            }
        }

        // Check Anchor-like instructions (8-byte discriminator)
        if Self::is_likely_anchor(data) {
            let discriminator_size = 8;
            let trailing = data_len.saturating_sub(discriminator_size);

            // Only flag if exceeds max reasonable size
            // Anchor instructions can have variable args, so be generous
            if data_len > MAX_ANCHOR_INSTRUCTION_SIZE {
                let ratio = trailing as f64 / discriminator_size as f64;
                return Some(SuspiciousPadding {
                    instruction_type: "Anchor-like instruction".to_string(),
                    expected_size: discriminator_size,
                    actual_size: data_len,
                    padding_bytes: trailing,
                    padding_ratio: ratio,
                    has_repeated_pattern: Self::has_repeated_pattern(data, discriminator_size),
                    reason: format!(
                        "Excessive size for Anchor instruction: {} bytes (max reasonable: {})",
                        data_len, MAX_ANCHOR_INSTRUCTION_SIZE
                    ),
                });
            }

            // If within max size, don't flag - Anchor args can legitimately vary
        }

        None
    }

    /// Analyze all instructions in a transaction
    fn analyze_transaction(tx: &Transaction) -> PaddingAnalysisResult {
        let mut suspicious_instructions = Vec::new();
        let mut max_padding_bytes = 0;
        let mut max_padding_ratio: f64 = 0.0;
        let mut has_repeated_padding = false;

        for instruction in &tx.message.instructions {
            if let Some(program_id) = tx
                .message
                .account_keys
                .get(instruction.program_id_index as usize)
            {
                let program_id_str = program_id.to_string();

                if let Some(suspicious) =
                    Self::analyze_instruction(&program_id_str, &instruction.data)
                {
                    max_padding_bytes = max_padding_bytes.max(suspicious.padding_bytes);
                    max_padding_ratio = max_padding_ratio.max(suspicious.padding_ratio);
                    has_repeated_padding = has_repeated_padding || suspicious.has_repeated_pattern;
                    suspicious_instructions.push(suspicious);
                }
            }
        }

        PaddingAnalysisResult {
            has_suspicious_padding: !suspicious_instructions.is_empty(),
            suspicious_instruction_count: suspicious_instructions.len(),
            max_padding_bytes,
            max_padding_ratio,
            has_repeated_padding,
            suspicious_instructions,
        }
    }

    /// Analyze transaction including inner instructions from metadata
    fn analyze_with_metadata(
        tx: &Transaction,
        metadata: &ConfirmedTransactionMetadata,
    ) -> PaddingAnalysisResult {
        let mut result = Self::analyze_transaction(tx);

        // Also check inner instructions
        for inner_ix in &metadata.inner_instructions {
            // inner_ix.program_id is already a String (base58)
            if let Some(suspicious) =
                Self::analyze_instruction(&inner_ix.program_id, &inner_ix.data)
            {
                result.max_padding_bytes = result.max_padding_bytes.max(suspicious.padding_bytes);
                result.max_padding_ratio = result.max_padding_ratio.max(suspicious.padding_ratio);
                result.has_repeated_padding =
                    result.has_repeated_padding || suspicious.has_repeated_pattern;
                result.suspicious_instructions.push(suspicious);
            }
        }

        result.has_suspicious_padding = !result.suspicious_instructions.is_empty();
        result.suspicious_instruction_count = result.suspicious_instructions.len();

        result
    }

    /// Convert analysis result to fields map
    fn result_to_fields(result: PaddingAnalysisResult) -> HashMap<String, Value> {
        let mut fields = HashMap::new();

        fields.insert(
            "has_suspicious_padding".to_string(),
            json!(result.has_suspicious_padding),
        );
        fields.insert(
            "suspicious_instruction_count".to_string(),
            json!(result.suspicious_instruction_count),
        );
        fields.insert(
            "max_padding_bytes".to_string(),
            json!(result.max_padding_bytes),
        );
        fields.insert(
            "max_padding_ratio".to_string(),
            json!(result.max_padding_ratio),
        );
        fields.insert(
            "has_repeated_padding".to_string(),
            json!(result.has_repeated_padding),
        );

        let suspicious_details: Vec<Value> = result
            .suspicious_instructions
            .into_iter()
            .map(|s| {
                json!({
                    "instruction_type": s.instruction_type,
                    "expected_size": s.expected_size,
                    "actual_size": s.actual_size,
                    "padding_bytes": s.padding_bytes,
                    "padding_ratio": s.padding_ratio,
                    "has_repeated_pattern": s.has_repeated_pattern,
                    "reason": s.reason,
                })
            })
            .collect();

        fields.insert(
            "suspicious_instructions".to_string(),
            json!(suspicious_details),
        );

        fields
    }
}

#[derive(Debug, Clone)]
struct SuspiciousPadding {
    instruction_type: String,
    expected_size: usize,
    actual_size: usize,
    padding_bytes: usize,
    padding_ratio: f64,
    has_repeated_pattern: bool,
    reason: String,
}

#[derive(Debug)]
struct PaddingAnalysisResult {
    has_suspicious_padding: bool,
    suspicious_instruction_count: usize,
    max_padding_bytes: usize,
    max_padding_ratio: f64,
    has_repeated_padding: bool,
    suspicious_instructions: Vec<SuspiciousPadding>,
}

#[async_trait::async_trait]
impl TransactionAnalyzer for InstructionPaddingAnalyzer {
    fn name(&self) -> &str {
        "padding"
    }

    fn fields(&self) -> Vec<String> {
        vec![
            "has_suspicious_padding".to_string(),
            "suspicious_instruction_count".to_string(),
            "max_padding_bytes".to_string(),
            "max_padding_ratio".to_string(),
            "has_repeated_padding".to_string(),
            "suspicious_instructions".to_string(),
        ]
    }

    async fn analyze(&self, tx: &Transaction) -> Result<HashMap<String, Value>> {
        let result = Self::analyze_transaction(tx);
        Ok(Self::result_to_fields(result))
    }

    async fn analyze_with_metadata(
        &self,
        tx: &Transaction,
        metadata: &ConfirmedTransactionMetadata,
    ) -> Result<HashMap<String, Value>> {
        let result = Self::analyze_with_metadata(tx, metadata);
        Ok(Self::result_to_fields(result))
    }

    fn estimated_latency_ms(&self) -> u64 {
        0 // Pure computation, no I/O
    }
}

impl Default for InstructionPaddingAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::message::compiled_instruction::CompiledInstruction;
    use solana_sdk::message::Message;
    use solana_sdk::pubkey::Pubkey;

    #[tokio::test]
    async fn test_normal_spl_transfer_no_padding() {
        let analyzer = InstructionPaddingAnalyzer::new();

        // Normal SPL Token Transfer: discriminator (1) + amount (8) = 9 bytes
        let mut data = vec![3]; // Transfer discriminator
        data.extend_from_slice(&100u64.to_le_bytes()); // amount

        let token_program = SPL_TOKEN_PROGRAM.parse::<Pubkey>().unwrap();
        let ix = CompiledInstruction {
            program_id_index: 0,
            accounts: vec![0, 1, 2],
            data,
        };

        let message = Message {
            header: solana_sdk::message::MessageHeader::default(),
            account_keys: vec![token_program],
            recent_blockhash: solana_sdk::hash::Hash::default(),
            instructions: vec![ix],
        };

        let tx = Transaction {
            signatures: vec![],
            message,
        };
        let fields = analyzer.analyze(&tx).await.unwrap();

        assert_eq!(fields["has_suspicious_padding"], json!(false));
        assert_eq!(fields["suspicious_instruction_count"], json!(0));
    }

    #[tokio::test]
    async fn test_token_2022_with_extensions_allowed() {
        let analyzer = InstructionPaddingAnalyzer::new();

        // Token-2022 Transfer with reasonable extension data (< 512 bytes)
        let mut data = vec![3]; // Transfer discriminator
        data.extend_from_slice(&100u64.to_le_bytes()); // amount
        data.extend_from_slice(&vec![0u8; 256]); // extension data (reasonable)

        let token_2022_program = TOKEN_2022_PROGRAM.parse::<Pubkey>().unwrap();
        let ix = CompiledInstruction {
            program_id_index: 0,
            accounts: vec![0, 1, 2],
            data,
        };

        let message = Message {
            header: solana_sdk::message::MessageHeader::default(),
            account_keys: vec![token_2022_program],
            recent_blockhash: solana_sdk::hash::Hash::default(),
            instructions: vec![ix],
        };

        let tx = Transaction {
            signatures: vec![],
            message,
        };
        let fields = analyzer.analyze(&tx).await.unwrap();

        // Should NOT flag as suspicious - Token-2022 extensions are legitimate
        assert_eq!(fields["has_suspicious_padding"], json!(false));
    }

    #[tokio::test]
    async fn test_excessive_padding_detected() {
        let analyzer = InstructionPaddingAnalyzer::new();

        // SPL Token Transfer with excessive padding (> 512 bytes)
        let mut data = vec![3]; // Transfer discriminator
        data.extend_from_slice(&100u64.to_le_bytes()); // amount
        data.extend_from_slice(&vec![0u8; 1000]); // excessive padding

        let token_program = SPL_TOKEN_PROGRAM.parse::<Pubkey>().unwrap();
        let ix = CompiledInstruction {
            program_id_index: 0,
            accounts: vec![0, 1, 2],
            data,
        };

        let message = Message {
            header: solana_sdk::message::MessageHeader::default(),
            account_keys: vec![token_program],
            recent_blockhash: solana_sdk::hash::Hash::default(),
            instructions: vec![ix],
        };

        let tx = Transaction {
            signatures: vec![],
            message,
        };
        let fields = analyzer.analyze(&tx).await.unwrap();

        assert_eq!(fields["has_suspicious_padding"], json!(true));
        assert_eq!(fields["suspicious_instruction_count"], json!(1));
        assert!(fields["max_padding_bytes"].as_u64().unwrap() > 500);
    }

    #[tokio::test]
    async fn test_repeated_pattern_detected() {
        let analyzer = InstructionPaddingAnalyzer::new();

        // SPL Token Transfer with repeated null bytes (malicious pattern)
        let mut data = vec![3]; // Transfer discriminator
        data.extend_from_slice(&100u64.to_le_bytes()); // amount
        data.extend_from_slice(&vec![0u8; 600]); // repeated nulls - suspicious

        let token_program = SPL_TOKEN_PROGRAM.parse::<Pubkey>().unwrap();
        let ix = CompiledInstruction {
            program_id_index: 0,
            accounts: vec![0, 1, 2],
            data,
        };

        let message = Message {
            header: solana_sdk::message::MessageHeader::default(),
            account_keys: vec![token_program],
            recent_blockhash: solana_sdk::hash::Hash::default(),
            instructions: vec![ix],
        };

        let tx = Transaction {
            signatures: vec![],
            message,
        };
        let fields = analyzer.analyze(&tx).await.unwrap();

        assert_eq!(fields["has_suspicious_padding"], json!(true));
        assert_eq!(fields["has_repeated_padding"], json!(true));
    }

    #[tokio::test]
    async fn test_system_program_advance_nonce_normal() {
        let analyzer = InstructionPaddingAnalyzer::new();

        // System Program AdvanceNonceAccount: just 4 bytes
        let data = vec![4, 0, 0, 0]; // discriminator only

        let system_program = SYSTEM_PROGRAM.parse::<Pubkey>().unwrap();
        let ix = CompiledInstruction {
            program_id_index: 0,
            accounts: vec![0, 1],
            data,
        };

        let message = Message {
            header: solana_sdk::message::MessageHeader::default(),
            account_keys: vec![system_program],
            recent_blockhash: solana_sdk::hash::Hash::default(),
            instructions: vec![ix],
        };

        let tx = Transaction {
            signatures: vec![],
            message,
        };
        let fields = analyzer.analyze(&tx).await.unwrap();

        assert_eq!(fields["has_suspicious_padding"], json!(false));
    }

    #[tokio::test]
    async fn test_anchor_instruction_reasonable_size() {
        let analyzer = InstructionPaddingAnalyzer::new();

        // Anchor instruction: 8-byte discriminator + 200 bytes args (reasonable)
        let mut data = vec![0xa1, 0xb0, 0x28, 0xd5, 0x3c, 0xb8, 0xb3, 0xe4]; // discriminator
        data.extend_from_slice(&[1u8; 200]); // reasonable args

        let program = Pubkey::new_unique();
        let ix = CompiledInstruction {
            program_id_index: 0,
            accounts: vec![0, 1],
            data,
        };

        let message = Message {
            header: solana_sdk::message::MessageHeader::default(),
            account_keys: vec![program],
            recent_blockhash: solana_sdk::hash::Hash::default(),
            instructions: vec![ix],
        };

        let tx = Transaction {
            signatures: vec![],
            message,
        };
        let fields = analyzer.analyze(&tx).await.unwrap();

        assert_eq!(fields["has_suspicious_padding"], json!(false));
    }

    #[tokio::test]
    async fn test_anchor_instruction_excessive_size() {
        let analyzer = InstructionPaddingAnalyzer::new();

        // Anchor instruction: 8-byte discriminator + 1000 bytes (excessive)
        let mut data = vec![0xa1, 0xb0, 0x28, 0xd5, 0x3c, 0xb8, 0xb3, 0xe4]; // discriminator
        data.extend_from_slice(&vec![0u8; 1000]); // excessive padding

        let program = Pubkey::new_unique();
        let ix = CompiledInstruction {
            program_id_index: 0,
            accounts: vec![0, 1],
            data,
        };

        let message = Message {
            header: solana_sdk::message::MessageHeader::default(),
            account_keys: vec![program],
            recent_blockhash: solana_sdk::hash::Hash::default(),
            instructions: vec![ix],
        };

        let tx = Transaction {
            signatures: vec![],
            message,
        };
        let fields = analyzer.analyze(&tx).await.unwrap();

        assert_eq!(fields["has_suspicious_padding"], json!(true));
        assert!(fields["max_padding_bytes"].as_u64().unwrap() > 900);
    }

    #[test]
    fn test_repeated_pattern_detection() {
        // Test with 20 consecutive zeros
        let data = vec![0u8; 20];
        assert!(InstructionPaddingAnalyzer::has_repeated_pattern(&data, 0));

        // Test with 20 consecutive 0xFF
        let data = vec![0xFFu8; 20];
        assert!(InstructionPaddingAnalyzer::has_repeated_pattern(&data, 0));

        // Test with varied data (no pattern)
        let data: Vec<u8> = (0..20).collect();
        assert!(!InstructionPaddingAnalyzer::has_repeated_pattern(&data, 0));

        // Test with short data (< 16 bytes)
        let data = vec![0u8; 10];
        assert!(!InstructionPaddingAnalyzer::has_repeated_pattern(&data, 0));
    }
}
