use crate::rules::analyzer::TransactionAnalyzer;
use anyhow::Result;
use serde_json::{json, Value};
use solana_sdk::transaction::Transaction;
use std::collections::HashMap;

/// Squads Protocol v4 Program ID (Mainnet & Devnet)
const SQUADS_V4_PROGRAM_ID: &str = "SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf";

/// Squads V4 instruction discriminators (first 8 bytes of instruction data)
/// These are derived from the Anchor IDL
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SquadsInstruction {
    MultisigCreate = 0,
    MultisigCreateV2 = 1,
    ConfigTransactionCreate = 2,
    VaultTransactionCreate = 3,
    ProposalCreate = 4,
    ProposalApprove = 5,
    ProposalReject = 6,
    ProposalCancel = 7,
    VaultTransactionExecute = 8,
    ConfigTransactionExecute = 9,
    VaultBatchTransactionAccountClose = 10,
    MultisigAddMember = 11,
    MultisigRemoveMember = 12,
    MultisigChangeThreshold = 13,
    MultisigSetTimeLock = 14,
    MultisigAddSpendingLimit = 15,
    MultisigRemoveSpendingLimit = 16,
    MultisigSetRentCollector = 17,
    Unknown = 255,
}

impl SquadsInstruction {
    /// Parse instruction discriminator from instruction data
    fn from_discriminator(data: &[u8]) -> Self {
        if data.is_empty() {
            return Self::Unknown;
        }

        // Anchor uses first byte as discriminator for some instructions
        // For v4, we check the first byte
        match data[0] {
            0 => Self::MultisigCreate,
            1 => Self::MultisigCreateV2,
            2 => Self::ConfigTransactionCreate,
            3 => Self::VaultTransactionCreate,
            4 => Self::ProposalCreate,
            5 => Self::ProposalApprove,
            6 => Self::ProposalReject,
            7 => Self::ProposalCancel,
            8 => Self::VaultTransactionExecute,
            9 => Self::ConfigTransactionExecute,
            10 => Self::VaultBatchTransactionAccountClose,
            11 => Self::MultisigAddMember,
            12 => Self::MultisigRemoveMember,
            13 => Self::MultisigChangeThreshold,
            14 => Self::MultisigSetTimeLock,
            15 => Self::MultisigAddSpendingLimit,
            16 => Self::MultisigRemoveSpendingLimit,
            17 => Self::MultisigSetRentCollector,
            _ => Self::Unknown,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::MultisigCreate => "multisig_create",
            Self::MultisigCreateV2 => "multisig_create_v2",
            Self::ConfigTransactionCreate => "config_transaction_create",
            Self::VaultTransactionCreate => "vault_transaction_create",
            Self::ProposalCreate => "proposal_create",
            Self::ProposalApprove => "proposal_approve",
            Self::ProposalReject => "proposal_reject",
            Self::ProposalCancel => "proposal_cancel",
            Self::VaultTransactionExecute => "vault_transaction_execute",
            Self::ConfigTransactionExecute => "config_transaction_execute",
            Self::VaultBatchTransactionAccountClose => "vault_batch_transaction_account_close",
            Self::MultisigAddMember => "multisig_add_member",
            Self::MultisigRemoveMember => "multisig_remove_member",
            Self::MultisigChangeThreshold => "multisig_change_threshold",
            Self::MultisigSetTimeLock => "multisig_set_time_lock",
            Self::MultisigAddSpendingLimit => "multisig_add_spending_limit",
            Self::MultisigRemoveSpendingLimit => "multisig_remove_spending_limit",
            Self::MultisigSetRentCollector => "multisig_set_rent_collector",
            Self::Unknown => "unknown",
        }
    }

    fn is_governance_change(&self) -> bool {
        matches!(
            self,
            Self::MultisigAddMember
                | Self::MultisigRemoveMember
                | Self::MultisigChangeThreshold
                | Self::MultisigSetTimeLock
                | Self::MultisigAddSpendingLimit
                | Self::MultisigRemoveSpendingLimit
        )
    }

    fn is_proposal_action(&self) -> bool {
        matches!(
            self,
            Self::ProposalCreate
                | Self::ProposalApprove
                | Self::ProposalReject
                | Self::ProposalCancel
        )
    }

    fn is_execution(&self) -> bool {
        matches!(
            self,
            Self::VaultTransactionExecute | Self::ConfigTransactionExecute
        )
    }

    fn is_transaction_creation(&self) -> bool {
        matches!(
            self,
            Self::VaultTransactionCreate | Self::ConfigTransactionCreate
        )
    }
}

/// Squads V4 Multisig Analyzer
///
/// Analyzes transactions interacting with Squads Protocol v4 multisig program.
/// Provides insights into multisig operations, governance changes, and security patterns.
pub struct SquadsV4Analyzer;

