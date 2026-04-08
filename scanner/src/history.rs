use anyhow::{anyhow, Result};
use base64::Engine;
use base64::engine::general_purpose;
use log::{debug, info, warn};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcTransactionConfig};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::Signature,
    transaction::Transaction,
    message::{Message, VersionedMessage},
    transaction::VersionedTransaction,
    bs58,
};
use solana_transaction_status::{
    EncodedTransaction, EncodedTransactionWithStatusMeta, UiTransactionEncoding,
    UiInstruction, option_serializer::OptionSerializer,
};
use parapet_core::rules::{AnalyzerRegistry, RuleEngine, RuleDecision, RuleAction};
use std::str::FromStr;
use std::sync::Arc;
use std::io::Write;

#[cfg(feature = "reqwest")]
use parapet_core::enrichment::EnrichmentService;

use crate::detector::{Severity, ThreatAssessment, ThreatType};
use crate::report::ScanConfig;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Extended transaction data including metadata (inner instructions, logs, etc.)
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TransactionWithMetadata {
    pub transaction: Transaction,
    pub inner_instructions: Vec<InnerInstructionSet>,
    pub program_ids_from_inner: Vec<String>,
}

/// Inner instructions from a specific top-level instruction
#[derive(Debug, Clone)]
pub struct InnerInstructionSet {
    pub index: u8,
    pub instructions: Vec<ParsedInnerInstruction>,
}

/// Parsed inner instruction (CPI)
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ParsedInnerInstruction {
    pub program_id: String,
    pub accounts: Vec<u8>,
    pub data: Vec<u8>,
}

/// Program encounter during transaction analysis
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ProgramEncounter {
    pub program_id: String,
    pub transaction_signatures: Vec<String>,
    pub first_seen: DateTime<Utc>,
    pub rule_decision: Option<RuleDecision>,
}

/// Result of history scan with threats and program encounters
#[derive(Debug)]
pub struct HistoryScanResult {
    pub threats: Vec<ThreatAssessment>,
    pub program_encounters: HashMap<String, ProgramEncounter>,
    pub transactions_analyzed: usize,
}

/// Scans historical transactions for threats
pub struct HistoryScanner;

