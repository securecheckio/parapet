use crate::rules::analyzer::TransactionAnalyzer;
use anyhow::Result;
use serde_json::{json, Value};
use solana_sdk::{pubkey::Pubkey, transaction::Transaction};
use std::collections::{HashMap, HashSet};

const SPL_TOKEN_PROGRAM: Pubkey =
    solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const TOKEN_2022_PROGRAM: Pubkey =
    solana_sdk::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
const APPROVE_INSTRUCTION: u8 = 4;
const APPROVE_CHECKED_INSTRUCTION: u8 = 13;
const SET_AUTHORITY_INSTRUCTION: u8 = 6;
const CLOSE_ACCOUNT_INSTRUCTION: u8 = 9;

pub struct CoreSecurityAnalyzer {
    blocklist: HashSet<String>,
    max_instructions: usize,
    max_signers: u8,
    max_writable_accounts: usize,
}

impl CoreSecurityAnalyzer {
    pub fn new(blocklist: HashSet<String>) -> Self {
        Self {
            blocklist,
            max_instructions: 20,
            max_signers: 5,
            max_writable_accounts: 15,
        }
    }

    fn detect_delegation(&self, tx: &Transaction) -> (bool, bool, usize) {
        let mut approval_count = 0;
        let mut is_unlimited = false;
        let mut has_delegation = false;

        for instruction in &tx.message.instructions {
            if let Some(program_id) = tx
                .message
                .account_keys
                .get(instruction.program_id_index as usize)
            {
                if program_id == &SPL_TOKEN_PROGRAM || program_id == &TOKEN_2022_PROGRAM {
                    if let Some(&discriminator) = instruction.data.first() {
                        if discriminator == APPROVE_INSTRUCTION
                            || discriminator == APPROVE_CHECKED_INSTRUCTION
                        {
                            approval_count += 1;
                            has_delegation = true;

                            if instruction.data.len() >= 9 {
                                let amount = u64::from_le_bytes(
                                    instruction.data[1..9].try_into().unwrap_or([0u8; 8]),
                                );
                                if amount == u64::MAX {
                                    is_unlimited = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        (has_delegation, is_unlimited, approval_count)
    }

    fn detect_authority_changes(&self, tx: &Transaction) -> bool {
        for instruction in &tx.message.instructions {
            if let Some(program_id) = tx
                .message
                .account_keys
                .get(instruction.program_id_index as usize)
            {
                if program_id == &SPL_TOKEN_PROGRAM || program_id == &TOKEN_2022_PROGRAM {
                    if let Some(&discriminator) = instruction.data.first() {
                        if discriminator == SET_AUTHORITY_INSTRUCTION
                            || discriminator == CLOSE_ACCOUNT_INSTRUCTION
                        {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn check_blocklist(&self, tx: &Transaction) -> Vec<String> {
        let mut blocked = Vec::new();

        for account_key in &tx.message.account_keys {
            let program_id_str = account_key.to_string();
            if self.blocklist.contains(&program_id_str) {
                blocked.push(program_id_str);
            }
        }

        blocked
    }

    fn analyze_patterns(&self, tx: &Transaction) -> Vec<String> {
        let mut issues = Vec::new();

        if tx.message.instructions.len() > self.max_instructions {
            issues.push(format!(
                "Transaction contains {} instructions (unusually high)",
                tx.message.instructions.len()
            ));
        }

        let signer_count = tx.message.header.num_required_signatures;
        if signer_count > self.max_signers {
            issues.push(format!(
                "Transaction requires {} signatures (unusually high)",
                signer_count
            ));
        }

        let writable_count = tx
            .message
            .account_keys
            .len()
            .saturating_sub(tx.message.header.num_readonly_signed_accounts as usize)
            .saturating_sub(tx.message.header.num_readonly_unsigned_accounts as usize);
        if writable_count > self.max_writable_accounts {
            issues.push(format!(
                "Transaction modifies {} accounts (unusually high)",
                writable_count
            ));
        }

        let unique_programs: HashSet<_> = tx
            .message
            .instructions
            .iter()
            .filter_map(|ix| tx.message.account_keys.get(ix.program_id_index as usize))
            .collect();
        if unique_programs.len() > 5 {
            issues.push(format!(
                "Transaction interacts with {} different programs",
                unique_programs.len()
            ));
        }

        for instruction in &tx.message.instructions {
            if instruction.data.len() > 1024 {
                issues.push(
                    "Transaction contains large instruction data (potential exploit)".to_string(),
                );
                break;
            }
        }

        issues
    }

    fn calculate_risk_score(
        &self,
        blocked: &[String],
        has_delegation: bool,
        is_unlimited: bool,
        approval_count: usize,
        has_authority_changes: bool,
        pattern_issues: &[String],
    ) -> u8 {
        let mut risk_score: u8 = 0;

        if !blocked.is_empty() {
            return 100;
        }

        if is_unlimited {
            risk_score = risk_score.max(95);
        } else if approval_count >= 3 {
            risk_score = risk_score.max(85);
        } else if has_delegation {
            risk_score = risk_score.max(30);
        }

        if has_authority_changes {
            let base_score = if has_delegation { 80 } else { 40 };
            risk_score = risk_score.max(base_score);
        }

        if !pattern_issues.is_empty() {
            let pattern_score = (pattern_issues.len() as u8 * 10).min(30);
            risk_score = risk_score.saturating_add(pattern_score);
        }

        risk_score
    }
}

#[async_trait::async_trait]
impl TransactionAnalyzer for CoreSecurityAnalyzer {
    fn name(&self) -> &str {
        "core_security"
    }

    fn fields(&self) -> Vec<String> {
        vec![
            "risk_score".to_string(),
            "risk_level".to_string(),
            "delegation_detected".to_string(),
            "delegation_is_unlimited".to_string(),
            "delegation_count".to_string(),
            "authority_changes".to_string(),
            "blocked_program_detected".to_string(),
            "blocked_program_count".to_string(),
            "has_issues".to_string(),
            "issue_count".to_string(),
        ]
    }

    async fn analyze(&self, tx: &Transaction) -> Result<HashMap<String, Value>> {
        let (has_delegation, is_unlimited, approval_count) = self.detect_delegation(tx);
        let has_authority_changes = self.detect_authority_changes(tx);
        let blocked = self.check_blocklist(tx);
        let pattern_issues = self.analyze_patterns(tx);

        let risk_score = self.calculate_risk_score(
            &blocked,
            has_delegation,
            is_unlimited,
            approval_count,
            has_authority_changes,
            &pattern_issues,
        );

        let risk_level = match risk_score {
            0..=25 => "low",
            26..=50 => "medium",
            51..=75 => "high",
            _ => "critical",
        };

        let mut fields = HashMap::new();

        fields.insert("risk_score".to_string(), json!(risk_score));
        fields.insert("risk_level".to_string(), json!(risk_level));
        fields.insert("delegation_detected".to_string(), json!(has_delegation));
        fields.insert("delegation_is_unlimited".to_string(), json!(is_unlimited));
        fields.insert("delegation_count".to_string(), json!(approval_count));
        fields.insert(
            "authority_changes".to_string(),
            json!(has_authority_changes),
        );
        fields.insert(
            "blocked_program_detected".to_string(),
            json!(!blocked.is_empty()),
        );
        fields.insert("blocked_program_count".to_string(), json!(blocked.len()));
        fields.insert("has_issues".to_string(), json!(!pattern_issues.is_empty()));
        fields.insert("issue_count".to_string(), json!(pattern_issues.len()));

        Ok(fields)
    }

    fn estimated_latency_ms(&self) -> u64 {
        2
    }
}