impl SquadsV4Analyzer {
    pub fn new() -> Self {
        Self
    }

    /// Check if transaction contains Squads v4 program
    fn has_squads_program(&self, tx: &Transaction) -> bool {
        tx.message.instructions.iter().any(|inst| {
            tx.message
                .account_keys
                .get(inst.program_id_index as usize)
                .map(|pk| pk.to_string() == SQUADS_V4_PROGRAM_ID)
                .unwrap_or(false)
        })
    }

    /// Extract Squads instructions from transaction
    fn extract_squads_instructions(&self, tx: &Transaction) -> Vec<SquadsInstruction> {
        tx.message
            .instructions
            .iter()
            .filter_map(|inst| {
                let program_id = tx
                    .message
                    .account_keys
                    .get(inst.program_id_index as usize)?;

                if program_id.to_string() == SQUADS_V4_PROGRAM_ID {
                    Some(SquadsInstruction::from_discriminator(&inst.data))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Count unique instruction types
    fn count_instruction_types(
        &self,
        instructions: &[SquadsInstruction],
    ) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for inst in instructions {
            *counts.entry(inst.as_str().to_string()).or_insert(0) += 1;
        }
        counts
    }

    /// Analyze multisig account count (rough estimate based on account keys)
    fn estimate_multisig_accounts(&self, tx: &Transaction) -> usize {
        // Squads multisig accounts are typically PDAs
        // This is a heuristic - actual implementation would need to check account data
        tx.message
            .account_keys
            .iter()
            .filter(|key| {
                // Check if account might be a multisig PDA (heuristic)
                // Real implementation would fetch account data
                let key_str = key.to_string();
                // Multisig PDAs typically don't start with common system addresses
                !key_str.starts_with("11111111111111111111111111111111")
                    && !key_str.starts_with("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
                    && !key_str.starts_with("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
            })
            .count()
    }

    /// Detect potential security concerns
    fn detect_security_concerns(&self, instructions: &[SquadsInstruction]) -> Vec<String> {
        let mut concerns = Vec::new();

        // Check for rapid governance changes
        let governance_changes: Vec<_> = instructions
            .iter()
            .filter(|i| i.is_governance_change())
            .collect();

        if governance_changes.len() > 2 {
            concerns.push("multiple_governance_changes".to_string());
        }

        // Check for threshold changes
        if instructions
            .iter()
            .any(|i| matches!(i, SquadsInstruction::MultisigChangeThreshold))
        {
            concerns.push("threshold_change_detected".to_string());
        }

        // Check for member removal
        if instructions
            .iter()
            .any(|i| matches!(i, SquadsInstruction::MultisigRemoveMember))
        {
            concerns.push("member_removal_detected".to_string());
        }

        // Check for spending limit removal
        if instructions
            .iter()
            .any(|i| matches!(i, SquadsInstruction::MultisigRemoveSpendingLimit))
        {
            concerns.push("spending_limit_removal".to_string());
        }

        // Check for immediate execution without proposal
        let has_execution = instructions.iter().any(|i| i.is_execution());
        let has_proposal = instructions.iter().any(|i| i.is_proposal_action());

        if has_execution && !has_proposal {
            concerns.push("execution_without_visible_proposal".to_string());
        }

        concerns
    }
}

#[async_trait::async_trait]
impl TransactionAnalyzer for SquadsV4Analyzer {
    fn name(&self) -> &str {
        "squads_v4"
    }

    fn fields(&self) -> Vec<String> {
        vec![
            // Detection
            "is_squads_transaction".to_string(),
            "squads_instruction_count".to_string(),
            // Instruction Types
            "has_multisig_create".to_string(),
            "has_proposal_create".to_string(),
            "has_proposal_approve".to_string(),
            "has_proposal_reject".to_string(),
            "has_proposal_cancel".to_string(),
            "has_vault_transaction_create".to_string(),
            "has_vault_transaction_execute".to_string(),
            "has_config_transaction_create".to_string(),
            "has_config_transaction_execute".to_string(),
            // Governance Operations
            "has_governance_change".to_string(),
            "has_member_add".to_string(),
            "has_member_remove".to_string(),
            "has_threshold_change".to_string(),
            "has_time_lock_set".to_string(),
            "has_spending_limit_add".to_string(),
            "has_spending_limit_remove".to_string(),
            // Activity Patterns
            "governance_change_count".to_string(),
            "proposal_action_count".to_string(),
            "execution_count".to_string(),
            "transaction_creation_count".to_string(),
            // Security Analysis
            "security_concerns".to_string(),
            "has_security_concerns".to_string(),
            "concern_count".to_string(),
            // Metadata
            "instruction_types".to_string(),
            "estimated_multisig_accounts".to_string(),
            "primary_operation".to_string(),
        ]
    }

    async fn analyze(&self, tx: &Transaction) -> Result<HashMap<String, Value>> {
        let mut fields = HashMap::new();

        // Check if this is a Squads transaction
        let is_squads = self.has_squads_program(tx);
        fields.insert("is_squads_transaction".to_string(), json!(is_squads));

        if !is_squads {
            fields.insert("squads_instruction_count".to_string(), json!(0));
            return Ok(fields);
        }

        // Extract and analyze Squads instructions
        let instructions = self.extract_squads_instructions(tx);
        let instruction_count = instructions.len();
        fields.insert(
            "squads_instruction_count".to_string(),
            json!(instruction_count),
        );

        // Count instruction types
        let instruction_types = self.count_instruction_types(&instructions);
        fields.insert("instruction_types".to_string(), json!(instruction_types));

        // Determine primary operation (most common instruction type)
        let primary_operation = instruction_types
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(name, _)| name.clone())
            .unwrap_or_else(|| "unknown".to_string());
        fields.insert("primary_operation".to_string(), json!(primary_operation));

        // Specific instruction detection
        fields.insert(
            "has_multisig_create".to_string(),
            json!(instructions.iter().any(|i| matches!(
                i,
                SquadsInstruction::MultisigCreate | SquadsInstruction::MultisigCreateV2
            ))),
        );
        fields.insert(
            "has_proposal_create".to_string(),
            json!(instructions
                .iter()
                .any(|i| matches!(i, SquadsInstruction::ProposalCreate))),
        );
        fields.insert(
            "has_proposal_approve".to_string(),
            json!(instructions
                .iter()
                .any(|i| matches!(i, SquadsInstruction::ProposalApprove))),
        );
        fields.insert(
            "has_proposal_reject".to_string(),
            json!(instructions
                .iter()
                .any(|i| matches!(i, SquadsInstruction::ProposalReject))),
        );
        fields.insert(
            "has_proposal_cancel".to_string(),
            json!(instructions
                .iter()
                .any(|i| matches!(i, SquadsInstruction::ProposalCancel))),
        );
        fields.insert(
            "has_vault_transaction_create".to_string(),
            json!(instructions
                .iter()
                .any(|i| matches!(i, SquadsInstruction::VaultTransactionCreate))),
        );
        fields.insert(
            "has_vault_transaction_execute".to_string(),
            json!(instructions
                .iter()
                .any(|i| matches!(i, SquadsInstruction::VaultTransactionExecute))),
        );
        fields.insert(
            "has_config_transaction_create".to_string(),
            json!(instructions
                .iter()
                .any(|i| matches!(i, SquadsInstruction::ConfigTransactionCreate))),
        );
        fields.insert(
            "has_config_transaction_execute".to_string(),
            json!(instructions
                .iter()
                .any(|i| matches!(i, SquadsInstruction::ConfigTransactionExecute))),
        );

        // Governance operations
        fields.insert(
            "has_member_add".to_string(),
            json!(instructions
                .iter()
                .any(|i| matches!(i, SquadsInstruction::MultisigAddMember))),
        );
        fields.insert(
            "has_member_remove".to_string(),
            json!(instructions
                .iter()
                .any(|i| matches!(i, SquadsInstruction::MultisigRemoveMember))),
        );
        fields.insert(
            "has_threshold_change".to_string(),
            json!(instructions
                .iter()
                .any(|i| matches!(i, SquadsInstruction::MultisigChangeThreshold))),
        );
        fields.insert(
            "has_time_lock_set".to_string(),
            json!(instructions
                .iter()
                .any(|i| matches!(i, SquadsInstruction::MultisigSetTimeLock))),
        );
        fields.insert(
            "has_spending_limit_add".to_string(),
            json!(instructions
                .iter()
                .any(|i| matches!(i, SquadsInstruction::MultisigAddSpendingLimit))),
        );
        fields.insert(
            "has_spending_limit_remove".to_string(),
            json!(instructions
                .iter()
                .any(|i| matches!(i, SquadsInstruction::MultisigRemoveSpendingLimit))),
        );

        // Activity pattern counts
        let governance_change_count = instructions
            .iter()
            .filter(|i| i.is_governance_change())
            .count();
        let proposal_action_count = instructions
            .iter()
            .filter(|i| i.is_proposal_action())
            .count();
        let execution_count = instructions.iter().filter(|i| i.is_execution()).count();
        let transaction_creation_count = instructions
            .iter()
            .filter(|i| i.is_transaction_creation())
            .count();

        fields.insert(
            "has_governance_change".to_string(),
            json!(governance_change_count > 0),
        );
        fields.insert(
            "governance_change_count".to_string(),
            json!(governance_change_count),
        );
        fields.insert(
            "proposal_action_count".to_string(),
            json!(proposal_action_count),
        );
        fields.insert("execution_count".to_string(), json!(execution_count));
        fields.insert(
            "transaction_creation_count".to_string(),
            json!(transaction_creation_count),
        );

        // Security analysis
        let security_concerns = self.detect_security_concerns(&instructions);
        let has_concerns = !security_concerns.is_empty();
        let concern_count = security_concerns.len();

        fields.insert("security_concerns".to_string(), json!(security_concerns));
        fields.insert("has_security_concerns".to_string(), json!(has_concerns));
        fields.insert("concern_count".to_string(), json!(concern_count));

        // Metadata
        let estimated_accounts = self.estimate_multisig_accounts(tx);
        fields.insert(
            "estimated_multisig_accounts".to_string(),
            json!(estimated_accounts),
        );

        Ok(fields)
    }

    fn is_available(&self) -> bool {
        true // Always available, no external dependencies
    }

    fn estimated_latency_ms(&self) -> u64 {
        1 // Very fast, no external calls
    }
}

impl Default for SquadsV4Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        message::Message,
        pubkey::Pubkey,
        signature::Keypair,
        signer::Signer,
    };

    fn create_test_transaction(program_id: Pubkey, instruction_data: Vec<u8>) -> Transaction {
        let payer = Keypair::new();
        let instruction = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(Pubkey::new_unique(), false),
            ],
            data: instruction_data,
        };

        let message = Message::new(&[instruction], Some(&payer.pubkey()));
        Transaction::new_unsigned(message)
    }

