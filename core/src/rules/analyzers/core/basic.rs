use crate::rules::analyzer::TransactionAnalyzer;
use anyhow::Result;
use serde_json::{json, Value};
use solana_sdk::transaction::Transaction;
use std::collections::HashMap;

/// Basic analyzer provides simple transaction fields
pub struct BasicAnalyzer;

impl BasicAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn extract_amount(tx: &Transaction) -> u64 {
        // Try to extract transfer amount from instruction data
        // This is a simplified version - real implementation would parse specific instruction types
        tx.message
            .instructions
            .first()
            .and_then(|inst| {
                if inst.data.len() >= 8 {
                    Some(u64::from_le_bytes(inst.data[0..8].try_into().ok()?))
                } else {
                    None
                }
            })
            .unwrap_or(0)
    }
}

#[async_trait::async_trait]
impl TransactionAnalyzer for BasicAnalyzer {
    fn name(&self) -> &str {
        "basic"
    }

    fn fields(&self) -> Vec<String> {
        vec![
            "instruction_count".to_string(),
            "account_keys_count".to_string(),
            "writable_accounts_count".to_string(),
            "signers_count".to_string(),
            "amount".to_string(),
            "has_instructions".to_string(),
            "program_ids".to_string(),
        ]
    }

    async fn analyze(&self, tx: &Transaction) -> Result<HashMap<String, Value>> {
        let writable_count = tx.message.header.num_required_signatures as usize
            + tx.message.header.num_readonly_signed_accounts as usize;

        // Extract unique program IDs from instructions
        let program_ids: Vec<String> = tx
            .message
            .instructions
            .iter()
            .filter_map(|inst| {
                tx.message
                    .account_keys
                    .get(inst.program_id_index as usize)
                    .map(|pk| pk.to_string())
            })
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let mut fields = HashMap::new();

        fields.insert(
            "instruction_count".to_string(),
            json!(tx.message.instructions.len()),
        );

        fields.insert(
            "account_keys_count".to_string(),
            json!(tx.message.account_keys.len()),
        );

        fields.insert("writable_accounts_count".to_string(), json!(writable_count));

        fields.insert(
            "signers_count".to_string(),
            json!(tx.message.header.num_required_signatures),
        );

        fields.insert("amount".to_string(), json!(Self::extract_amount(tx)));

        fields.insert(
            "has_instructions".to_string(),
            json!(!tx.message.instructions.is_empty()),
        );

        fields.insert("program_ids".to_string(), json!(program_ids));

        Ok(fields)
    }

    fn estimated_latency_ms(&self) -> u64 {
        1
    }
}

impl Default for BasicAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
