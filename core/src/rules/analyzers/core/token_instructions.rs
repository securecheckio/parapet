use crate::rules::analyzer::TransactionAnalyzer;
use anyhow::Result;
use serde_json::{json, Value};
use solana_sdk::transaction::Transaction;
use std::collections::{HashMap, HashSet};

const SPL_TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
const ASSOCIATED_TOKEN_PROGRAM: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

// SPL Token instruction discriminators
const INITIALIZE_MINT: u8 = 0;
const INITIALIZE_ACCOUNT: u8 = 1;
const INITIALIZE_MULTISIG: u8 = 2;
const TRANSFER: u8 = 3;
const APPROVE: u8 = 4;
const REVOKE: u8 = 5;
const SET_AUTHORITY: u8 = 6;
const MINT_TO: u8 = 7;
const BURN: u8 = 8;
const CLOSE_ACCOUNT: u8 = 9;
const FREEZE_ACCOUNT: u8 = 10;
const THAW_ACCOUNT: u8 = 11;
const TRANSFER_CHECKED: u8 = 12;
const APPROVE_CHECKED: u8 = 13;
const MINT_TO_CHECKED: u8 = 14;
const BURN_CHECKED: u8 = 15;

/// Comprehensive token instruction analyzer
/// Detects all SPL Token and Token-2022 instruction types
pub struct TokenInstructionAnalyzer;

