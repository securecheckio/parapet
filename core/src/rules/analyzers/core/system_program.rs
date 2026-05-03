use crate::rules::analyzer::TransactionAnalyzer;
use anyhow::Result;
use serde_json::{json, Value};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use std::collections::HashMap;

const SYSTEM_PROGRAM: Pubkey = solana_sdk::pubkey!("11111111111111111111111111111111");

// System program instruction discriminators
const CREATE_ACCOUNT: u32 = 0;
const ASSIGN: u32 = 1;
const TRANSFER: u32 = 2;
const CREATE_ACCOUNT_WITH_SEED: u32 = 3;
const ADVANCE_NONCE_ACCOUNT: u32 = 4;
const ALLOCATE: u32 = 8;
const ALLOCATE_WITH_SEED: u32 = 9;

/// System program analyzer for SOL transfers and account operations
pub struct SystemProgramAnalyzer;

impl SystemProgramAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn parse_u32(data: &[u8], offset: usize) -> Option<u32> {
        if data.len() >= offset + 4 {
            Some(u32::from_le_bytes(
                data[offset..offset + 4].try_into().ok()?,
            ))
        } else {
            None
        }
    }

    fn parse_u64(data: &[u8], offset: usize) -> Option<u64> {
        if data.len() >= offset + 8 {
            Some(u64::from_le_bytes(
                data[offset..offset + 8].try_into().ok()?,
            ))
        } else {
            None
        }
    }

    fn analyze_system_instructions(tx: &Transaction) -> SystemStats {
        let mut stats = SystemStats::default();

        for instruction in &tx.message.instructions {
            if let Some(program_id) = tx
                .message
                .account_keys
                .get(instruction.program_id_index as usize)
            {
                if program_id != &SYSTEM_PROGRAM {
                    continue;
                }

                // System instructions use u32 discriminator
                if let Some(discriminator) = Self::parse_u32(&instruction.data, 0) {
                    match discriminator {
                        CREATE_ACCOUNT => {
                            stats.create_account_count += 1;
                            // Parse lamports (offset 4) and space (offset 12)
                            if let Some(lamports) = Self::parse_u64(&instruction.data, 4) {
                                stats.total_rent_required += lamports;
                            }
                        }
                        TRANSFER => {
                            stats.transfer_count += 1;
                            // Parse lamports (offset 4)
                            if let Some(lamports) = Self::parse_u64(&instruction.data, 4) {
                                stats.total_sol_transferred += lamports;
                                stats.max_sol_transfer = stats.max_sol_transfer.max(lamports);
                            }
                            // Extract recipient (accounts[1])
                            if let Some(&recipient_idx) = instruction.accounts.get(1) {
                                if let Some(recipient) =
                                    tx.message.account_keys.get(recipient_idx as usize)
                                {
                                    stats.sol_recipients.push(recipient.to_string());
                                }
                            }
                        }
                        ASSIGN => {
                            stats.assign_count += 1;
                        }
                        CREATE_ACCOUNT_WITH_SEED => {
                            stats.create_account_count += 1;
                        }
                        ADVANCE_NONCE_ACCOUNT => {
                            stats.uses_durable_nonce = true;
                            stats.advances_nonce = true;
                            // Extract nonce account (accounts[0])
                            if let Some(&nonce_idx) = instruction.accounts.first() {
                                if let Some(nonce_account) =
                                    tx.message.account_keys.get(nonce_idx as usize)
                                {
                                    stats.nonce_account = Some(nonce_account.to_string());
                                }
                            }
                        }
                        ALLOCATE | ALLOCATE_WITH_SEED => {
                            stats.allocate_count += 1;
                        }
                        _ => {
                            stats.unknown_system_instruction_count += 1;
                        }
                    }
                }
            }
        }

        stats
    }
}

#[derive(Default)]
struct SystemStats {
    create_account_count: usize,
    transfer_count: usize,
    assign_count: usize,
    allocate_count: usize,
    unknown_system_instruction_count: usize,

    total_sol_transferred: u64,
    max_sol_transfer: u64,
    total_rent_required: u64,
    uses_durable_nonce: bool,
    advances_nonce: bool,
    nonce_account: Option<String>,

    // Target addresses
    sol_recipients: Vec<String>,
}

#[async_trait::async_trait]
impl TransactionAnalyzer for SystemProgramAnalyzer {
    fn name(&self) -> &str {
        "system"
    }

    fn fields(&self) -> Vec<String> {
        vec![
            // SOL transfers
            "has_sol_transfer".to_string(),
            "sol_transfer_count".to_string(),
            "total_sol_transferred".to_string(),
            "max_sol_transfer".to_string(),
            "sol_recipients".to_string(),
            // Account operations
            "creates_accounts".to_string(),
            "account_creation_count".to_string(),
            "total_rent_required".to_string(),
            // Program assignment
            "assigns_program_ownership".to_string(),
            "assign_count".to_string(),
            // Advanced features
            "uses_durable_nonce".to_string(),
            "advances_nonce".to_string(),
            "nonce_account".to_string(),
            "allocate_count".to_string(),
            // Security indicators
            "high_rent_spam".to_string(),
            "large_sol_transfer".to_string(),
        ]
    }

    async fn analyze(&self, tx: &Transaction) -> Result<HashMap<String, Value>> {
        let stats = Self::analyze_system_instructions(tx);

        let mut fields = HashMap::new();

        // SOL transfers
        fields.insert(
            "has_sol_transfer".to_string(),
            json!(stats.transfer_count > 0),
        );
        fields.insert(
            "sol_transfer_count".to_string(),
            json!(stats.transfer_count),
        );
        fields.insert(
            "total_sol_transferred".to_string(),
            json!(stats.total_sol_transferred),
        );
        fields.insert(
            "max_sol_transfer".to_string(),
            json!(stats.max_sol_transfer),
        );

        // Account operations
        fields.insert(
            "creates_accounts".to_string(),
            json!(stats.create_account_count > 0),
        );
        fields.insert(
            "account_creation_count".to_string(),
            json!(stats.create_account_count),
        );
        fields.insert(
            "total_rent_required".to_string(),
            json!(stats.total_rent_required),
        );

        // Program assignment
        fields.insert(
            "assigns_program_ownership".to_string(),
            json!(stats.assign_count > 0),
        );
        fields.insert("assign_count".to_string(), json!(stats.assign_count));

        // Advanced features
        fields.insert(
            "uses_durable_nonce".to_string(),
            json!(stats.uses_durable_nonce),
        );
        fields.insert("advances_nonce".to_string(), json!(stats.advances_nonce));
        if let Some(nonce_account) = stats.nonce_account {
            fields.insert("nonce_account".to_string(), json!(nonce_account));
        }
        fields.insert("allocate_count".to_string(), json!(stats.allocate_count));

        // Security indicators
        let high_rent_spam = stats.create_account_count > 10;
        fields.insert("high_rent_spam".to_string(), json!(high_rent_spam));

        // Large SOL transfer (> 1 SOL = 1B lamports)
        let large_transfer = stats.max_sol_transfer > 1_000_000_000;
        fields.insert("large_sol_transfer".to_string(), json!(large_transfer));

        // SOL recipients
        fields.insert("sol_recipients".to_string(), json!(stats.sol_recipients));

        Ok(fields)
    }

    fn estimated_latency_ms(&self) -> u64 {
        1
    }
}

impl Default for SystemProgramAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