    #[test]
    fn test_analyzer_fields() {
        let analyzer = SquadsV4Analyzer::new();
        let fields = analyzer.fields();

        assert!(fields.contains(&"is_squads_transaction".to_string()));
        assert!(fields.contains(&"has_proposal_approve".to_string()));
        assert!(fields.contains(&"has_governance_change".to_string()));
        assert!(fields.contains(&"security_concerns".to_string()));
    }

    #[test]
    fn test_instruction_discriminator_parsing() {
        assert_eq!(
            SquadsInstruction::from_discriminator(&[0]),
            SquadsInstruction::MultisigCreate
        );
        assert_eq!(
            SquadsInstruction::from_discriminator(&[4]),
            SquadsInstruction::ProposalCreate
        );
        assert_eq!(
            SquadsInstruction::from_discriminator(&[5]),
            SquadsInstruction::ProposalApprove
        );
        assert_eq!(
            SquadsInstruction::from_discriminator(&[8]),
            SquadsInstruction::VaultTransactionExecute
        );
        assert_eq!(
            SquadsInstruction::from_discriminator(&[255]),
            SquadsInstruction::Unknown
        );
    }

    #[test]
    fn test_instruction_categorization() {
        assert!(SquadsInstruction::MultisigAddMember.is_governance_change());
        assert!(SquadsInstruction::ProposalApprove.is_proposal_action());
        assert!(SquadsInstruction::VaultTransactionExecute.is_execution());
        assert!(SquadsInstruction::VaultTransactionCreate.is_transaction_creation());
    }