impl TokenInstructionAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn is_token_program(program_id: &str) -> bool {
        program_id == SPL_TOKEN_PROGRAM || program_id == TOKEN_2022_PROGRAM
    }

    fn parse_amount_from_instruction(data: &[u8], offset: usize) -> Option<u64> {
        if data.len() >= offset + 8 {
            Some(u64::from_le_bytes(
                data[offset..offset + 8].try_into().ok()?,
            ))
        } else {
            None
        }
    }

    fn is_signer(tx: &Transaction, account_idx: u8) -> bool {
        tx.message.is_signer(account_idx as usize)
    }

    fn analyze_instructions(tx: &Transaction) -> InstructionStats {
        let mut stats = InstructionStats::default();

        // First pass: track account creation and ownership
        for instruction in &tx.message.instructions {
            if let Some(program_id) = tx
                .message
                .account_keys
                .get(instruction.program_id_index as usize)
            {
                let prog_str = program_id.to_string();

                // Track Associated Token Account creation
                if prog_str == ASSOCIATED_TOKEN_PROGRAM {
                    // ATA Create: accounts[1] is the token account, accounts[0] is the wallet owner
                    if let (Some(&account_idx), Some(&owner_idx)) =
                        (instruction.accounts.first(), instruction.accounts.get(1))
                    {
                        if let (Some(account), Some(owner)) = (
                            tx.message.account_keys.get(account_idx as usize),
                            tx.message.account_keys.get(owner_idx as usize),
                        ) {
                            let account_str = account.to_string();
                            let owner_str = owner.to_string();
                            stats
                                .newly_created_token_accounts
                                .insert(account_str.clone());
                            stats
                                .token_account_owners
                                .insert(account_str, owner_str.clone());

                            // Check if owner is a signer (fee payer is always first signer)
                            let fee_payer = tx.message.account_keys.first().map(|k| k.to_string());
                            if Some(owner_str.clone()) != fee_payer {
                                stats.creates_account_for_other = true;
                            }
                        }
                    }
                }

                // Track token InitializeAccount instructions
                if Self::is_token_program(&prog_str) {
                    if let Some(&discriminator) = instruction.data.first() {
                        if discriminator == INITIALIZE_ACCOUNT {
                            // InitializeAccount: accounts[0] is the account, accounts[3] is the owner
                            if let (Some(&account_idx), Some(&owner_idx)) =
                                (instruction.accounts.first(), instruction.accounts.get(3))
                            {
                                if let (Some(account), Some(owner)) = (
                                    tx.message.account_keys.get(account_idx as usize),
                                    tx.message.account_keys.get(owner_idx as usize),
                                ) {
                                    let account_str = account.to_string();
                                    let owner_str = owner.to_string();
                                    stats
                                        .newly_created_token_accounts
                                        .insert(account_str.clone());
                                    stats
                                        .token_account_owners
                                        .insert(account_str, owner_str.clone());

                                    let fee_payer =
                                        tx.message.account_keys.first().map(|k| k.to_string());
                                    if Some(owner_str) != fee_payer {
                                        stats.creates_account_for_other = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Second pass: analyze instructions
        for instruction in &tx.message.instructions {
            if let Some(program_id) = tx
                .message
                .account_keys
                .get(instruction.program_id_index as usize)
            {
                let prog_str = program_id.to_string();

                if !Self::is_token_program(&prog_str) {
                    continue;
                }

                if let Some(&discriminator) = instruction.data.first() {
                    match discriminator {
                        TRANSFER => {
                            stats.transfer_count += 1;
                            if let Some(amount) =
                                Self::parse_amount_from_instruction(&instruction.data, 1)
                            {
                                stats.total_transfer_amount += amount;
                                stats.max_transfer_amount = stats.max_transfer_amount.max(amount);
                            }
                            // Extract destination (accounts[1])
                            if let Some(&dest_idx) = instruction.accounts.get(1) {
                                if let Some(dest) = tx.message.account_keys.get(dest_idx as usize) {
                                    let dest_str = dest.to_string();
                                    stats.transfer_recipients.push(dest_str.clone());

                                    // Check if transferring to newly created account
                                    if stats.newly_created_token_accounts.contains(&dest_str) {
                                        stats.transfers_to_newly_created = true;
                                    }
                                }
                            }
                            // Check ownership: source owner is accounts[2]
                            if let Some(&owner_idx) = instruction.accounts.get(2) {
                                if !Self::is_signer(tx, owner_idx) {
                                    stats.non_owned_transfer_count += 1;
                                }
                            }
                        }
                        TRANSFER_CHECKED => {
                            stats.transfer_count += 1;
                            stats.transfer_checked_count += 1;
                            if let Some(amount) =
                                Self::parse_amount_from_instruction(&instruction.data, 1)
                            {
                                stats.total_transfer_amount += amount;
                                stats.max_transfer_amount = stats.max_transfer_amount.max(amount);
                            }
                            // Extract destination (accounts[1])
                            if let Some(&dest_idx) = instruction.accounts.get(1) {
                                if let Some(dest) = tx.message.account_keys.get(dest_idx as usize) {
                                    let dest_str = dest.to_string();
                                    stats.transfer_recipients.push(dest_str.clone());

                                    // Check if transferring to newly created account
                                    if stats.newly_created_token_accounts.contains(&dest_str) {
                                        stats.transfers_to_newly_created = true;
                                    }
                                }
                            }
                            // Extract mint address (accounts[2])
                            if let Some(&mint_idx) = instruction.accounts.get(2) {
                                if let Some(mint) = tx.message.account_keys.get(mint_idx as usize) {
                                    stats.mint_addresses.insert(mint.to_string());
                                }
                            }
                            // Check ownership: source owner is accounts[3]
                            if let Some(&owner_idx) = instruction.accounts.get(3) {
                                if !Self::is_signer(tx, owner_idx) {
                                    stats.non_owned_transfer_count += 1;
                                }
                            }
                        }
                        APPROVE => {
                            stats.approve_count += 1;
                            if let Some(amount) =
                                Self::parse_amount_from_instruction(&instruction.data, 1)
                            {
                                if amount == u64::MAX {
                                    stats.unlimited_approve_count += 1;
                                }
                                stats.total_approve_amount += amount;
                            }
                            // Extract delegate address (accounts[1])
                            if let Some(&delegate_idx) = instruction.accounts.get(1) {
                                if let Some(delegate) =
                                    tx.message.account_keys.get(delegate_idx as usize)
                                {
                                    stats.delegate_addresses.push(delegate.to_string());
                                }
                            }
                            // Check ownership: source owner is accounts[2]
                            if let Some(&owner_idx) = instruction.accounts.get(2) {
                                if !Self::is_signer(tx, owner_idx) {
                                    stats.non_owned_approve_count += 1;
                                }
                            }
                        }
                        APPROVE_CHECKED => {
                            stats.approve_count += 1;
                            if let Some(amount) =
                                Self::parse_amount_from_instruction(&instruction.data, 1)
                            {
                                if amount == u64::MAX {
                                    stats.unlimited_approve_count += 1;
                                }
                                stats.total_approve_amount += amount;
                            }
                            // Extract delegate address (accounts[1])
                            if let Some(&delegate_idx) = instruction.accounts.get(1) {
                                if let Some(delegate) =
                                    tx.message.account_keys.get(delegate_idx as usize)
                                {
                                    stats.delegate_addresses.push(delegate.to_string());
                                }
                            }
                            // Extract mint address (accounts[2])
                            if let Some(&mint_idx) = instruction.accounts.get(2) {
                                if let Some(mint) = tx.message.account_keys.get(mint_idx as usize) {
                                    stats.mint_addresses.insert(mint.to_string());
                                }
                            }
                            // Check ownership: source owner is accounts[3]
                            if let Some(&owner_idx) = instruction.accounts.get(3) {
                                if !Self::is_signer(tx, owner_idx) {
                                    stats.non_owned_approve_count += 1;
                                }
                            }
                        }
                        REVOKE => {
                            stats.revoke_count += 1;
                        }
                        MINT_TO | MINT_TO_CHECKED => {
                            stats.mint_count += 1;
                            if let Some(amount) =
                                Self::parse_amount_from_instruction(&instruction.data, 1)
                            {
                                stats.total_mint_amount += amount;
                            }
                        }
                        BURN | BURN_CHECKED => {
                            stats.burn_count += 1;
                            if let Some(amount) =
                                Self::parse_amount_from_instruction(&instruction.data, 1)
                            {
                                stats.total_burn_amount += amount;
                            }
                        }
                        FREEZE_ACCOUNT => {
                            stats.freeze_count += 1;
                        }
                        THAW_ACCOUNT => {
                            stats.thaw_count += 1;
                        }
                        CLOSE_ACCOUNT => {
                            stats.close_count += 1;
                            // Check ownership: account owner is accounts[2]
                            if let Some(&owner_idx) = instruction.accounts.get(2) {
                                if !Self::is_signer(tx, owner_idx) {
                                    stats.non_owned_close_count += 1;
                                }
                            }
                        }
                        SET_AUTHORITY => {
                            stats.set_authority_count += 1;
                            // Check ownership: current authority is accounts[1]
                            if let Some(&authority_idx) = instruction.accounts.get(1) {
                                if !Self::is_signer(tx, authority_idx) {
                                    stats.non_owned_authority_change_count += 1;
                                }
                            }
                        }
                        INITIALIZE_MINT => {
                            stats.initialize_mint_count += 1;
                        }
                        INITIALIZE_ACCOUNT => {
                            stats.initialize_account_count += 1;
                        }
                        INITIALIZE_MULTISIG => {
                            stats.initialize_multisig_count += 1;
                        }
                        _ => {
                            stats.unknown_instruction_count += 1;
                        }
                    }
                }
            }
        }

        stats
    }
}

#[derive(Default)]
struct InstructionStats {
    transfer_count: usize,
    approve_count: usize,
    revoke_count: usize,
    mint_count: usize,
    burn_count: usize,
    freeze_count: usize,
    thaw_count: usize,
    close_count: usize,
    set_authority_count: usize,
    initialize_mint_count: usize,
    initialize_account_count: usize,
    initialize_multisig_count: usize,
    unknown_instruction_count: usize,

    unlimited_approve_count: usize,
    total_transfer_amount: u64,
    max_transfer_amount: u64,
    total_approve_amount: u64,
    total_mint_amount: u64,
    total_burn_amount: u64,

    // Target addresses
    delegate_addresses: Vec<String>,
    transfer_recipients: Vec<String>,
    mint_addresses: HashSet<String>,

    // Ownership tracking (non-owned operations)
    non_owned_close_count: usize,
    non_owned_transfer_count: usize,
    non_owned_authority_change_count: usize,
    non_owned_approve_count: usize,

    // TransferChecked usage
    transfer_checked_count: usize,

    // Wallet drainer detection
    newly_created_token_accounts: HashSet<String>,
    token_account_owners: HashMap<String, String>,
    creates_account_for_other: bool,
    transfers_to_newly_created: bool,
}

#[async_trait::async_trait]
impl TransactionAnalyzer for TokenInstructionAnalyzer {
    fn name(&self) -> &str {
        "token_instructions"
    }

    fn fields(&self) -> Vec<String> {
        vec![
            // Instruction presence checks
            "has_transfer".to_string(),
            "has_approve".to_string(),
            "has_revoke".to_string(),
            "has_mint".to_string(),
            "has_burn".to_string(),
            "has_freeze".to_string(),
            "has_thaw".to_string(),
            "has_close_account".to_string(),
            "has_set_authority".to_string(),
            "has_initialize_mint".to_string(),
            "has_initialize_account".to_string(),
            // Counts
            "transfer_count".to_string(),
            "approve_count".to_string(),
            "revoke_count".to_string(),
            "mint_count".to_string(),
            "burn_count".to_string(),
            "freeze_count".to_string(),
            "thaw_count".to_string(),
            "close_count".to_string(),
            "set_authority_count".to_string(),
            // Amounts
            "total_transfer_amount".to_string(),
            "max_transfer_amount".to_string(),
            "total_mint_amount".to_string(),
            "total_burn_amount".to_string(),
            // Target addresses
            "delegate_addresses".to_string(),
            "transfer_recipients".to_string(),
            "mints_involved".to_string(),
            // Ownership checks (non-owned operations)
            "closes_non_owned_account".to_string(),
            "transfers_from_non_owned".to_string(),
            "modifies_non_owned_authority".to_string(),
            "approves_non_owned_tokens".to_string(),
            "non_owned_operations_count".to_string(),
            // TransferChecked usage
            "transfer_checked_count".to_string(),
            "uses_transfer_checked".to_string(),
            // Risk indicators
            "unlimited_approve_count".to_string(),
            "net_delegation_change".to_string(),
            "dangerous_operation_combo".to_string(),
            // Summary flags
            "modifies_supply".to_string(),
            "modifies_permissions".to_string(),
            "account_management_detected".to_string(),
            // Wallet drainer detection
            "creates_account_for_other".to_string(),
            "transfers_to_newly_created".to_string(),
            "newly_created_account_owners".to_string(),
            "new_account_owner_address".to_string(),
        ]
    }

    async fn analyze(&self, tx: &Transaction) -> Result<HashMap<String, Value>> {
        let stats = Self::analyze_instructions(tx);

        let mut fields = HashMap::new();

        // Instruction presence
        fields.insert("has_transfer".to_string(), json!(stats.transfer_count > 0));
        fields.insert("has_approve".to_string(), json!(stats.approve_count > 0));
        fields.insert("has_revoke".to_string(), json!(stats.revoke_count > 0));
        fields.insert("has_mint".to_string(), json!(stats.mint_count > 0));
        fields.insert("has_burn".to_string(), json!(stats.burn_count > 0));
        fields.insert("has_freeze".to_string(), json!(stats.freeze_count > 0));
        fields.insert("has_thaw".to_string(), json!(stats.thaw_count > 0));
        fields.insert(
            "has_close_account".to_string(),
            json!(stats.close_count > 0),
        );
        fields.insert(
            "has_set_authority".to_string(),
            json!(stats.set_authority_count > 0),
        );
        fields.insert(
            "has_initialize_mint".to_string(),
            json!(stats.initialize_mint_count > 0),
        );
        fields.insert(
            "has_initialize_account".to_string(),
            json!(stats.initialize_account_count > 0),
        );

        // Counts
        fields.insert("transfer_count".to_string(), json!(stats.transfer_count));
        fields.insert("approve_count".to_string(), json!(stats.approve_count));
        fields.insert("revoke_count".to_string(), json!(stats.revoke_count));
        fields.insert("mint_count".to_string(), json!(stats.mint_count));
        fields.insert("burn_count".to_string(), json!(stats.burn_count));
        fields.insert("freeze_count".to_string(), json!(stats.freeze_count));
        fields.insert("thaw_count".to_string(), json!(stats.thaw_count));
        fields.insert("close_count".to_string(), json!(stats.close_count));
        fields.insert(
            "set_authority_count".to_string(),
            json!(stats.set_authority_count),
        );

        // Amounts
        fields.insert(
            "total_transfer_amount".to_string(),
            json!(stats.total_transfer_amount),
        );
        fields.insert(
            "max_transfer_amount".to_string(),
            json!(stats.max_transfer_amount),
        );
        fields.insert(
            "total_mint_amount".to_string(),
            json!(stats.total_mint_amount),
        );
        fields.insert(
            "total_burn_amount".to_string(),
            json!(stats.total_burn_amount),
        );

        // Risk indicators
        fields.insert(
            "unlimited_approve_count".to_string(),
            json!(stats.unlimited_approve_count),
        );

        // Net delegation change = approvals - revokes
        let net_delegation = stats.approve_count as i32 - stats.revoke_count as i32;
        fields.insert("net_delegation_change".to_string(), json!(net_delegation));

        // Dangerous combinations
        let dangerous_combo = ((stats.freeze_count > 0 || stats.burn_count > 0)
            && stats.approve_count > 0)
            || (stats.mint_count > 0 && stats.set_authority_count > 0); // Mint + Authority change
        fields.insert(
            "dangerous_operation_combo".to_string(),
            json!(dangerous_combo),
        );

        // Summary flags
        fields.insert(
            "modifies_supply".to_string(),
            json!(stats.mint_count > 0 || stats.burn_count > 0),
        );

        fields.insert(
            "modifies_permissions".to_string(),
            json!(
                stats.approve_count > 0 || stats.set_authority_count > 0 || stats.freeze_count > 0
            ),
        );

        fields.insert(
            "account_management_detected".to_string(),
            json!(stats.close_count > 0 || stats.initialize_account_count > 0),
        );

        // Target addresses
        fields.insert(
            "delegate_addresses".to_string(),
            json!(stats.delegate_addresses),
        );
        fields.insert(
            "transfer_recipients".to_string(),
            json!(stats.transfer_recipients),
        );
        let mints: Vec<String> = stats.mint_addresses.iter().cloned().collect();
        fields.insert("mints_involved".to_string(), json!(mints));

        // Ownership checks
        let total_non_owned = stats.non_owned_close_count
            + stats.non_owned_transfer_count
            + stats.non_owned_authority_change_count
            + stats.non_owned_approve_count;
        fields.insert(
            "closes_non_owned_account".to_string(),
            json!(stats.non_owned_close_count > 0),
        );
        fields.insert(
            "transfers_from_non_owned".to_string(),
            json!(stats.non_owned_transfer_count > 0),
        );
        fields.insert(
            "modifies_non_owned_authority".to_string(),
            json!(stats.non_owned_authority_change_count > 0),
        );
        fields.insert(
            "approves_non_owned_tokens".to_string(),
            json!(stats.non_owned_approve_count > 0),
        );
        fields.insert(
            "non_owned_operations_count".to_string(),
            json!(total_non_owned),
        );

        // TransferChecked usage
        fields.insert(
            "transfer_checked_count".to_string(),
            json!(stats.transfer_checked_count),
        );
        fields.insert(
            "uses_transfer_checked".to_string(),
            json!(stats.transfer_checked_count > 0),
        );

        // Wallet drainer detection
        fields.insert(
            "creates_account_for_other".to_string(),
            json!(stats.creates_account_for_other),
        );
        fields.insert(
            "transfers_to_newly_created".to_string(),
            json!(stats.transfers_to_newly_created),
        );

        // Extract unique owners of newly created accounts
        let mut newly_created_owners: Vec<String> = stats
            .newly_created_token_accounts
            .iter()
            .filter_map(|account| stats.token_account_owners.get(account))
            .cloned()
            .collect();
        newly_created_owners.sort();
        newly_created_owners.dedup();

        fields.insert(
            "newly_created_account_owners".to_string(),
            json!(newly_created_owners.clone()),
        );

        // Helper field: first newly created account owner (for rule matching)
        fields.insert(
            "new_account_owner_address".to_string(),
            json!(newly_created_owners.first().cloned().unwrap_or_default()),
        );

        Ok(fields)
    }

    fn estimated_latency_ms(&self) -> u64 {
        1
    }
}

impl Default for TokenInstructionAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
