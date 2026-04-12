use anyhow::{anyhow, Result};
use base64::Engine;
use clap::Parser;
use colored::Colorize;
use parapet_core::rules::analyzer::{ConfirmedInnerInstruction, ConfirmedTransactionMetadata};
use parapet_core::rules::analyzers::*;
use parapet_core::rules::{AnalyzerRegistry, RuleEngine};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::message::{Message, VersionedMessage};
use solana_sdk::signature::Signature;
use solana_sdk::transaction::{Transaction, VersionedTransaction};
use solana_transaction_status::{
    option_serializer::OptionSerializer, EncodedTransaction, EncodedTransactionWithStatusMeta,
    UiCompiledInstruction, UiInstruction, UiTransactionEncoding,
};
use std::str::FromStr;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "tx-check")]
#[command(about = "Evaluate a Solana transaction signature against parapet security rules")]
#[command(long_about = "\
Fetches a transaction from the RPC, decodes it, and runs it through the \
parapet rule engine. Prints which rules matched, the action taken \
(block / alert / pass), and the total risk score.\n\
\n\
Examples:\n\
  tx-check 4BKBmAJn6TdsENij7... \\\n\
    --rules ./rules/presets/default-protection.json\n\
\n\
  tx-check 4BKBmAJn6TdsENij7... \\\n\
    --rules ../proxy/tests/fixtures/rules/presets/drift-exploit-protection.json \\\n\
    --rpc-url https://api.mainnet-beta.solana.com")]
struct Args {
    /// Transaction signature (base58)
    #[arg(value_name = "SIGNATURE")]
    signature: String,

    /// Rules JSON file to evaluate against
    #[arg(short, long, default_value = "./rules/presets/default-protection.json")]
    rules: String,

    /// Solana RPC endpoint URL
    #[arg(long, default_value = "https://api.mainnet-beta.solana.com")]
    rpc_url: String,

    /// Rule-engine blocking threshold (0-100)
    #[arg(long, default_value_t = 70)]
    threshold: u8,

    /// Output format: pretty or json
    #[arg(short, long, default_value = "pretty")]
    format: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let args = Args::parse();

    if args.format == "pretty" {
        println!();
        println!(
            "{}",
            "═══════════════════════════════════════════════════════════"
                .bright_blue()
                .bold()
        );
        println!(
            "{}",
            "              Parapet Transaction Checker"
                .bright_blue()
                .bold()
        );
        println!(
            "{}",
            "═══════════════════════════════════════════════════════════"
                .bright_blue()
                .bold()
        );
        println!();
        println!("  Signature : {}", args.signature.bright_cyan());
        println!("  Rules     : {}", args.rules.bright_cyan());
        println!("  Threshold : {}", args.threshold.to_string().bright_cyan());
        println!("  RPC       : {}", args.rpc_url.bright_cyan());
        println!();
    }

    // --- 1. Fetch transaction from RPC ---
    if args.format == "pretty" {
        println!("{}", "Fetching transaction...".dimmed());
    }

    let rpc = RpcClient::new_with_commitment(args.rpc_url.clone(), CommitmentConfig::confirmed());
    let signature = Signature::from_str(&args.signature)
        .map_err(|e| anyhow!("Invalid signature '{}': {}", args.signature, e))?;

