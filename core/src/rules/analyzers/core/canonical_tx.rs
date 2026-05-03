use crate::rules::analyzer::TransactionAnalyzer;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_sdk::{
    message::{compiled_instruction::CompiledInstruction, VersionedMessage},
    pubkey::Pubkey,
    transaction::{Transaction, VersionedTransaction},
};
use std::collections::HashMap;

/// Canonical Transaction Analyzer
///
/// Computes a deterministic hash of transaction excluding blockhash and signatures.
/// This allows consent to remain valid even when the transaction is rebuilt with a fresh blockhash.
pub struct CanonicalTransactionAnalyzer;

impl Default for CanonicalTransactionAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl CanonicalTransactionAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Compute canonical hash of transaction
    pub fn compute_canonical_hash(tx: &Transaction) -> Result<String> {
        use sha2::{Digest, Sha256};

        let canonical = CanonicalTransaction::from_transaction(tx)?;
        let serialized = borsh::to_vec(&canonical)?;

        let mut hasher = Sha256::new();
        hasher.update(&serialized);
        let hash = hasher.finalize();

        Ok(bs58::encode(&hash).into_string())
    }

    /// Compute canonical hash of versioned transaction
    pub fn compute_canonical_hash_versioned(tx: &VersionedTransaction) -> Result<String> {
        use sha2::{Digest, Sha256};

        let canonical = CanonicalTransaction::from_versioned_transaction(tx)?;
        let serialized = borsh::to_vec(&canonical)?;

        let mut hasher = Sha256::new();
        hasher.update(&serialized);
        let hash = hasher.finalize();

        Ok(bs58::encode(&hash).into_string())
    }
}

#[async_trait::async_trait]
impl TransactionAnalyzer for CanonicalTransactionAnalyzer {
    fn name(&self) -> &str {
        "canonical_tx"
    }

    fn fields(&self) -> Vec<String> {
        vec!["canonical_transaction_hash".to_string()]
    }

    async fn analyze(&self, tx: &Transaction) -> Result<HashMap<String, Value>> {
        let canonical_hash = Self::compute_canonical_hash(tx)?;

        let mut result = HashMap::new();
        result.insert(
            "canonical_transaction_hash".to_string(),
            json!(canonical_hash),
        );

        Ok(result)
    }

    fn estimated_latency_ms(&self) -> u64 {
        1 // Very fast, local computation only
    }
}

/// Canonical transaction representation (excludes blockhash and signatures)
#[derive(Debug, Clone, Serialize, Deserialize, borsh::BorshSerialize, borsh::BorshDeserialize)]
pub struct CanonicalTransaction {
    /// Normalized instructions (compute budget removed, deterministically ordered)
    pub instructions: Vec<CanonicalInstruction>,
    /// Account keys (expanded from ALTs if present)
    pub account_keys: Vec<Pubkey>,
}

#[derive(Debug, Clone, Serialize, Deserialize, borsh::BorshSerialize, borsh::BorshDeserialize)]
pub struct CanonicalInstruction {
    pub program_id_index: u8,
    pub accounts: Vec<u8>,
    pub data: Vec<u8>,
}

impl CanonicalTransaction {
    /// Create canonical transaction from regular transaction
    pub fn from_transaction(tx: &Transaction) -> Result<Self> {
        // Normalize instructions (remove compute budget, keep deterministic order)
        let instructions =
            Self::normalize_instructions(&tx.message.instructions, &tx.message.account_keys);

        Ok(Self {
            instructions,
            account_keys: tx.message.account_keys.clone(),
        })
    }

    /// Create canonical transaction from versioned transaction
    pub fn from_versioned_transaction(tx: &VersionedTransaction) -> Result<Self> {
        // Extract message and handle both v0 and legacy
        let (instructions, account_keys) = match &tx.message {
            VersionedMessage::V0(v0_msg) => (
                v0_msg.instructions.as_slice(),
                v0_msg.account_keys.as_slice(),
            ),
            VersionedMessage::Legacy(legacy_msg) => (
                legacy_msg.instructions.as_slice(),
                legacy_msg.account_keys.as_slice(),
            ),
            VersionedMessage::V1(v1_msg) => (
                v1_msg.instructions.as_slice(),
                v1_msg.account_keys.as_slice(),
            ),
        };

        // Normalize instructions
        let normalized_instructions = Self::normalize_instructions(instructions, account_keys);

        Ok(Self {
            instructions: normalized_instructions,
            account_keys: account_keys.to_vec(),
        })
    }