impl HistoryScanner {
    /// Scan wallet's transaction history
    pub async fn scan_history(
        rpc: &RpcClient,
        analyzer_registry: &Arc<AnalyzerRegistry>,
        rule_engine: &Arc<RuleEngine>,
        wallet: &str,
        config: &ScanConfig,
        #[cfg(feature = "reqwest")]
        enrichment: Option<&Arc<EnrichmentService>>,
    ) -> Result<HistoryScanResult> {
        let wallet_pubkey = Pubkey::from_str(wallet)
            .map_err(|e| anyhow!("Invalid wallet address: {}", e))?;

        // Fetch transaction signatures
        info!("Fetching transaction history for wallet: {}", wallet);
        let signatures = Self::fetch_signatures(rpc, &wallet_pubkey, config)?;
        info!("Found {} transactions to analyze", signatures.len());
        
        // ENRICHMENT STEP 1: Extract all unique token addresses from transactions (if enrichment enabled)
        #[cfg(feature = "reqwest")]
        let enrichment_cache = if enrichment.is_some() {
            info!("🔍 Pre-scanning transactions to extract tokens for enrichment...");
            let mut unique_tokens = std::collections::HashSet::new();

            // Quick scan to find all tokens (we'll re-fetch transactions for full analysis later)
            for (signature, _) in &signatures {
                if let Ok(tx_with_meta) = Self::fetch_transaction(rpc, signature).await {
                    for account in tx_with_meta.transaction.message.account_keys.iter() {
                        unique_tokens.insert(account.to_string());
                    }
                }
            }

            info!("📊 Found {} unique addresses, enriching token data...", unique_tokens.len());

            // ENRICHMENT STEP 2: Bulk enrich all tokens (1 API call per 50 tokens)
            if let Some(service) = enrichment {
                let tokens: Vec<String> = unique_tokens.into_iter().collect();
                match service.enrich_tokens_bulk(&tokens).await {
                    Ok(cache) => {
                        info!("✅ Enriched {} tokens with off-chain data", cache.len());
                        cache
                    }
                    Err(e) => {
                        warn!("⚠️  Failed to enrich tokens: {} - continuing without enrichment", e);
                        std::collections::HashMap::new()
                    }
                }
            } else {
                std::collections::HashMap::new()
            }
        } else {
            std::collections::HashMap::new()
        };

        // ENRICHMENT STEP 3: Populate RuleEngine cache
        #[cfg(feature = "reqwest")]
        if !enrichment_cache.is_empty() {
            rule_engine.set_enrichment_cache(enrichment_cache).await;
            info!("✅ Rules now have access to enrichment data for tokens");
        }

        // Analyze each transaction (with enrichment context already loaded)
        let mut threats = Vec::new();
        let mut program_encounters: HashMap<String, ProgramEncounter> = HashMap::new();
        let mut error_count = 0;
        let total = signatures.len();
        
        for (idx, (signature, block_time)) in signatures.iter().enumerate() {
            // Progress output every transaction (visible to user)
            let progress = idx + 1;
            eprint!("\r⏳ Analyzing transactions [{}/{}] ", progress, total);
            std::io::stderr().flush().ok();

            // RPC throttling: Add delay between requests to avoid rate limits
            // Coordinate delays to respect all rate limits (RPC + Analyzers)
            // With Helius (20/min = 3s), OtterSec (30/min = 2s), must go at pace of slowest
            if idx > 0 && config.rpc_delay_ms > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(config.rpc_delay_ms)).await;
            }