    let tx_response = rpc
        .get_transaction_with_config(
            &signature,
            RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::Base64),
                commitment: Some(CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
            },
        )
        .map_err(|e| {
            let s = e.to_string();
            if s.contains("invalid type: null") {
                anyhow!("Transaction not found (too old or wrong network?)")
            } else {
                anyhow!("RPC error: {}", e)
            }
        })?;

    // --- 2. Decode transaction bytes ---
    let tx_with_meta = tx_response.transaction;

    // Extract log messages now (don't need decoded tx for this)
    let log_messages: Vec<String> = tx_with_meta
        .meta
        .as_ref()
        .and_then(|meta| match &meta.log_messages {
            OptionSerializer::Some(logs) => Some(logs.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let encoded_data = match &tx_with_meta {
        EncodedTransactionWithStatusMeta {
            transaction: EncodedTransaction::LegacyBinary(data),
            ..
        } => data.clone(),
        EncodedTransactionWithStatusMeta {
            transaction: EncodedTransaction::Binary(data, _),
            ..
        } => data.clone(),
        _ => return Err(anyhow!("Unexpected transaction encoding from RPC")),
    };

    let raw_bytes = base64::engine::general_purpose::STANDARD
        .decode(&encoded_data)
        .map_err(|e| anyhow!("Failed to base64-decode transaction: {}", e))?;

    let versioned_tx: VersionedTransaction = bincode::deserialize(&raw_bytes)
        .map_err(|e| anyhow!("Failed to deserialize transaction: {}", e))?;

    // Convert versioned → legacy Transaction for the rule engine
    let transaction = match versioned_tx.message {
        VersionedMessage::Legacy(legacy_message) => Transaction {
            signatures: versioned_tx.signatures,
            message: legacy_message,
        },
        VersionedMessage::V0(v0_message) => Transaction {
            signatures: versioned_tx.signatures,
            message: Message {
                header: v0_message.header,
                account_keys: v0_message.account_keys,
                recent_blockhash: v0_message.recent_blockhash,
                instructions: v0_message.instructions,
            },
        },
    };

    // Parse inner (CPI) instructions — needs account_keys for program ID resolution
    let account_keys: Vec<String> = transaction
        .message
        .account_keys
        .iter()
        .map(|pk| pk.to_string())
        .collect();

    let inner_instructions: Vec<ConfirmedInnerInstruction> = tx_with_meta
        .meta
        .as_ref()
        .and_then(|meta| match &meta.inner_instructions {
            OptionSerializer::Some(inner) => Some(inner.clone()),
            _ => None,
        })
        .unwrap_or_default()
        .into_iter()
        .flat_map(|set| {
            let outer_index = set.index;
            let keys = account_keys.clone();
            set.instructions.into_iter().filter_map(move |ix| {
                if let UiInstruction::Compiled(UiCompiledInstruction {
                    program_id_index,
                    accounts,
                    data,
                    stack_height,
                }) = ix
                {
                    let data_bytes = bs58::decode(&data).into_vec().unwrap_or_default();
                    let program_id = keys
                        .get(program_id_index as usize)
                        .cloned()
                        .unwrap_or_default();
                    Some(ConfirmedInnerInstruction {
                        outer_index,
                        program_id,
                        data: data_bytes,
                        accounts,
                        stack_height: stack_height.map(|h| h as u8),
                    })
                } else {
                    None
                }
            })
        })
        .collect();

    let tx_metadata = ConfirmedTransactionMetadata {
        logs: log_messages.clone(),
        inner_instructions,
    };

    if args.format == "pretty" {
        println!("{}", "Transaction decoded successfully.".dimmed());
        println!();
        println!(
            "  Instructions : {}",
            transaction
                .message
                .instructions
                .len()
                .to_string()
                .bright_white()
        );
        let programs: Vec<String> = transaction
            .message
            .instructions
            .iter()
            .filter_map(|ix| {
                transaction
                    .message
                    .account_keys
                    .get(ix.program_id_index as usize)
                    .map(|pk| pk.to_string())
            })
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        println!("  Programs     : {}", programs.join(", ").bright_white());
        println!(
            "  Log lines    : {}",
            log_messages.len().to_string().bright_white()
        );
        println!(
            "  CPI calls    : {}",
            tx_metadata
                .inner_instructions
                .len()
                .to_string()
                .bright_white()
        );
        println!();
    }

    // --- 3. Build rule engine ---
    let mut registry = AnalyzerRegistry::new();
    registry.register(Arc::new(BasicAnalyzer::new()));
    registry.register(Arc::new(CoreSecurityAnalyzer::new(
        std::collections::HashSet::new(),
    )));
    registry.register(Arc::new(TokenInstructionAnalyzer::new()));
    registry.register(Arc::new(SystemProgramAnalyzer::new()));
    registry.register(Arc::new(ProgramComplexityAnalyzer::new()));
    registry.register(Arc::new(TransactionLogAnalyzer::new()));

    // Load instruction fingerprints from config alongside the rules file, or use defaults
    {
        let fingerprint_path = std::path::Path::new(&args.rules)
            .parent()
            .and_then(|p| p.parent())
            .map(|base| base.join("fingerprints/authority-change.json"));

        let analyzer = match fingerprint_path.as_deref() {
            Some(path) if path.exists() => {
                InstructionDataAnalyzer::from_config_file(path.to_str().unwrap_or(""))
                    .unwrap_or_else(|_| {
                        InstructionDataAnalyzer::with_authority_fingerprints_embedded()
                    })
            }
            _ => InstructionDataAnalyzer::with_authority_fingerprints_embedded(),
        };
        registry.register(Arc::new(analyzer));
    }

    let mut engine = RuleEngine::new(registry);
    engine
        .load_rules_from_file(&args.rules)
        .map_err(|e| anyhow!("Failed to load rules from '{}': {}", args.rules, e))?;

    if args.format == "pretty" {
        println!(
            "  Loaded {} rules from {}",
            engine.enabled_rule_count(),
            args.rules.bright_cyan()
        );
        println!();
        println!("{}", "Evaluating...".dimmed());
        println!();
    }

    // --- 4. Optional: dump all analyzer fields (debug-fields feature) ---
    #[cfg(feature = "debug-fields")]
    {
        let mut debug_registry = AnalyzerRegistry::new();
        debug_registry.register(Arc::new(BasicAnalyzer::new()));
        debug_registry.register(Arc::new(CoreSecurityAnalyzer::new(
            std::collections::HashSet::new(),
        )));
        debug_registry.register(Arc::new(TokenInstructionAnalyzer::new()));
        debug_registry.register(Arc::new(SystemProgramAnalyzer::new()));
        debug_registry.register(Arc::new(ProgramComplexityAnalyzer::new()));
        debug_registry.register(Arc::new(TransactionLogAnalyzer::new()));
        debug_registry.register(Arc::new(
            InstructionDataAnalyzer::with_authority_fingerprints_embedded(),
        ));

        let all_analyzers: Vec<String> = debug_registry.list_all();
        let fields = debug_registry
            .analyze_selected_with_metadata(&transaction, &all_analyzers, &tx_metadata)
            .await?;
        let mut sorted: Vec<_> = fields.iter().collect();
        sorted.sort_by_key(|(k, _)| k.as_str());

        if args.format == "json" {
            let map: serde_json::Map<String, serde_json::Value> = sorted
                .iter()
                .map(|(k, v)| (k.to_string(), (*v).clone()))
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::Value::Object(map))?
            );
            return Ok(());
        }

        println!(
            "{}",
            "─────────────────────────────────────────────────────────────".bright_blue()
        );
        println!("{}", "  ANALYZER FIELDS".bold());
        println!(
            "{}",
            "─────────────────────────────────────────────────────────────".bright_blue()
        );
        for (k, v) in &sorted {
            println!(
                "  {:<55} {}",
                k.bright_white(),
                v.to_string().bright_yellow()
            );
        }
        println!();
    }

    // --- 5. Evaluate with full metadata (logs + CPI inner instructions) ---
    let decision = engine
        .evaluate_with_metadata_and_threshold(&transaction, &tx_metadata, args.threshold)
        .await?;

    // --- 6. Output ---
    if args.format == "json" {
        let out = serde_json::json!({
            "signature": args.signature,
            "rules_file": args.rules,
            "matched": decision.matched,
            "action": format!("{:?}", decision.action).to_lowercase(),
            "message": decision.message,
            "total_risk": decision.total_risk,
            "threshold": args.threshold,
            "matched_rules": decision.matched_rules.iter().map(|r| serde_json::json!({
                "id": r.rule_id,
                "name": r.rule_name,
                "action": format!("{:?}", r.action).to_lowercase(),
                "weight": r.weight,
                "message": r.message,
            })).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    // Pretty output
    println!(
        "{}",
        "─────────────────────────────────────────────────────────────".bright_blue()
    );
    println!("{}", "  RESULT".bold());
    println!(
        "{}",
        "─────────────────────────────────────────────────────────────".bright_blue()
    );
    println!();

    if !decision.matched {
        println!("  {} — no rules matched", "PASS".bright_green().bold());
        println!(
            "  Risk score : {}/100",
            decision.total_risk.to_string().bright_green()
        );
    } else {
        use parapet_core::rules::types::RuleAction;
        let action_str = match decision.action {
            RuleAction::Block => "BLOCK".bright_red().bold(),
            RuleAction::Alert => "ALERT".bright_yellow().bold(),
            RuleAction::Pass => "PASS".bright_green().bold(),
        };
        println!("  {} — {}", action_str, decision.message);
        println!(
            "  Risk score : {}/100  (threshold: {})",
            decision.total_risk.to_string().bright_red(),
            args.threshold
        );
        println!();

        if !decision.matched_rules.is_empty() {
            println!("  Matched rules:");
            for rule in &decision.matched_rules {
                use parapet_core::rules::types::RuleAction;
                let tag = match rule.action {
                    RuleAction::Block => "[BLOCK]".bright_red(),
                    RuleAction::Alert => "[ALERT]".bright_yellow(),
                    RuleAction::Pass => "[PASS]".bright_green(),
                };
                println!(
                    "    {} {} — {}",
                    tag,
                    rule.rule_id.bright_white(),
                    rule.message
                );
            }
        }
    }

    println!();
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════"
            .bright_blue()
            .bold()
    );
    println!();

    // Exit with non-zero code if blocked so it can be used in scripts
    use parapet_core::rules::types::RuleAction;
    if decision.matched && decision.action == RuleAction::Block {
        std::process::exit(1);
    }

    Ok(())
}