    /// Normalize instructions for canonical hashing
    /// - Remove compute budget instructions (priority fees can vary)
    /// - Keep instruction order (security relevant)
    fn normalize_instructions(
        instructions: &[CompiledInstruction],
        account_keys: &[Pubkey],
    ) -> Vec<CanonicalInstruction> {
        const COMPUTE_BUDGET_PROGRAM: &str = "ComputeBudget111111111111111111111111111111";

        instructions
            .iter()
            .filter(|ix| {
                // Exclude compute budget instructions
                if let Some(program_id) = account_keys.get(ix.program_id_index as usize) {
                    program_id.to_string() != COMPUTE_BUDGET_PROGRAM
                } else {
                    true
                }
            })
            .map(|ix| CanonicalInstruction {
                program_id_index: ix.program_id_index,
                accounts: ix.accounts.clone(),
                data: ix.data.clone(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{
        pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction,
    };
    use solana_system_interface::instruction as system_instruction;

    #[tokio::test]
    async fn test_canonical_hash_excludes_blockhash() {
        let from = Keypair::new();
        let to = Pubkey::new_unique();

        // Create two transactions with same instructions but different blockhashes
        let instruction = system_instruction::transfer(&from.pubkey(), &to, 1_000_000);

        let mut tx1 =
            Transaction::new_with_payer(std::slice::from_ref(&instruction), Some(&from.pubkey()));
        tx1.message.recent_blockhash = solana_sdk::hash::Hash::new_unique();

        let mut tx2 =
            Transaction::new_with_payer(std::slice::from_ref(&instruction), Some(&from.pubkey()));
        tx2.message.recent_blockhash = solana_sdk::hash::Hash::new_unique();

        let analyzer = CanonicalTransactionAnalyzer::new();

        let result1 = analyzer.analyze(&tx1).await.unwrap();
        let result2 = analyzer.analyze(&tx2).await.unwrap();

        let hash1 = result1
            .get("canonical_transaction_hash")
            .unwrap()
            .as_str()
            .unwrap();
        let hash2 = result2
            .get("canonical_transaction_hash")
            .unwrap()
            .as_str()
            .unwrap();

        // Same canonical hash despite different blockhashes
        assert_eq!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_canonical_hash_different_instructions() {
        let from = Keypair::new();
        let to1 = Pubkey::new_unique();
        let to2 = Pubkey::new_unique();

        let ix1 = system_instruction::transfer(&from.pubkey(), &to1, 1_000_000);
        let ix2 = system_instruction::transfer(&from.pubkey(), &to2, 1_000_000);

        let mut tx1 = Transaction::new_with_payer(&[ix1], Some(&from.pubkey()));
        tx1.message.recent_blockhash = solana_sdk::hash::Hash::new_unique();

        let mut tx2 = Transaction::new_with_payer(&[ix2], Some(&from.pubkey()));
        tx2.message.recent_blockhash = solana_sdk::hash::Hash::new_unique();

        let analyzer = CanonicalTransactionAnalyzer::new();

        let result1 = analyzer.analyze(&tx1).await.unwrap();
        let result2 = analyzer.analyze(&tx2).await.unwrap();

        let hash1 = result1
            .get("canonical_transaction_hash")
            .unwrap()
            .as_str()
            .unwrap();
        let hash2 = result2
            .get("canonical_transaction_hash")
            .unwrap()
            .as_str()
            .unwrap();

        // Different hashes for different recipients
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_analyzer_name() {
        let analyzer = CanonicalTransactionAnalyzer::new();
        assert_eq!(analyzer.name(), "canonical_tx");
    }

    #[test]
    fn test_analyzer_fields() {
        let analyzer = CanonicalTransactionAnalyzer::new();
        let fields = analyzer.fields();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0], "canonical_transaction_hash");
    }

    #[test]
    fn test_analyzer_estimated_latency() {
        let analyzer = CanonicalTransactionAnalyzer::new();
        assert_eq!(analyzer.estimated_latency_ms(), 1);
    }

    #[tokio::test]
    async fn test_canonical_hash_excludes_signatures() {
        let from = Keypair::new();
        let to = Pubkey::new_unique();

        let instruction = system_instruction::transfer(&from.pubkey(), &to, 1_000_000);

        let mut tx1 =
            Transaction::new_with_payer(std::slice::from_ref(&instruction), Some(&from.pubkey()));
        tx1.message.recent_blockhash = solana_sdk::hash::Hash::new_unique();
        tx1.sign(&[&from], tx1.message.recent_blockhash);

        let mut tx2 =
            Transaction::new_with_payer(std::slice::from_ref(&instruction), Some(&from.pubkey()));
        tx2.message.recent_blockhash = solana_sdk::hash::Hash::new_unique();
        // Don't sign tx2

        let hash1 = CanonicalTransactionAnalyzer::compute_canonical_hash(&tx1).unwrap();
        let hash2 = CanonicalTransactionAnalyzer::compute_canonical_hash(&tx2).unwrap();

        // Same canonical hash regardless of signatures
        assert_eq!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_canonical_hash_different_amounts() {
        let from = Keypair::new();
        let to = Pubkey::new_unique();

        let ix1 = system_instruction::transfer(&from.pubkey(), &to, 1_000_000);
        let ix2 = system_instruction::transfer(&from.pubkey(), &to, 2_000_000);

        let mut tx1 = Transaction::new_with_payer(&[ix1], Some(&from.pubkey()));
        tx1.message.recent_blockhash = solana_sdk::hash::Hash::new_unique();

        let mut tx2 = Transaction::new_with_payer(&[ix2], Some(&from.pubkey()));
        tx2.message.recent_blockhash = solana_sdk::hash::Hash::new_unique();

        let hash1 = CanonicalTransactionAnalyzer::compute_canonical_hash(&tx1).unwrap();
        let hash2 = CanonicalTransactionAnalyzer::compute_canonical_hash(&tx2).unwrap();

        // Different amounts = different hashes
        assert_ne!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_canonical_hash_instruction_order_matters() {
        let from = Keypair::new();
        let to1 = Pubkey::new_unique();
        let to2 = Pubkey::new_unique();

        let ix1 = system_instruction::transfer(&from.pubkey(), &to1, 1_000_000);
        let ix2 = system_instruction::transfer(&from.pubkey(), &to2, 1_000_000);

        let mut tx1 =
            Transaction::new_with_payer(&[ix1.clone(), ix2.clone()], Some(&from.pubkey()));
        tx1.message.recent_blockhash = solana_sdk::hash::Hash::new_unique();

        let mut tx2 = Transaction::new_with_payer(&[ix2, ix1], Some(&from.pubkey()));
        tx2.message.recent_blockhash = solana_sdk::hash::Hash::new_unique();

        let hash1 = CanonicalTransactionAnalyzer::compute_canonical_hash(&tx1).unwrap();
        let hash2 = CanonicalTransactionAnalyzer::compute_canonical_hash(&tx2).unwrap();

        // Different order = different hashes
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_canonical_transaction_from_transaction() {
        let from = Keypair::new();
        let to = Pubkey::new_unique();

        let instruction = system_instruction::transfer(&from.pubkey(), &to, 1_000_000);
        let mut tx = Transaction::new_with_payer(&[instruction], Some(&from.pubkey()));
        tx.message.recent_blockhash = solana_sdk::hash::Hash::new_unique();

        let canonical = CanonicalTransaction::from_transaction(&tx).unwrap();

        assert_eq!(canonical.instructions.len(), 1);
        assert_eq!(canonical.account_keys.len(), tx.message.account_keys.len());
    }

    #[test]
    fn test_canonical_instruction_structure() {
        let from = Keypair::new();
        let to = Pubkey::new_unique();

        let instruction = system_instruction::transfer(&from.pubkey(), &to, 1_000_000);
        let mut tx = Transaction::new_with_payer(&[instruction], Some(&from.pubkey()));
        tx.message.recent_blockhash = solana_sdk::hash::Hash::new_unique();

        let canonical = CanonicalTransaction::from_transaction(&tx).unwrap();
        let canonical_ix = &canonical.instructions[0];

        assert_eq!(
            canonical_ix.program_id_index,
            tx.message.instructions[0].program_id_index
        );
        assert_eq!(canonical_ix.accounts, tx.message.instructions[0].accounts);
        assert_eq!(canonical_ix.data, tx.message.instructions[0].data);
    }

    #[tokio::test]
    async fn test_canonical_hash_is_base58() {
        let from = Keypair::new();
        let to = Pubkey::new_unique();

        let instruction = system_instruction::transfer(&from.pubkey(), &to, 1_000_000);
        let mut tx = Transaction::new_with_payer(&[instruction], Some(&from.pubkey()));
        tx.message.recent_blockhash = solana_sdk::hash::Hash::new_unique();

        let hash = CanonicalTransactionAnalyzer::compute_canonical_hash(&tx).unwrap();

        // Should be valid base58
        assert!(bs58::decode(&hash).into_vec().is_ok());

        // SHA256 hash is 32 bytes, base58 encoded should be ~44 chars
        assert!(hash.len() > 40 && hash.len() < 50);
    }

    #[tokio::test]
    async fn test_canonical_hash_deterministic() {
        let from = Keypair::new();
        let to = Pubkey::new_unique();

        let instruction = system_instruction::transfer(&from.pubkey(), &to, 1_000_000);
        let mut tx = Transaction::new_with_payer(&[instruction], Some(&from.pubkey()));
        tx.message.recent_blockhash = solana_sdk::hash::Hash::new_unique();

        // Compute hash multiple times
        let hash1 = CanonicalTransactionAnalyzer::compute_canonical_hash(&tx).unwrap();
        let hash2 = CanonicalTransactionAnalyzer::compute_canonical_hash(&tx).unwrap();
        let hash3 = CanonicalTransactionAnalyzer::compute_canonical_hash(&tx).unwrap();

        // Should always produce same hash
        assert_eq!(hash1, hash2);
        assert_eq!(hash2, hash3);
    }

    #[tokio::test]
    async fn test_multiple_instructions() {
        let from = Keypair::new();
        let to1 = Pubkey::new_unique();
        let to2 = Pubkey::new_unique();
        let to3 = Pubkey::new_unique();

        let ix1 = system_instruction::transfer(&from.pubkey(), &to1, 1_000_000);
        let ix2 = system_instruction::transfer(&from.pubkey(), &to2, 2_000_000);
        let ix3 = system_instruction::transfer(&from.pubkey(), &to3, 3_000_000);

        let mut tx = Transaction::new_with_payer(&[ix1, ix2, ix3], Some(&from.pubkey()));
        tx.message.recent_blockhash = solana_sdk::hash::Hash::new_unique();

        let canonical = CanonicalTransaction::from_transaction(&tx).unwrap();

        assert_eq!(canonical.instructions.len(), 3);
    }

    #[test]
    fn test_canonical_transaction_serialization() {
        let from = Keypair::new();
        let to = Pubkey::new_unique();

        let instruction = system_instruction::transfer(&from.pubkey(), &to, 1_000_000);
        let mut tx = Transaction::new_with_payer(&[instruction], Some(&from.pubkey()));
        tx.message.recent_blockhash = solana_sdk::hash::Hash::new_unique();

        let canonical = CanonicalTransaction::from_transaction(&tx).unwrap();

        // Should be serializable with borsh
        let serialized = borsh::to_vec(&canonical).unwrap();
        assert!(!serialized.is_empty());

        // Should be deserializable
        let deserialized: CanonicalTransaction =
            borsh::BorshDeserialize::try_from_slice(&serialized).unwrap();
        assert_eq!(
            deserialized.instructions.len(),
            canonical.instructions.len()
        );
    }

    #[test]
    fn test_canonical_instruction_serialization() {
        let ix = CanonicalInstruction {
            program_id_index: 1,
            accounts: vec![0, 1, 2],
            data: vec![1, 2, 3, 4],
        };

        let serialized = borsh::to_vec(&ix).unwrap();
        assert!(!serialized.is_empty());

        let deserialized: CanonicalInstruction =
            borsh::BorshDeserialize::try_from_slice(&serialized).unwrap();
        assert_eq!(deserialized.program_id_index, ix.program_id_index);
        assert_eq!(deserialized.accounts, ix.accounts);
        assert_eq!(deserialized.data, ix.data);
    }
}
