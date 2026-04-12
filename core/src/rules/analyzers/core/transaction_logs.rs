use crate::rules::analyzer::{ConfirmedTransactionMetadata, TransactionAnalyzer};
use anyhow::Result;
use serde_json::{json, Value};
use solana_sdk::transaction::Transaction;
use std::collections::HashMap;

/// Parses confirmed transaction log messages into queryable rule fields.
///
/// Log messages are only available on confirmed transactions (from meta.logMessages)
/// and simulation results — not on pre-execution sendTransaction calls.
///
/// Fields exposed (all prefixed "logs:" in the rule engine):
///   instruction_names   — array of instruction names extracted from "Program log: Instruction: X"
///   program_invocations — array of program IDs extracted from "Program X invoke [N]"
///   raw                 — full array of log lines verbatim
///
/// Example rules:
///   { "field": "logs:instruction_names", "operator": "contains", "value": "UpdateAdmin" }
///   { "field": "logs:program_invocations", "operator": "contains", "value": "dRiftyHA39..." }
pub struct TransactionLogAnalyzer;

impl TransactionLogAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Extract "X" from "Program log: Instruction: X"
    fn extract_instruction_names(logs: &[String]) -> Vec<String> {
        logs.iter()
            .filter_map(|line| {
                let prefix = "Program log: Instruction: ";
                if line.starts_with(prefix) {
                    Some(line[prefix.len()..].trim().to_string())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Extract program IDs from "Program <ID> invoke [N]"
    fn extract_program_invocations(logs: &[String]) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        logs.iter()
            .filter_map(|line| {
                if line.starts_with("Program ") && line.contains(" invoke [") {
                    let parts: Vec<&str> = line.splitn(3, ' ').collect();
                    if parts.len() >= 2 {
                        let id = parts[1].to_string();
                        if seen.insert(id.clone()) {
                            return Some(id);
                        }
                    }
                }
                None
            })
            .collect()
    }

    fn parse_logs(logs: &[String]) -> HashMap<String, Value> {
        let mut fields = HashMap::new();
        let instruction_names = Self::extract_instruction_names(logs);
        let program_invocations = Self::extract_program_invocations(logs);
        let raw: Vec<Value> = logs.iter().map(|l| json!(l)).collect();

        fields.insert("instruction_names".to_string(), json!(instruction_names));
        fields.insert(
            "program_invocations".to_string(),
            json!(program_invocations),
        );
        fields.insert("raw".to_string(), json!(raw));
        fields
    }
}

impl Default for TransactionLogAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TransactionAnalyzer for TransactionLogAnalyzer {
    fn name(&self) -> &str {
        "logs"
    }

    fn fields(&self) -> Vec<String> {
        vec![
            "instruction_names".to_string(),
            "program_invocations".to_string(),
            "raw".to_string(),
        ]
    }

    /// Without logs, returns empty arrays — rules using logs:* fields will not match.
    async fn analyze(&self, _tx: &Transaction) -> Result<HashMap<String, Value>> {
        let mut fields = HashMap::new();
        fields.insert("instruction_names".to_string(), json!(Vec::<String>::new()));
        fields.insert(
            "program_invocations".to_string(),
            json!(Vec::<String>::new()),
        );
        fields.insert("raw".to_string(), json!(Vec::<String>::new()));
        Ok(fields)
    }

    async fn analyze_with_metadata(
        &self,
        _tx: &Transaction,
        metadata: &ConfirmedTransactionMetadata,
    ) -> Result<HashMap<String, Value>> {
        Ok(Self::parse_logs(&metadata.logs))
    }

    fn estimated_latency_ms(&self) -> u64 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drift_logs() -> Vec<String> {
        vec![
            "Program 11111111111111111111111111111111 invoke [1]".to_string(),
            "Program 11111111111111111111111111111111 success".to_string(),
            "Program SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf invoke [1]".to_string(),
            "Program log: Instruction: ProposalApprove".to_string(),
            "Program SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf success".to_string(),
            "Program SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf invoke [1]".to_string(),
            "Program log: Instruction: VaultTransactionExecute".to_string(),
            "Program dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH invoke [2]".to_string(),
            "Program log: Instruction: UpdateAdmin".to_string(),
            "Program log: admin: AiLGdNitMjv8n5HMS7HAdV2kaeJZZFd4jdfn5xp1PKrW -> H7PiGqqUaanBovwKgEtreJbKmQe6dbq6VTrw6guy7ZgL".to_string(),
            "Program dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH success".to_string(),
            "Program SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf success".to_string(),
        ]
    }

    #[test]
    fn test_extracts_instruction_names() {
        let logs = drift_logs();
        let names = TransactionLogAnalyzer::extract_instruction_names(&logs);
        assert!(names.contains(&"UpdateAdmin".to_string()));
        assert!(names.contains(&"VaultTransactionExecute".to_string()));
        assert!(names.contains(&"ProposalApprove".to_string()));
        assert_eq!(names.len(), 3);
    }

    #[test]
    fn test_extracts_program_invocations() {
        let logs = drift_logs();
        let programs = TransactionLogAnalyzer::extract_program_invocations(&logs);
        assert!(programs.contains(&"SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf".to_string()));
        assert!(programs.contains(&"dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH".to_string()));
        assert!(programs.contains(&"11111111111111111111111111111111".to_string()));
        assert_eq!(programs.len(), 3);
    }

    #[tokio::test]
    async fn test_analyze_without_logs_returns_empty() {
        use solana_sdk::transaction::Transaction;
        let analyzer = TransactionLogAnalyzer::new();
        let tx = Transaction::default();
        let fields = analyzer.analyze(&tx).await.unwrap();
        let names = fields["instruction_names"].as_array().unwrap();
        assert!(names.is_empty());
    }

    #[tokio::test]
    async fn test_analyze_with_logs_populates_fields() {
        use solana_sdk::transaction::Transaction;
        let analyzer = TransactionLogAnalyzer::new();
        let tx = Transaction::default();
        let logs = drift_logs();
        let fields = analyzer.analyze_with_logs(&tx, &logs).await.unwrap();
        let names = fields["instruction_names"].as_array().unwrap();
        assert!(names.contains(&json!("UpdateAdmin")));
    }
}
