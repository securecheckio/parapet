use crate::rules::analyzer::TransactionAnalyzer;
use anyhow::Result;
use serde_json::{json, Value};
use solana_sdk::transaction::Transaction;
use std::collections::HashMap;

/// Analyzes transaction account structure and validates account permissions.
///
/// Focuses on detecting potential authority mismatches and writable account
/// patterns that could indicate security issues.
pub struct AccountsAnalyzer;

impl AccountsAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Count writable accounts that are not signers.
    ///
    /// Writable non-signers can indicate:
    /// - PDAs (Program Derived Addresses) being modified
    /// - Token accounts being transferred
    /// - Potential authority confusion attacks
    fn calculate_writable_non_signers(tx: &Transaction) -> usize {
        let total_accounts = tx.message.account_keys.len();
        let num_signers = tx.message.header.num_required_signatures as usize;
        let _readonly_signed = tx.message.header.num_readonly_signed_accounts as usize;
        let readonly_unsigned = tx.message.header.num_readonly_unsigned_accounts as usize;

        // Calculate writable accounts
        // writable_signed would be num_signers - readonly_signed but we only need writable_unsigned
        let unsigned_accounts = total_accounts.saturating_sub(num_signers);

        unsigned_accounts.saturating_sub(readonly_unsigned)
    }
}

impl Default for AccountsAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TransactionAnalyzer for AccountsAnalyzer {
    fn name(&self) -> &str {
        "accounts"
    }

    fn fields(&self) -> Vec<String> {
        vec![
            "writable_non_signer_count".to_string(),
            "potential_authority_mismatch".to_string(),
        ]
    }

    async fn analyze(&self, tx: &Transaction) -> Result<HashMap<String, Value>> {
        let writable_non_signers = Self::calculate_writable_non_signers(tx);
        let mut fields = HashMap::new();

        fields.insert(
            "writable_non_signer_count".to_string(),
            json!(writable_non_signers),
        );

        // Potential authority mismatch if many writable non-signers
        // Threshold of 5 is heuristic - high count suggests complex authority patterns
        let potential_mismatch = writable_non_signers > 5;
        fields.insert(
            "potential_authority_mismatch".to_string(),
            json!(potential_mismatch),
        );

        Ok(fields)
    }

    fn estimated_latency_ms(&self) -> u64 {
        1 // Very fast, just header inspection
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{message::Message, pubkey::Pubkey};
    use solana_system_interface::instruction as system_instruction;

    #[tokio::test]
    async fn test_accounts_analyzer_basic() {
        let analyzer = AccountsAnalyzer::new();
        let from = Pubkey::new_unique();
        let to = Pubkey::new_unique();
        // Create a simple transfer instruction
        let instruction = system_instruction::transfer(&from, &to, 1_000_000);
        let message = Message::new(&[instruction], Some(&from));
        let tx = Transaction::new_unsigned(message);

        let fields = analyzer.analyze(&tx).await.unwrap();

        assert!(fields.contains_key("writable_non_signer_count"));
        assert!(fields.contains_key("potential_authority_mismatch"));

        // Simple transfer has 1 writable non-signer (recipient)
        assert_eq!(
            fields.get("writable_non_signer_count").unwrap().as_u64(),
            Some(1)
        );
        assert_eq!(
            fields
                .get("potential_authority_mismatch")
                .unwrap()
                .as_bool(),
            Some(false)
        );
    }

    #[test]
    fn test_analyzer_fields() {
        let analyzer = AccountsAnalyzer::new();
        let fields = analyzer.fields();

        assert_eq!(fields.len(), 2);
        assert!(fields.contains(&"writable_non_signer_count".to_string()));
        assert!(fields.contains(&"potential_authority_mismatch".to_string()));
    }
}
