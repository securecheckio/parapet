use crate::program_analysis::{FeedPoller, ProgramBlocklistState, ProgramDisassembler, ProgramFetcher};
use crate::rules::analyzer::TransactionAnalyzer;
use anyhow::Result;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct BlockedHash {
    pub program_id: String,
    pub hash: String,
}

pub struct ProgramAnalyzer {
    fetcher: ProgramFetcher,
    disassembler: ProgramDisassembler,
    blocklist_state: Arc<RwLock<ProgramBlocklistState>>,
}

impl ProgramAnalyzer {
    pub fn new(
        rpc_url: String,
        blocked_programs: Vec<String>,
        blocked_hashes: Vec<BlockedHash>,
    ) -> Result<Self> {
        let mut hash_map: HashMap<String, HashSet<String>> = HashMap::new();
        for blocked in blocked_hashes {
            hash_map
                .entry(blocked.program_id)
                .or_default()
                .insert(blocked.hash.to_lowercase());
        }

        let blocklist_state = Arc::new(RwLock::new(ProgramBlocklistState {
            blocked_programs: blocked_programs.into_iter().collect(),
            blocked_hashes: hash_map
                .into_iter()
                .flat_map(|(program_id, hashes)| {
                    hashes.into_iter().map(move |hash| BlockedHash {
                        program_id: program_id.clone(),
                        hash,
                    })
                })
                .collect(),
        }));

        Ok(Self {
            fetcher: ProgramFetcher::new(rpc_url),
            disassembler: ProgramDisassembler::new()?,
            blocklist_state,
        })
    }

    pub fn with_empty_blocklists(rpc_url: String) -> Result<Self> {
        Self::new(rpc_url, vec![], vec![])
    }

    pub fn with_feed_poller(
        rpc_url: String,
        blocked_programs: Vec<String>,
        blocked_hashes: Vec<BlockedHash>,
        feed_urls: Vec<String>,
        poll_interval: Duration,
    ) -> Result<Self> {
        let analyzer = Self::new(rpc_url, blocked_programs, blocked_hashes)?;
        if !feed_urls.is_empty() {
            let poller = FeedPoller::new(
                feed_urls,
                analyzer.blocklist_state.clone(),
                poll_interval,
            );
            tokio::spawn(async move {
                poller.start().await;
            });
        }
        Ok(analyzer)
    }

    fn extract_program_ids(tx: &Transaction) -> Vec<String> {
        tx.message
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
            .collect()
    }
}

#[async_trait::async_trait]
impl TransactionAnalyzer for ProgramAnalyzer {
    fn name(&self) -> &str {
        "program_analysis"
    }

    fn fields(&self) -> Vec<String> {
        vec![
            "program_ids".to_string(),
            "program_count".to_string(),
            "program_details".to_string(),
            "is_in_blocklist".to_string(),
            "blocked_program_ids".to_string(),
            "blocked_hashes".to_string(),
            "missing_signer_check".to_string(),
            "missing_owner_check".to_string(),
            "arbitrary_cpi".to_string(),
            "has_account_write".to_string(),
            "account_write_count".to_string(),
            "has_cpi_call".to_string(),
            "cpi_call_count".to_string(),
            "reads_account_data".to_string(),
            "account_read_count".to_string(),
            "has_signer_check".to_string(),
            "has_owner_check".to_string(),
            "has_key_check".to_string(),
            "checked_account_count".to_string(),
            "unchecked_account_count".to_string(),
            "bytecode_hashes".to_string(),
            "is_upgradeable".to_string(),
            "instruction_count".to_string(),
            "entropy_score".to_string(),
            "analysis_cached".to_string(),
            "analysis_duration_ms".to_string(),
            "spl_token_related".to_string(),
            "token_2022_related".to_string(),
        ]
    }

