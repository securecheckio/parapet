use crate::rules::analyzer::{ConfirmedTransactionMetadata, TransactionAnalyzer};
use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use solana_sdk::transaction::Transaction;
use std::collections::HashMap;

/// How a fingerprint's bytes are derived.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FingerprintMethod {
    /// Anchor convention: SHA256("global:<name>")[0..8]
    Anchor,
    /// Explicit byte prefix — for native programs or non-Anchor frameworks
    Bytes,
}

/// A single fingerprint entry from the config file.
#[derive(Debug, Clone, Deserialize)]
pub struct FingerprintEntry {
    pub name: String,
    pub method: FingerprintMethod,
    /// Required when method = "bytes"
    pub bytes: Option<Vec<u8>>,
    /// Optional category tag — used to derive per-category boolean fields.
    /// e.g. "authority_change" → instruction_data:has_authority_change
    pub category: Option<String>,
}

/// Top-level structure of a fingerprint config file.
#[derive(Debug, Deserialize)]
pub struct FingerprintConfig {
    pub fingerprints: Vec<FingerprintEntry>,
}

/// A resolved fingerprint: name + the actual bytes to match against.
#[derive(Debug, Clone)]
struct ResolvedFingerprint {
    name: String,
    bytes: Vec<u8>,
    category: Option<String>,
}

/// Analyzes raw instruction data bytes to identify known instruction types.
///
/// Framework-agnostic and config-driven — fingerprint definitions live in JSON
/// config files, not in Rust code. Supports:
///   - Anchor programs: discriminator = SHA256("global:<name>")[0..8]
///   - Native/other programs: explicit byte prefix from config
///
/// Fields exposed (prefixed "instruction_data:" in the rule engine):
///   matched_names  — array of instruction names whose fingerprint matched
///   has_match      — true if any fingerprint matched
///
/// Example rule:
///   { "field": "instruction_data:matched_names", "operator": "contains", "value": "update_admin" }
pub struct InstructionDataAnalyzer {
    fingerprints: Vec<ResolvedFingerprint>,
    /// Lookup index: prefix_bytes → fingerprint name, bucketed by prefix length.
    /// Built once at construction time for O(1) matching per instruction.
    index: HashMap<usize, HashMap<Vec<u8>, String>>,
}

impl InstructionDataAnalyzer {
    /// Create with an explicit list of fingerprint entries (e.g. loaded from config).
    pub fn new(entries: Vec<FingerprintEntry>) -> Self {
        let fingerprints: Vec<ResolvedFingerprint> = entries
            .into_iter()
            .filter_map(|entry| {
                let bytes = match entry.method {
                    FingerprintMethod::Anchor => Some(anchor_discriminator(&entry.name)),
                    FingerprintMethod::Bytes => {
                        if let Some(b) = entry.bytes {
                            if b.is_empty() {
                                log::warn!(
                                    "Fingerprint '{}' has method=bytes but empty bytes array — skipping",
                                    entry.name
                                );
                                None
                            } else {
                                Some(b)
                            }
                        } else {
                            log::warn!(
                                "Fingerprint '{}' has method=bytes but no bytes field — skipping",
                                entry.name
                            );
                            None
                        }
                    }
                };
                bytes.map(|b| ResolvedFingerprint { name: entry.name, bytes: b, category: entry.category })
            })
            .collect();

        let index = Self::build_index(&fingerprints);
        Self { fingerprints, index }
    }

    fn build_index(fingerprints: &[ResolvedFingerprint]) -> HashMap<usize, HashMap<Vec<u8>, String>> {
        let mut index: HashMap<usize, HashMap<Vec<u8>, String>> = HashMap::new();
        for fp in fingerprints {
            index
                .entry(fp.bytes.len())
                .or_default()
                .insert(fp.bytes.clone(), fp.name.clone());
        }
        index
    }