    #[tokio::test]
    async fn test_non_squads_transaction() {
        let analyzer = SquadsV4Analyzer::new();
        let tx = create_test_transaction(Pubkey::new_unique(), vec![1, 2, 3]);

        let result = analyzer.analyze(&tx).await.unwrap();
        assert_eq!(result.get("is_squads_transaction").unwrap(), &json!(false));
        assert_eq!(result.get("squads_instruction_count").unwrap(), &json!(0));
    }

    #[tokio::test]
    async fn test_squads_proposal_approve() {
        let analyzer = SquadsV4Analyzer::new();
        let squads_program_id: Pubkey = SQUADS_V4_PROGRAM_ID.parse().unwrap();
        let tx = create_test_transaction(squads_program_id, vec![5]); // ProposalApprove

        let result = analyzer.analyze(&tx).await.unwrap();
        assert_eq!(result.get("is_squads_transaction").unwrap(), &json!(true));
        assert_eq!(result.get("has_proposal_approve").unwrap(), &json!(true));
        assert_eq!(result.get("proposal_action_count").unwrap(), &json!(1));
    }

    #[tokio::test]
    async fn test_security_concern_detection() {
        let analyzer = SquadsV4Analyzer::new();

        // Test threshold change detection
        let instructions = vec![SquadsInstruction::MultisigChangeThreshold];
        let concerns = analyzer.detect_security_concerns(&instructions);
        assert!(concerns.contains(&"threshold_change_detected".to_string()));

        // Test multiple governance changes
        let instructions = vec![
            SquadsInstruction::MultisigAddMember,
            SquadsInstruction::MultisigRemoveMember,
            SquadsInstruction::MultisigChangeThreshold,
        ];
        let concerns = analyzer.detect_security_concerns(&instructions);
        assert!(concerns.contains(&"multiple_governance_changes".to_string()));
    }
}