    async fn analyze(&self, tx: &Transaction) -> Result<HashMap<String, Value>> {
        let start = std::time::Instant::now();
        let program_ids = Self::extract_program_ids(tx);
        let mut fields = HashMap::new();
        let blocklist_snapshot = self.blocklist_state.read().await.clone();

        let mut blocked_hash_map: HashMap<String, HashSet<String>> = HashMap::new();
        for entry in &blocklist_snapshot.blocked_hashes {
            blocked_hash_map
                .entry(entry.program_id.clone())
                .or_default()
                .insert(entry.hash.to_lowercase());
        }

        let mut program_details = Vec::new();
        let mut blocked_program_ids = Vec::new();
        let mut blocked_hashes = Vec::new();
        let mut bytecode_hashes = HashMap::new();

        let mut any_missing_signer_check = false;
        let mut any_missing_owner_check = false;
        let mut any_arbitrary_cpi = false;
        let mut any_has_account_write = false;
        let mut any_has_cpi_call = false;
        let mut any_reads_account_data = false;
        let mut any_has_signer_check = false;
        let mut any_has_owner_check = false;
        let mut any_has_key_check = false;
        let mut any_upgradeable = false;
        let mut any_spl_token_related = false;
        let mut any_token_2022_related = false;

        let mut account_write_count = 0usize;
        let mut cpi_call_count = 0usize;
        let mut account_read_count = 0usize;
        let mut checked_account_count = 0usize;
        let mut unchecked_account_count = 0usize;
        let mut instruction_count = 0usize;
        let mut entropy_score_sum = 0.0f64;

        for program_id in &program_ids {
            let mut detail = serde_json::Map::new();
            detail.insert("program_id".to_string(), json!(program_id));

            if blocklist_snapshot.blocked_programs.contains(program_id) {
                blocked_program_ids.push(program_id.clone());
                detail.insert("in_program_blocklist".to_string(), json!(true));
            } else {
                detail.insert("in_program_blocklist".to_string(), json!(false));
            }

            let program_pubkey = match Pubkey::from_str(program_id) {
                Ok(pk) => pk,
                Err(_) => {
                    detail.insert("analysis_error".to_string(), json!("invalid_program_id"));
                    program_details.push(Value::Object(detail));
                    continue;
                }
            };

            let program_data = match self.fetcher.fetch_program(&program_pubkey).await {
                Ok(program_data) => program_data,
                Err(err) => {
                    detail.insert("analysis_error".to_string(), json!(err.to_string()));
                    program_details.push(Value::Object(detail));
                    continue;
                }
            };

            let mut hasher = Sha256::new();
            hasher.update(&program_data.executable_data);
            let bytecode_hash = format!("{:x}", hasher.finalize());
            bytecode_hashes.insert(program_id.clone(), bytecode_hash.clone());
            detail.insert("bytecode_hash".to_string(), json!(bytecode_hash.clone()));
            detail.insert("is_upgradeable".to_string(), json!(program_data.is_upgradeable));

            if program_data.is_upgradeable {
                any_upgradeable = true;
            }

            if blocked_hash_map
                .get(program_id)
                .map(|hashes| hashes.contains(&bytecode_hash))
                .unwrap_or(false)
            {
                blocked_hashes.push(format!("{program_id}:{bytecode_hash}"));
                detail.insert("in_hash_blocklist".to_string(), json!(true));
            } else {
                detail.insert("in_hash_blocklist".to_string(), json!(false));
            }

            let disassembly = match self.disassembler.disassemble(&program_data.executable_data) {
                Ok(disassembly) => disassembly,
                Err(err) => {
                    detail.insert("analysis_error".to_string(), json!(err.to_string()));
                    program_details.push(Value::Object(detail));
                    continue;
                }
            };

            any_missing_signer_check |= disassembly.missing_signer_check;
            any_missing_owner_check |= disassembly.missing_owner_check;
            any_arbitrary_cpi |= disassembly.arbitrary_cpi;
            any_has_account_write |= disassembly.has_account_write;
            any_has_cpi_call |= disassembly.has_cpi_call;
            any_reads_account_data |= disassembly.reads_account_data;
            any_has_signer_check |= disassembly.has_signer_check;
            any_has_owner_check |= disassembly.has_owner_check;
            any_has_key_check |= disassembly.has_key_check;
            any_spl_token_related |= disassembly.spl_token_related;
            any_token_2022_related |= disassembly.token_2022_related;

            account_write_count += disassembly.account_write_count;
            cpi_call_count += disassembly.cpi_call_count;
            account_read_count += disassembly.account_read_count;
            checked_account_count += disassembly.checked_account_count;
            unchecked_account_count += disassembly.unchecked_account_count;
            instruction_count += disassembly.total_instructions;
            entropy_score_sum += disassembly.entropy_score;

            detail.insert(
                "missing_signer_check".to_string(),
                json!(disassembly.missing_signer_check),
            );
            detail.insert(
                "missing_owner_check".to_string(),
                json!(disassembly.missing_owner_check),
            );
            detail.insert("arbitrary_cpi".to_string(), json!(disassembly.arbitrary_cpi));
            detail.insert(
                "has_account_write".to_string(),
                json!(disassembly.has_account_write),
            );
            detail.insert("has_cpi_call".to_string(), json!(disassembly.has_cpi_call));
            detail.insert(
                "reads_account_data".to_string(),
                json!(disassembly.reads_account_data),
            );
            detail.insert(
                "spl_token_related".to_string(),
                json!(disassembly.spl_token_related),
            );
            detail.insert(
                "token_2022_related".to_string(),
                json!(disassembly.token_2022_related),
            );
            detail.insert(
                "analysis_patterns".to_string(),
                json!(disassembly.suspicious_patterns),
            );

            program_details.push(Value::Object(detail));
        }

        let entropy_score = if program_ids.is_empty() {
            0.0
        } else {
            entropy_score_sum / program_ids.len() as f64
        };

        fields.insert("program_ids".to_string(), json!(program_ids));
        fields.insert("program_count".to_string(), json!(program_details.len()));
        fields.insert("program_details".to_string(), json!(program_details));
        fields.insert(
            "is_in_blocklist".to_string(),
            json!(!blocked_program_ids.is_empty() || !blocked_hashes.is_empty()),
        );
        fields.insert("blocked_program_ids".to_string(), json!(blocked_program_ids));
        fields.insert("blocked_hashes".to_string(), json!(blocked_hashes));
        fields.insert("missing_signer_check".to_string(), json!(any_missing_signer_check));
        fields.insert("missing_owner_check".to_string(), json!(any_missing_owner_check));
        fields.insert("arbitrary_cpi".to_string(), json!(any_arbitrary_cpi));
        fields.insert("has_account_write".to_string(), json!(any_has_account_write));
        fields.insert("account_write_count".to_string(), json!(account_write_count));
        fields.insert("has_cpi_call".to_string(), json!(any_has_cpi_call));
        fields.insert("cpi_call_count".to_string(), json!(cpi_call_count));
        fields.insert("reads_account_data".to_string(), json!(any_reads_account_data));
        fields.insert("account_read_count".to_string(), json!(account_read_count));
        fields.insert("has_signer_check".to_string(), json!(any_has_signer_check));
        fields.insert("has_owner_check".to_string(), json!(any_has_owner_check));
        fields.insert("has_key_check".to_string(), json!(any_has_key_check));
        fields.insert("checked_account_count".to_string(), json!(checked_account_count));
        fields.insert("unchecked_account_count".to_string(), json!(unchecked_account_count));
        fields.insert("bytecode_hashes".to_string(), json!(bytecode_hashes));
        fields.insert("is_upgradeable".to_string(), json!(any_upgradeable));
        fields.insert("instruction_count".to_string(), json!(instruction_count));
        fields.insert("entropy_score".to_string(), json!(entropy_score));
        fields.insert("analysis_cached".to_string(), json!(false));
        fields.insert(
            "analysis_duration_ms".to_string(),
            json!(start.elapsed().as_millis() as u64),
        );
        fields.insert(
            "spl_token_related".to_string(),
            json!(any_spl_token_related),
        );
        fields.insert(
            "token_2022_related".to_string(),
            json!(any_token_2022_related),
        );

        Ok(fields)
    }

    fn estimated_latency_ms(&self) -> u64 {
        500
    }
}
