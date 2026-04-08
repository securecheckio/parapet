use crate::rules::analyzer::TransactionAnalyzer;
use anyhow::Result;
use serde_json::{json, Value};
use solana_sdk::transaction::Transaction;
use std::collections::{HashMap, HashSet};

/// Core Solana system programs (always safe)
fn get_core_programs() -> HashSet<String> {
    let mut core = HashSet::new();

    // Core Solana programs only
    core.insert("11111111111111111111111111111111".to_string()); // System
    core.insert("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string()); // SPL Token
    core.insert("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb".to_string()); // Token-2022
    core.insert("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL".to_string()); // Associated Token
    core.insert("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr".to_string()); // Memo
    core.insert("ComputeBudget111111111111111111111111111111".to_string()); // Compute Budget
    core.insert("Stake11111111111111111111111111111111111111".to_string()); // Stake
    core.insert("Vote111111111111111111111111111111111111111".to_string()); // Vote

    core
}

/// Analyzes program complexity and interaction patterns
pub struct ProgramComplexityAnalyzer;

impl ProgramComplexityAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn categorize_programs(tx: &Transaction) -> ProgramCategories {
        let core_programs = get_core_programs();
        let mut categories = ProgramCategories::default();

        let unique_programs: HashSet<String> = tx
            .message
            .instructions
            .iter()
            .filter_map(|inst| {
                tx.message
                    .account_keys
                    .get(inst.program_id_index as usize)
                    .map(|pk| pk.to_string())
            })
            .collect();

        for program in &unique_programs {
            if core_programs.contains(program) {
                categories.core_programs.push(program.clone());
            } else {
                categories.non_core_programs.push(program.clone());
            }

            // Detect core program types only
            match program.as_str() {
                "11111111111111111111111111111111" => {
                    categories.uses_system_program = true;
                }
                "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
                | "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb" => {
                    categories.uses_token_program = true;
                }
                _ => {}
            }
        }

        categories.total_programs = unique_programs.len();
        categories.core_program_count = categories.core_programs.len();
        categories.non_core_program_count = categories.non_core_programs.len();

        categories
    }

    fn calculate_complexity_score(tx: &Transaction, categories: &ProgramCategories) -> u8 {
        let mut score: u16 = 0;

        // Base complexity from instruction count (0-30 points)
        let instruction_score = (tx.message.instructions.len() * 2).min(30);
        score += instruction_score as u16;

        // Non-core programs (0-40 points)
        let non_core_score = (categories.non_core_program_count * 10).min(40);
        score += non_core_score as u16;

        // Multiple programs (0-20 points)
        if categories.total_programs > 3 {
            let program_score = (10 * (categories.total_programs - 3)).min(20);
            score += program_score as u16;
        }

        // Account complexity (0-10 points)
        let account_score = (tx.message.account_keys.len() / 5).min(10);
        score += account_score as u16;

        // Cap at 100
        score.min(100) as u8
    }

    fn calculate_writable_non_signers(tx: &Transaction) -> usize {
        let total_accounts = tx.message.account_keys.len();
        let num_signers = tx.message.header.num_required_signatures as usize;
        let readonly_signed = tx.message.header.num_readonly_signed_accounts as usize;
        let readonly_unsigned = tx.message.header.num_readonly_unsigned_accounts as usize;

        // Writable accounts = total - readonly
        let writable_total = total_accounts
            .saturating_sub(readonly_signed)
            .saturating_sub(readonly_unsigned);

        // Writable non-signers = writable - signers
        writable_total.saturating_sub(num_signers)
    }
}

#[derive(Default)]
struct ProgramCategories {
    total_programs: usize,
    core_programs: Vec<String>,
    non_core_programs: Vec<String>,
    core_program_count: usize,
    non_core_program_count: usize,

    uses_system_program: bool,
    uses_token_program: bool,
}

#[async_trait::async_trait]
impl TransactionAnalyzer for ProgramComplexityAnalyzer {
    fn name(&self) -> &str {
        "complexity"
    }

    fn fields(&self) -> Vec<String> {
        vec![
            // Program categorization
            "unique_program_count".to_string(),
            "core_program_count".to_string(),
            "non_core_program_count".to_string(),
            "core_programs".to_string(),
            "non_core_programs".to_string(),
            "all_programs".to_string(),
            // Program type detection
            "uses_system_program".to_string(),
            "uses_token_program".to_string(),
            // Complexity scoring
            "complexity_score".to_string(),
            "is_complex_transaction".to_string(),
            // Account validation risks
            "writable_non_signer_count".to_string(),
            "potential_authority_mismatch".to_string(),
        ]
    }

    async fn analyze(&self, tx: &Transaction) -> Result<HashMap<String, Value>> {
        let categories = Self::categorize_programs(tx);
        let complexity_score = Self::calculate_complexity_score(tx, &categories);
        let writable_non_signers = Self::calculate_writable_non_signers(tx);

        // Get all unique program IDs
        let all_programs: Vec<String> = tx
            .message
            .instructions
            .iter()
            .filter_map(|inst| {
                tx.message
                    .account_keys
                    .get(inst.program_id_index as usize)
                    .map(|pk| pk.to_string())
            })
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        let mut fields = HashMap::new();

        // Program categorization
        fields.insert(
            "unique_program_count".to_string(),
            json!(categories.total_programs),
        );
        fields.insert(
            "core_program_count".to_string(),
            json!(categories.core_program_count),
        );
        fields.insert(
            "non_core_program_count".to_string(),
            json!(categories.non_core_program_count),
        );
        fields.insert("core_programs".to_string(), json!(categories.core_programs));
        fields.insert(
            "non_core_programs".to_string(),
            json!(categories.non_core_programs),
        );
        fields.insert("all_programs".to_string(), json!(all_programs));

        // Program type detection
        fields.insert(
            "uses_system_program".to_string(),
            json!(categories.uses_system_program),
        );
        fields.insert(
            "uses_token_program".to_string(),
            json!(categories.uses_token_program),
        );

        // Complexity scoring
        fields.insert("complexity_score".to_string(), json!(complexity_score));
        fields.insert(
            "is_complex_transaction".to_string(),
            json!(complexity_score > 60),
        );

        // Account validation risks
        fields.insert(
            "writable_non_signer_count".to_string(),
            json!(writable_non_signers),
        );

        // Potential authority mismatch (writable accounts that aren't signers)
        let potential_mismatch = writable_non_signers > 5;
        fields.insert(
            "potential_authority_mismatch".to_string(),
            json!(potential_mismatch),
        );

        Ok(fields)
    }

    fn estimated_latency_ms(&self) -> u64 {
        1
    }
}

impl Default for ProgramComplexityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