    /// Load from a JSON config file.
    pub fn from_config_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read fingerprint config '{}': {}", path, e))?;
        let config: FingerprintConfig = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse fingerprint config '{}': {}", path, e))?;
        Ok(Self::new(config.fingerprints))
    }

    /// Create with a default set of authority-change instruction names.
    /// All use the Anchor discriminator formula — no hardcoded bytes.
    pub fn with_default_authority_names() -> Self {
        let names = [
            "update_admin",
            "set_admin",
            "transfer_admin",
            "change_admin",
            "update_authority",
            "set_authority",
            "transfer_authority",
            "change_authority",
            "update_owner",
            "set_owner",
            "transfer_owner",
            "change_owner",
            "transfer_ownership",
            "update_governance",
            "set_governance",
            "update_multisig",
            "change_multisig",
            "update_threshold",
            "set_threshold",
            "add_member",
            "remove_member",
            "update_config",
            "set_config",
            "add_admin",
            "remove_admin",
            "set_delegate",
            "update_delegate",
            "grant_role",
            "revoke_role",
            "update_treasury",
            "set_treasury",
            "update_vault",
            "set_vault",
            "emergency_withdraw",
            "set_upgrade_authority",
            "set_freeze_authority",
            "set_mint_authority",
        ];

        let fingerprints: Vec<ResolvedFingerprint> = names
            .iter()
            .map(|name| ResolvedFingerprint {
                name: name.to_string(),
                bytes: anchor_discriminator(name),
                category: Some("authority_change".to_string()),
            })
            // Native SPL Token SetAuthority (tag=4 u32-le + authority_type=1).
            // 5-byte prefix avoids collision with System Program AdvanceNonceAccount [4,0,0,0].
            .chain(std::iter::once(ResolvedFingerprint {
                name: "set_authority_spl".to_string(),
                bytes: vec![4, 0, 0, 0, 1],
                category: Some("authority_change".to_string()),
            }))
            .collect();

        let index = Self::build_index(&fingerprints);
        Self { fingerprints, index }
    }

    fn match_data(&self, data: &[u8], matched: &mut Vec<String>) {
        if data.is_empty() {
            return;
        }
        // O(prefix_lengths) lookups instead of O(fingerprints) comparisons.
        // For each distinct prefix length, slice the data and do a single HashMap lookup.
        for (&prefix_len, bucket) in &self.index {
            if data.len() < prefix_len {
                continue;
            }
            // For short prefixes (< 8 bytes) require data is not dramatically longer
            // to avoid coincidental matches in large Anchor payloads.
            let is_plausible = prefix_len >= 8 || data.len() <= prefix_len + 64;
            if !is_plausible {
                continue;
            }
            if let Some(name) = bucket.get(&data[..prefix_len]) {
                if !matched.contains(name) {
                    matched.push(name.clone());
                }
            }
        }
    }

    /// Scan top-level transaction instructions only.
    fn scan(&self, tx: &Transaction) -> Vec<String> {
        let mut matched = Vec::new();
        for ix in &tx.message.instructions {
            self.match_data(&ix.data, &mut matched);
        }
        matched
    }

    /// Scan top-level instructions AND CPI inner instructions from confirmed tx metadata.
    fn scan_with_metadata(
        &self,
        tx: &Transaction,
        metadata: &ConfirmedTransactionMetadata,
    ) -> Vec<String> {
        let mut matched = self.scan(tx);
        for inner_ix in &metadata.inner_instructions {
            self.match_data(&inner_ix.data, &mut matched);
        }
        matched
    }
}

/// Compute the Anchor instruction discriminator: SHA256("global:<name>")[0..8]
pub fn anchor_discriminator(name: &str) -> Vec<u8> {
    let preimage = format!("global:{}", name);
    let hash = Sha256::digest(preimage.as_bytes());
    hash[..8].to_vec()
}

impl InstructionDataAnalyzer {
    /// Build the fields map from a list of matched fingerprint names.
    fn build_fields(&self, matched: Vec<String>) -> HashMap<String, Value> {
        // Collect per-category matched names
        let mut by_category: HashMap<String, Vec<String>> = HashMap::new();
        for name in &matched {
            if let Some(fp) = self.fingerprints.iter().find(|fp| fp.name == *name) {
                if let Some(cat) = &fp.category {
                    by_category.entry(cat.clone()).or_default().push(name.clone());
                }
            }
        }

        let has_authority_change = by_category.contains_key("authority_change");

        let mut fields = HashMap::new();
        fields.insert("matched_names".to_string(), json!(matched));
        fields.insert("has_authority_change".to_string(), json!(has_authority_change));

        // Per-category arrays e.g. "authority_change_names"
        for (cat, names) in by_category {
            fields.insert(format!("{}_names", cat), json!(names));
        }

        fields
    }
}

#[async_trait::async_trait]
impl TransactionAnalyzer for InstructionDataAnalyzer {
    fn name(&self) -> &str {
        "instruction_data"
    }

    fn fields(&self) -> Vec<String> {
        let mut fields = vec![
            "matched_names".to_string(),
            "has_authority_change".to_string(),
        ];
        let categories: std::collections::HashSet<&str> = self
            .fingerprints
            .iter()
            .filter_map(|fp| fp.category.as_deref())
            .collect();
        for cat in categories {
            fields.push(format!("{}_names", cat));
        }
        fields
    }

    async fn analyze(&self, tx: &Transaction) -> Result<HashMap<String, Value>> {
        Ok(self.build_fields(self.scan(tx)))
    }