            match Self::analyze_transaction_with_programs(
                rpc,
                analyzer_registry,
                rule_engine,
                signature,
                *block_time,
            )
            .await
            {
                Ok((threat_opt, programs)) => {
                    if let Some(threat) = threat_opt {
                        threats.push(threat);
                    }
                    
                    // Track program encounters
                    for (program_id, rule_decision) in programs {
                        program_encounters
                            .entry(program_id.clone())
                            .and_modify(|e| {
                                e.transaction_signatures.push(signature.to_string());
                                // Keep the most severe rule decision
                                if rule_decision.is_some() {
                                    e.rule_decision = rule_decision.clone();
                                }
                            })
                            .or_insert(ProgramEncounter {
                                program_id,
                                transaction_signatures: vec![signature.to_string()],
                                first_seen: block_time
                                    .map(|t| DateTime::from_timestamp(t, 0).unwrap_or_else(Utc::now))
                                    .unwrap_or_else(Utc::now),
                                rule_decision: rule_decision.clone(),
                            });
                    }
                }
                Err(e) => {
                    error_count += 1;
                    let error_msg = e.to_string();
                    
                    // Check if it's a rate limit error
                    if error_msg.contains("429") || error_msg.contains("Too Many Requests") {
                        // Clear progress line and show error
                        eprint!("\r");
                        eprintln!("⚠️  Rate limit hit at transaction {}/{}", progress, total);
                        eprintln!("    Waiting 5s before retry... (or stop and increase --rpc-delay-ms)");
                        warn!("RPC rate limit (429) at transaction {}: {}", signature, e);
                        
                        // Wait longer then retry this transaction
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        
                        // Retry once
                        match Self::analyze_transaction_with_programs(
                            rpc,
                            analyzer_registry,
                            rule_engine,
                            signature,
                            *block_time,
                        )
                        .await
                        {
                            Ok((threat_opt, programs)) => {
                                if let Some(threat) = threat_opt {
                                    threats.push(threat);
                                }
                                
                                for (program_id, rule_decision) in programs {
                                    program_encounters
                                        .entry(program_id.clone())
                                        .and_modify(|e| {
                                            e.transaction_signatures.push(signature.to_string());
                                            if rule_decision.is_some() {
                                                e.rule_decision = rule_decision.clone();
                                            }
                                        })
                                        .or_insert(ProgramEncounter {
                                            program_id,
                                            transaction_signatures: vec![signature.to_string()],
                                            first_seen: block_time
                                                .map(|t| DateTime::from_timestamp(t, 0).unwrap_or_else(Utc::now))
                                                .unwrap_or_else(Utc::now),
                                            rule_decision: rule_decision.clone(),
                                        });
                                }
                                error_count -= 1; // Retry succeeded
                            }
                            Err(e2) => {
                                eprintln!("❌ Retry failed: {}", e2);
                                warn!("Retry failed for transaction {}: {}", signature, e2);
                            }
                        }
                    } else {
                        warn!("Failed to analyze transaction {}: {}", signature, e);
                    }
                }
            }
        }

        // Clear progress line
        eprint!("\r");
        eprintln!("✓ Completed analysis of {} transactions{}", 
            signatures.len(),
            if error_count > 0 { 
                format!(" ({} errors, consider slower --rpc-delay-ms)", error_count) 
            } else { 
                String::new() 
            }
        );

        info!("Found {} threats in historical transactions", threats.len());
        info!("Encountered {} unique programs", program_encounters.len());
        
        Ok(HistoryScanResult {
            threats,
            program_encounters,
            transactions_analyzed: signatures.len(),
        })
    }

    /// Fetch transaction signatures for a wallet
    fn fetch_signatures(
        rpc: &RpcClient,
        wallet: &Pubkey,
        config: &ScanConfig,
    ) -> Result<Vec<(Signature, Option<i64>)>> {
        use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
        
        // Fetch signatures with limit
        let limit = config.max_transactions.unwrap_or(100);
        
        // Create config with limit
        let fetch_config = GetConfirmedSignaturesForAddress2Config {
            before: None,
            until: None,
            limit: Some(limit),
            commitment: Some(config.commitment),
        };
        
        // Fetch with limit
        let sigs = rpc.get_signatures_for_address_with_config(
            wallet,
            fetch_config,
        )?;
        
        info!("RPC returned {} signatures (requested limit: {})", sigs.len(), limit);
        
        let mut results = Vec::new();
        for sig_info in sigs.iter() {
            let signature = Signature::from_str(&sig_info.signature)?;
            let block_time = sig_info.block_time;
            
            // Filter by time window if configured
            if let Some(window_days) = config.time_window_days {
                if let Some(time) = block_time {
                    let now = chrono::Utc::now().timestamp();
                    let window_seconds = (window_days as i64) * 24 * 60 * 60;
                    if now - time > window_seconds {
                        debug!("Transaction {} too old, skipping", signature);
                        continue;
                    }
                }
            }
            
            results.push((signature, block_time));
        }
        
        Ok(results)
    }

    /// Fetch and decode a single transaction with full metadata (including inner instructions)
    async fn fetch_transaction(
        rpc: &RpcClient,
        signature: &Signature,
    ) -> Result<TransactionWithMetadata> {
        // Get transaction with Base64 encoding (for transaction) + metadata
        let tx_response = rpc.get_transaction_with_config(
            signature,
            RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::Base64),
                commitment: Some(CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
            },
        ).map_err(|e| {
            let error_str = e.to_string();
            if error_str.contains("invalid type: null, expected struct") {
                anyhow!("Transaction not found or too old: {}", signature)
            } else {
                anyhow!("RPC error fetching transaction: {}", e)
            }
        })?;

        // Extract transaction and metadata
        let tx_with_meta = tx_response.transaction;
        
        // Parse inner instructions from metadata (preserved even with Base64 encoding)
        let inner_instructions = Self::parse_inner_instructions(&tx_with_meta)?;
        
        // Decode the transaction itself
        let encoded_tx = match &tx_with_meta {
            EncodedTransactionWithStatusMeta {
                transaction: EncodedTransaction::LegacyBinary(encoded_data),
                ..
            } => encoded_data,
            EncodedTransactionWithStatusMeta {
                transaction: EncodedTransaction::Binary(encoded_data, _),
                ..
            } => encoded_data,
            _ => return Err(anyhow!("Unexpected transaction encoding")),
        };

        // Decode from base64
        let decoded_tx_data = general_purpose::STANDARD
            .decode(encoded_tx)
            .map_err(|e| anyhow!("Failed to decode transaction: {}", e))?;

        // Deserialize as versioned transaction
        let versioned_tx: VersionedTransaction = bincode::deserialize(&decoded_tx_data)
            .map_err(|e| anyhow!("Failed to deserialize transaction: {}", e))?;

        // Convert to legacy Transaction format for analyzers
        let transaction = Self::convert_to_legacy_transaction(versioned_tx)?;
        
        // Extract all program IDs from inner instructions
        let program_ids_from_inner = Self::extract_inner_program_ids(&inner_instructions, &transaction);
        
        debug!(
            "Transaction {}: {} inner instruction sets, {} unique inner programs",
            signature,
            inner_instructions.len(),
            program_ids_from_inner.len()
        );
        
        Ok(TransactionWithMetadata {
            transaction,
            inner_instructions,
            program_ids_from_inner,
        })
    }
    
    /// Parse inner instructions from transaction metadata
    fn parse_inner_instructions(
        tx_with_meta: &EncodedTransactionWithStatusMeta,
    ) -> Result<Vec<InnerInstructionSet>> {
        let mut result = Vec::new();
        
        if let Some(ref meta) = tx_with_meta.meta {
            // Handle OptionSerializer - convert to Option
            let inner_instructions_opt = match &meta.inner_instructions {
                OptionSerializer::Some(inner) => Some(inner),
                OptionSerializer::None | OptionSerializer::Skip => None,
            };
            
            if let Some(inner_instructions) = inner_instructions_opt {
                for ui_inner in inner_instructions {
                    let mut parsed_instructions = Vec::new();
                    
                    for ui_instruction in &ui_inner.instructions {
                        if let Some(parsed) = Self::parse_ui_instruction(ui_instruction) {
                            parsed_instructions.push(parsed);
                        }
                    }
                    
                    result.push(InnerInstructionSet {
                        index: ui_inner.index,
                        instructions: parsed_instructions,
                    });
                }
            }
        }
        
        Ok(result)
    }
    
    /// Parse a UI instruction into our format
    fn parse_ui_instruction(ui_inst: &UiInstruction) -> Option<ParsedInnerInstruction> {
        match ui_inst {
            UiInstruction::Compiled(compiled) => {
                let data = bs58::decode(&compiled.data)
                    .into_vec()
                    .ok()?;
                
                Some(ParsedInnerInstruction {
                    program_id: compiled.program_id_index.to_string(),
                    accounts: compiled.accounts.clone(),
                    data,
                })
            }
            _ => None, // Skip parsed instructions for now
        }
    }
    
    /// Extract program IDs from inner instructions
    fn extract_inner_program_ids(
        inner_instructions: &[InnerInstructionSet],
        transaction: &Transaction,
    ) -> Vec<String> {
        let mut program_ids = std::collections::HashSet::new();
        
        for inner_set in inner_instructions {
            for inner_inst in &inner_set.instructions {
                // Convert program_id_index to actual pubkey
                if let Ok(idx) = inner_inst.program_id.parse::<usize>() {
                    if let Some(pubkey) = transaction.message.account_keys.get(idx) {
                        program_ids.insert(pubkey.to_string());
                    }
                }
            }
        }
        
        program_ids.into_iter().collect()
    }

    /// Convert VersionedTransaction to legacy Transaction
    fn convert_to_legacy_transaction(versioned_tx: VersionedTransaction) -> Result<Transaction> {
        match versioned_tx.message {
            VersionedMessage::Legacy(legacy_message) => {
                // Already legacy format
                Ok(Transaction {
                    signatures: versioned_tx.signatures,
                    message: legacy_message,
                })
            }
            VersionedMessage::V0(v0_message) => {
                // Convert V0 to legacy (best effort)
                // Note: This loses address lookup table info, but should work for basic analysis
                let message = Message {
                    header: v0_message.header,
                    account_keys: v0_message.account_keys,
                    recent_blockhash: v0_message.recent_blockhash,
                    instructions: v0_message.instructions,
                };
                
                Ok(Transaction {
                    signatures: versioned_tx.signatures,
                    message,
                })
            }
        }
    }

    /// Analyze a single transaction and extract program IDs (including from inner instructions)
    async fn analyze_transaction_with_programs(
        rpc: &RpcClient,
        _analyzer_registry: &Arc<AnalyzerRegistry>,
        rule_engine: &Arc<RuleEngine>,
        signature: &Signature,
        block_time: Option<i64>,
    ) -> Result<(Option<ThreatAssessment>, Vec<(String, Option<RuleDecision>)>)> {
        // Fetch transaction with full metadata (including inner instructions)
        let tx_with_meta = Self::fetch_transaction(rpc, signature).await?;

        // Extract program IDs from both top-level and inner instructions
        let mut all_program_ids = Self::extract_program_ids(&tx_with_meta.transaction);
        
        // Add programs from inner instructions (CPIs)
        for inner_program in &tx_with_meta.program_ids_from_inner {
            if !all_program_ids.contains(inner_program) {
                all_program_ids.push(inner_program.clone());
            }
        }
        
        debug!(
            "Transaction {}: {} top-level programs, {} inner programs, {} total unique",
            signature,
            Self::extract_program_ids(&tx_with_meta.transaction).len(),
            tx_with_meta.program_ids_from_inner.len(),
            all_program_ids.len()
        );

        // Evaluate rules (RuleEngine runs analyzers internally)
        let rule_decision = rule_engine.evaluate(&tx_with_meta.transaction).await?;

        // Convert to threat assessment
        let threat = Self::rule_decision_to_threat(signature, block_time, rule_decision.clone())?;
        
        // Attach rule decision to programs if there was a threat
        let programs_with_decisions = if threat.is_some() {
            all_program_ids.into_iter()
                .map(|p| (p, Some(rule_decision.clone())))
                .collect()
        } else {
            all_program_ids.into_iter()
                .map(|p| (p, None))
                .collect()
        };

        Ok((threat, programs_with_decisions))
    }
    
    /// Extract all program IDs from top-level transaction instructions
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
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }

    /// Convert rule engine decision to threat assessment
    fn rule_decision_to_threat(
        signature: &Signature,
        block_time: Option<i64>,
        decision: RuleDecision,
    ) -> Result<Option<ThreatAssessment>> {
        // Only report blocked or alerted transactions as threats
        let severity = match decision.action {
            RuleAction::Block => Severity::Critical,
            RuleAction::Alert => Severity::High,
            RuleAction::Pass => return Ok(None),
        };

        // Build threat description from matched rules
        let threat_description = if decision.matched_rules.is_empty() {
            "Suspicious transaction detected".to_string()
        } else {
            format!(
                "Matched rules: {}",
                decision
                    .matched_rules
                    .iter()
                    .map(|r| r.rule_name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };

        let recommendation = format!(
            "Review transaction {} for potential security issues. Risk score: {}",
            signature, decision.total_risk
        );

        Ok(Some(ThreatAssessment {
            threat_type: ThreatType::SuspiciousTransaction {
                signature: signature.to_string(),
                threat_description,
                risk_score: decision.total_risk,
                timestamp: block_time,
            },
            severity,
            recommendation,
        }))
    }
}