    async fn analyze_with_metadata(
        &self,
        tx: &Transaction,
        metadata: &ConfirmedTransactionMetadata,
    ) -> Result<HashMap<String, Value>> {
        Ok(self.build_fields(self.scan_with_metadata(tx, metadata)))
    }

    fn estimated_latency_ms(&self) -> u64 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anchor_discriminator_is_deterministic() {
        let d1 = anchor_discriminator("update_admin");
        let d2 = anchor_discriminator("update_admin");
        assert_eq!(d1, d2);
        assert_eq!(d1.len(), 8);
    }

    #[test]
    fn test_different_names_produce_different_discriminators() {
        let d1 = anchor_discriminator("update_admin");
        let d2 = anchor_discriminator("set_authority");
        assert_ne!(d1, d2);
    }

    #[test]
    fn test_known_update_admin_discriminator() {
        // Verified against the real Drift UpdateAdmin instruction
        let d = anchor_discriminator("update_admin");
        assert_eq!(d, vec![0xa1, 0xb0, 0x28, 0xd5, 0x3c, 0xb8, 0xb3, 0xe4]);
    }

    #[test]
    fn test_spl_set_authority_does_not_match_advance_nonce() {
        // System Program AdvanceNonceAccount is [4,0,0,0] — must NOT match set_authority_spl
        // which requires [4,0,0,0,1] (5 bytes)
        use solana_sdk::instruction::CompiledInstruction;
        use solana_sdk::message::Message;
        use solana_sdk::pubkey::Pubkey;

        let analyzer = InstructionDataAnalyzer::with_default_authority_names();

        let ix = CompiledInstruction {
            program_id_index: 0,
            accounts: vec![],
            data: vec![4, 0, 0, 0], // AdvanceNonceAccount — exactly 4 bytes
        };

        let message = Message {
            header: solana_sdk::message::MessageHeader::default(),
            account_keys: vec![Pubkey::new_unique()],
            recent_blockhash: solana_sdk::hash::Hash::default(),
            instructions: vec![ix],
        };

        let tx = Transaction { signatures: vec![], message };

        // Must not match — AdvanceNonceAccount should not be flagged as authority change
        let matched = analyzer.scan(&tx);
        assert!(!matched.contains(&"set_authority_spl".to_string()), 
            "AdvanceNonceAccount [4,0,0,0] must not match set_authority_spl");
        assert!(matched.is_empty(), "No fingerprints should match AdvanceNonceAccount");
    }

    #[tokio::test]
    async fn test_scan_detects_matching_instruction() {
        use solana_sdk::instruction::CompiledInstruction;
        use solana_sdk::message::Message;
        use solana_sdk::pubkey::Pubkey;

        let analyzer = InstructionDataAnalyzer::with_default_authority_names();

        // Build a transaction with an instruction whose data starts with the update_admin discriminator
        let discriminator = anchor_discriminator("update_admin");
        let mut data = discriminator.clone();
        data.extend_from_slice(&[0u8; 32]); // fake authority pubkey bytes after discriminator

        let program_id = Pubkey::new_unique();
        let ix = CompiledInstruction {
            program_id_index: 0,
            accounts: vec![],
            data,
        };

        let message = Message {
            header: solana_sdk::message::MessageHeader {
                num_required_signatures: 0,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 0,
            },
            account_keys: vec![program_id],
            recent_blockhash: solana_sdk::hash::Hash::default(),
            instructions: vec![ix],
        };

        let tx = Transaction {
            signatures: vec![],
            message,
        };

        let fields = analyzer.analyze(&tx).await.unwrap();
        let matched = fields["matched_names"].as_array().unwrap();
        assert!(matched.contains(&json!("update_admin")));
        assert_eq!(fields["has_authority_change"], json!(true));
    }

    #[tokio::test]
    async fn test_scan_no_match_on_empty_data() {
        use solana_sdk::instruction::CompiledInstruction;
        use solana_sdk::message::Message;
        use solana_sdk::pubkey::Pubkey;

        let analyzer = InstructionDataAnalyzer::with_default_authority_names();

        let ix = CompiledInstruction {
            program_id_index: 0,
            accounts: vec![],
            data: vec![],
        };

        let message = Message {
            header: solana_sdk::message::MessageHeader::default(),
            account_keys: vec![Pubkey::new_unique()],
            recent_blockhash: solana_sdk::hash::Hash::default(),
            instructions: vec![ix],
        };

        let tx = Transaction { signatures: vec![], message };
        let fields = analyzer.analyze(&tx).await.unwrap();
        assert_eq!(fields["has_authority_change"], json!(false));
    }
}
