//! MCP Tool Implementations (Shared)
//!
//! This module contains the canonical implementations of all MCP tools.
//! Both the STDIO MCP server (`mcp/`) and HTTP MCP server (`api/`) import from here.
//!
//! **DO NOT DUPLICATE THIS CODE** - all MCP tool logic lives here.

use crate::scan_coverage::{build_rules_snapshot, build_scan_coverage, FeedSnapshotEntry};
use crate::scan_types::{DecisionSummary, TransactionScanResult};
use anyhow::Result;
use parapet_core::enrichment::EnrichmentService;
use parapet_core::rules::types::RuleDefinition;
use parapet_core::rules::{AnalyzerRegistry, FeedConfig, FeedSource, FeedUpdater, RuleEngine};
use parapet_scanner::{ScanReport, ThreatType};
use solana_client::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;

/// Merge ephemeral custom rules onto the engine after base rules are loaded.
fn merge_custom_rules(
    engine: &mut RuleEngine,
    custom_rules: Option<Vec<RuleDefinition>>,
) -> Result<()> {
    let Some(extra) = custom_rules else {
        return Ok(());
    };
    if extra.is_empty() {
        return Ok(());
    }
    let mut combined = engine.rules().to_vec();
    combined.extend(extra);
    engine.load_rules(combined).map_err(|e| {
        anyhow::anyhow!(
            "Custom rule validation failed: {}. Use list_analyzers for valid fields; read parapet://rules-guide.",
            e
        )
    })?;
    Ok(())
}

/// Markdown listing of registered analyzers and fields (for MCP `list_analyzers`).
pub fn format_analyzer_registry(registry: &AnalyzerRegistry) -> String {
    let mut out = String::from("# Available analyzers\n\n");
    let mut fields_map = registry.get_all_fields();
    let mut names: Vec<String> = fields_map.keys().cloned().collect();
    names.sort();
    for name in names {
        let fields = fields_map.remove(&name).unwrap_or_default();
        let status_line = registry
            .get(&name)
            .map(|a| {
                if a.is_available() {
                    "available".to_string()
                } else {
                    "unavailable (may require API keys or features)".to_string()
                }
            })
            .unwrap_or_else(|| "unknown".to_string());
        out.push_str(&format!("## `{}`\n", name));
        out.push_str(&format!("**Status:** {}\n\n", status_line));
        if fields.is_empty() {
            out.push_str("- *(no fields declared)*\n\n");
            continue;
        }
        for field in fields {
            out.push_str(&format!(
                "- `{}:{}` — reference as **`{}:{}`** in rule conditions\n",
                name, field, name, field
            ));
        }
        out.push('\n');
    }
    out.push_str(
        "Use **`namespace:field`** in `conditions.field`. Run `list_analyzers` whenever deployment changes.\n",
    );
    out
}

/// Parse optional `custom_rules` from MCP tool arguments JSON.
pub fn parse_custom_rules_arg(
    args: &serde_json::Value,
) -> anyhow::Result<Option<Vec<RuleDefinition>>> {
    match args.get("custom_rules") {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Array(arr)) if arr.is_empty() => Ok(None),
        Some(serde_json::Value::Array(arr)) => {
            let v = serde_json::Value::Array(arr.clone());
            serde_json::from_value(v)
                .map_err(|e| {
                    anyhow::anyhow!(
                        "custom_rules JSON parse error: {}. Read parapet://rules-guide for format.",
                        e
                    )
                })
                .map(Some)
        }
        Some(_) => Err(anyhow::anyhow!(
            "custom_rules must be a JSON array of rule objects (see parapet://rules-guide)"
        )),
    }
}

/// Initialize analyzers and rule engine (same as wallet-scanner binary)
pub async fn initialize_analyzers_and_rules(
    safe_programs_file: Option<String>,
    custom_rules: Option<Vec<RuleDefinition>>,
) -> Result<(Arc<AnalyzerRegistry>, Arc<RuleEngine>)> {
    fn register_all_analyzers(registry: &mut AnalyzerRegistry, safe_programs_file: Option<String>) {
        use parapet_core::rules::analyzers::*;
        let rpc_url = std::env::var("UPSTREAM_RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
        registry.register(Arc::new(BasicAnalyzer::new()));
        registry.register(Arc::new(CoreSecurityAnalyzer::new(
            std::collections::HashSet::new(),
        )));
        registry.register(Arc::new(TokenInstructionAnalyzer::new()));
        registry.register(Arc::new(SystemProgramAnalyzer::new()));
        registry.register(Arc::new(ProgramComplexityAnalyzer::new()));
        if let Ok(program_analyzer) = ProgramAnalyzer::with_empty_blocklists(rpc_url) {
            registry.register(Arc::new(program_analyzer));
        }

        let inner_analyzer = if let Some(ref path) = safe_programs_file {
            match InnerInstructionAnalyzer::with_custom_list(path) {
                Ok(analyzer) => {
                    log::info!("Loaded custom safe programs from: {}", path);
                    analyzer
                }
                Err(e) => {
                    log::warn!("Failed to load custom safe programs from {}: {}", path, e);
                    InnerInstructionAnalyzer::new()
                }
            }
        } else {
            InnerInstructionAnalyzer::new()
        };
        registry.register(Arc::new(inner_analyzer));

        // Register TransactionLogAnalyzer for logs:* fields
        registry.register(Arc::new(TransactionLogAnalyzer::new()));

        // Register InstructionDataAnalyzer for instruction_data:* fields (authority change detection)
        registry.register(Arc::new(
            InstructionDataAnalyzer::with_authority_fingerprints_embedded(),
        ));

        registry.register(Arc::new(HeliusIdentityAnalyzer::new()));
        registry.register(Arc::new(HeliusTransferAnalyzer::new()));
        registry.register(Arc::new(HeliusFundingAnalyzer::new()));
        registry.register(Arc::new(OtterSecVerifiedAnalyzer::new()));
        registry.register(Arc::new(JupiterTokenAnalyzer::new()));
        registry.register(Arc::new(RugcheckAnalyzer::new()));
    }

    let mut engine_registry = AnalyzerRegistry::new();
    register_all_analyzers(&mut engine_registry, safe_programs_file.clone());

    let mut engine = RuleEngine::new(engine_registry);

    // Check for rules feed configuration first (recommended)
    let rules_feed_urls = std::env::var("RULES_FEED_URLS").ok();

    if let Some(feed_urls) = rules_feed_urls {
        // Parse feed URLs
        let feed_sources: Vec<FeedSource> = feed_urls
            .split(',')
            .enumerate()
            .map(|(i, url)| FeedSource {
                url: url.trim().to_string(),
                name: Some(format!("feed-{}", i)),
                priority: i as u32,
                min_request_interval: 300, // 5 minutes
            })
            .collect();

        if !feed_sources.is_empty() {
            let poll_interval = std::env::var("RULES_FEED_POLL_INTERVAL")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(3600);

            let feed_config = FeedConfig {
                feed_sources: feed_sources.clone(),
                poll_interval,
                enabled: true,
            };

            let num_sources = feed_sources.len();

            // Fetch rules from feed on startup
            let updater = FeedUpdater::new(feed_config);
            match updater.fetch_all_sources().await {
                Ok(merged) => {
                    if let Err(e) = engine.load_rules(merged.rules) {
                        log::warn!("Failed to load rules from feed: {}", e);
                    } else {
                        log::info!(
                            "✅ Rules feed enabled - loaded from {} sources",
                            num_sources
                        );
                    }
                }
                Err(e) => {
                    log::warn!("Failed to fetch rules from feed: {}", e);
                }
            }

            let mut scanner_registry = AnalyzerRegistry::new();
            register_all_analyzers(&mut scanner_registry, safe_programs_file);

            merge_custom_rules(&mut engine, custom_rules)?;
            return Ok((Arc::new(scanner_registry), Arc::new(engine)));
        }
    }

    // Fallback to static rules file (development/testing only - production uses RULES_FEED_URLS)
    let rules_path = std::env::var("RULES_PATH").ok().or_else(|| {
        let candidates = [
            "rules/presets/comprehensive-protection.json",
            "../rules/presets/comprehensive-protection.json",
            "rules/presets/default-protection.json",
            "../rules/presets/default-protection.json",
        ];

        candidates
            .iter()
            .find(|p| std::path::Path::new(p).exists())
            .map(|s| s.to_string())
    });

    if let Some(path) = rules_path {
        engine.load_rules_from_file(&path)?;
        log::info!("📋 Loaded rules from: {}", path);
    } else {
        log::warn!("⚠️  No rules file or feed found, using minimal built-in protection");
    }

    merge_custom_rules(&mut engine, custom_rules)?;

    let mut scanner_registry = AnalyzerRegistry::new();
    register_all_analyzers(&mut scanner_registry, safe_programs_file);

    Ok((Arc::new(scanner_registry), Arc::new(engine)))
}

pub fn format_scan_summary(report: &ScanReport) -> String {
    let mut output = String::new();

    output.push_str(&format!("# Wallet Security Scan: {}\n\n", report.wallet));
    output.push_str(&format!(
        "**Security Score:** {}/100\n",
        report.security_score
    ));
    output.push_str(&format!("**Risk Level:** {}\n\n", report.risk_level));

    output.push_str("## Statistics\n");
    output.push_str(&format!(
        "- Time Range: {} days\n",
        report.stats.time_range_days
    ));
    output.push_str(&format!(
        "- Transactions Analyzed: {}\n",
        report.stats.transactions_analyzed
    ));
    output.push_str(&format!(
        "- Threats Found: {}\n",
        report.stats.threats_found
    ));

    if report.stats.threats_found > 0 {
        output.push_str(&format!("  - Critical: {}\n", report.stats.critical_count));
        output.push_str(&format!("  - High: {}\n", report.stats.high_count));
        output.push_str(&format!("  - Medium: {}\n", report.stats.medium_count));
        output.push_str(&format!("  - Low: {}\n", report.stats.low_count));
    }

    if !report.threats.is_empty() {
        output.push_str("\n## Detected Threats\n");
        for (i, threat) in report.threats.iter().enumerate() {
            output.push_str(&format!("\n### {}. {:?}\n", i + 1, threat.severity));
            match &threat.threat_type {
                ThreatType::ActiveUnlimitedDelegation {
                    token_account,
                    delegate,
                    ..
                } => {
                    output.push_str("**Type:** Active Unlimited Delegation\n");
                    output.push_str(&format!("- Token: `{}`\n", token_account));
                    output.push_str(&format!("- Delegate: `{}`\n", delegate));
                }
                ThreatType::SuspiciousTransaction {
                    signature,
                    threat_description,
                    risk_score,
                    ..
                } => {
                    output.push_str("**Type:** Suspicious Transaction\n");
                    output.push_str(&format!("- Description: {}\n", threat_description));
                    output.push_str(&format!("- Risk Score: {}/100\n", risk_score));
                    output.push_str(&format!("- Transaction: `{}`\n", signature));
                }
                _ => {}
            }
            output.push_str(&format!(
                "\n**Recommendation:** {}\n",
                threat.recommendation
            ));
        }
    }

    output
}

pub fn format_scan_detailed(report: &ScanReport) -> String {
    match serde_json::to_string_pretty(report) {
        Ok(json) => json,
        Err(_) => format_scan_summary(report),
    }
}

pub async fn analyze_program(program_id: &str, rpc_url: &str, network: &str) -> Result<String> {
    let mut output = String::new();

    output.push_str(&format!("# Program Analysis: {}\n\n", program_id));
    output.push_str(&format!("**Network:** {}\n\n", network));

    let program_pubkey = Pubkey::from_str(program_id)?;
    let rpc_client =
        RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());

    output.push_str("## On-Chain Data\n");
    match rpc_client.get_account(&program_pubkey) {
        Ok(account) => {
            output.push_str(&format!("- **Owner:** `{}`\n", account.owner));
            output.push_str(&format!(
                "- **Executable:** {}\n",
                if account.executable {
                    "Yes ✓"
                } else {
                    "No ✗"
                }
            ));
            output.push_str(&format!("- **Data Size:** {} bytes\n", account.data.len()));
            output.push_str(&format!("- **Lamports:** {}\n\n", account.lamports));
        }
        Err(e) => {
            output.push_str(&format!("⚠️ Could not fetch program account: {}\n\n", e));
        }
    }

    // Add enrichment data
    output.push_str("## Verification & Reputation\n");
    let enrichment = EnrichmentService::new();
    match enrichment.enrich_program(program_id).await {
        Ok(data) => {
            if let Some(ref helius) = data.helius {
                output.push_str(&format!(
                    "- **Helius Verified:** {}\n",
                    if helius.is_verified {
                        "✅ Yes"
                    } else {
                        "❌ No"
                    }
                ));
                if let Some(ref label) = helius.label {
                    output.push_str(&format!("- **Label:** {}\n", label));
                }
            }

            if let Some(ref ottersec) = data.ottersec {
                output.push_str(&format!(
                    "- **OtterSec Verified:** {}\n",
                    if ottersec.is_verified {
                        "✅ Yes"
                    } else {
                        "❌ No"
                    }
                ));
                if ottersec.source_available {
                    output.push_str("- **Source Code:** Available ✅\n");
                }
            }

            if data.helius.is_none() && data.ottersec.is_none() {
                output.push_str("⚠️ No verification data available\n");
            }
        }
        Err(e) => {
            output.push_str(&format!("⚠️ Could not fetch verification data: {}\n", e));
        }
    }
    output.push('\n');

    output.push_str("## Explorer Links\n");
    output.push_str(&format!(
        "- [Solscan](https://solscan.io/account/{})\n",
        program_id
    ));
    output.push_str(&format!(
        "- [Solana Explorer](https://explorer.solana.com/address/{})\n",
        program_id
    ));

    Ok(output)
}

/// Get token risk data from enrichment services (with advanced analysis)
pub async fn check_token_reputation(token_address: &str) -> Result<String> {
    let enrichment = EnrichmentService::new();
    let data = enrichment.enrich_token(token_address).await?;

    let mut output = String::new();
    let mut overall_risk_score = 0;
    let mut critical_warnings = Vec::new();

    output.push_str(&format!("# Token Reputation: {}\n\n", token_address));

    // Domain Registration (NEW!)
    if let Some(ref domain_reg) = data.domain_registration {
        output.push_str("## ✅ Blockchain Domain Verified\n");
        output.push_str(&format!("- **Domain:** `{}`\n", domain_reg.domain));
        output.push_str(&format!(
            "- **Verified:** {}\n",
            if domain_reg.verified {
                "✅ Yes"
            } else {
                "⚠️ No"
            }
        ));
        if let Some(ref reg_date) = domain_reg.registered_at {
            output.push_str(&format!("- **Registered:** {}\n", reg_date));
        }
        output.push('\n');
    }

    // Insider Trading Analysis (NEW!)
    if let Some(ref insider) = data.insider_analysis {
        if insider.risk_score > 0 {
            let emoji = match insider.risk_level.as_str() {
                "Critical" => "🚨",
                "High" => "⚠️",
                "Medium" => "⚡",
                _ => "ℹ️",
            };

            output.push_str(&format!("## {} Insider Trading Analysis\n", emoji));
            output.push_str(&format!(
                "- **Risk Level:** {} (Score: {}/100)\n",
                insider.risk_level, insider.risk_score
            ));
            output.push_str(&format!(
                "- **Trade Networks:** {} (wash trading indicator)\n",
                insider.trade_networks
            ));
            output.push_str(&format!(
                "- **Transfer Networks:** {} (holder inflation indicator)\n",
                insider.transfer_networks
            ));
            output.push_str(&format!(
                "- **Total Insiders:** {} connected wallets\n",
                insider.total_insiders
            ));
            output.push_str(&format!(
                "- **Insider Concentration:** {:.1}% of supply\n",
                insider.insider_concentration
            ));

            if !insider.warnings.is_empty() {
                output.push_str("\n**Warnings:**\n");
                for warning in &insider.warnings {
                    output.push_str(&format!("- ⚠️ {}\n", warning));
                }
            }
            output.push('\n');

            overall_risk_score += insider.risk_score;
            if insider.risk_score >= 50 {
                critical_warnings.push(format!("Insider risk: {}", insider.risk_level));
            }
        }
    }

    // Liquidity Vault Analysis (NEW!)
    if let Some(ref vault) = data.vault_analysis {
        let emoji = match vault.rugpull_risk.as_str() {
            "Critical" | "High" => "🚨",
            "Medium" => "⚠️",
            _ => "✅",
        };

        output.push_str(&format!("## {} Liquidity Analysis\n", emoji));
        output.push_str(&format!(
            "- **Locked Liquidity:** {}\n",
            if vault.has_locked_liquidity {
                "Yes ✓"
            } else {
                "No ✗"
            }
        ));
        output.push_str(&format!(
            "- **Locked Percentage:** {:.1}%\n",
            vault.locked_percentage
        ));
        output.push_str(&format!("- **Rugpull Risk:** {}\n", vault.rugpull_risk));

        if let Some(ref unlock) = vault.unlock_date {
            output.push_str(&format!("- **Earliest Unlock:** {}\n", unlock));
        }

        if !vault.lockers.is_empty() {
            output.push_str(&format!("\n**Lockers ({}):**\n", vault.total_lockers));
            for locker in &vault.lockers {
                output.push_str(&format!(
                    "- {} - {:.1}% locked",
                    locker.locker_type, locker.percentage_of_supply
                ));
                if let Some(ref unlock) = locker.unlock_date {
                    output.push_str(&format!(" until {}", unlock));
                }
                output.push('\n');
            }
        }
        output.push('\n');

        // Add to overall risk
        match vault.rugpull_risk.as_str() {
            "Critical" => {
                overall_risk_score += 40;
                critical_warnings.push("Liquidity not locked - HIGH RUGPULL RISK".to_string());
            }
            "High" => overall_risk_score += 25,
            "Medium" => overall_risk_score += 10,
            _ => {}
        }
    }

    // Rugcheck data
    if let Some(ref rugcheck) = data.rugcheck {
        output.push_str("## Rugcheck Base Analysis\n");
        output.push_str(&format!("- **Risk Score:** {}/100\n", rugcheck.score));
        output.push_str(&format!("- **Risk Level:** {}\n", rugcheck.risk_level));

        if let Some(mc) = rugcheck.market_cap {
            output.push_str(&format!("- **Market Cap:** ${:.2}\n", mc));
        }
        if let Some(liq) = rugcheck.liquidity {
            output.push_str(&format!("- **Liquidity:** ${:.2}\n", liq));
        }
        if let Some(age) = rugcheck.token_age_days {
            output.push_str(&format!("- **Token Age:** {} days\n", age));
        }
        if let Some(holders) = rugcheck.top_holders_percentage {
            output.push_str(&format!("- **Top Holders:** {:.1}%\n", holders));
        }

        if !rugcheck.risks.is_empty() {
            output.push_str("\n**Identified Risks:**\n");
            for risk in &rugcheck.risks {
                output.push_str(&format!(
                    "- [{}] {} - {}\n",
                    risk.level, risk.name, risk.description
                ));
            }
        }
        output.push('\n');
    }

    // Jupiter data
    if let Some(ref jupiter) = data.jupiter {
        output.push_str("## Market Data (Jupiter)\n");
        if let Some(price) = jupiter.price_usd {
            output.push_str(&format!("- **Price:** ${:.6}\n", price));
        }
        if let Some(vol) = jupiter.volume_24h {
            output.push_str(&format!("- **24h Volume:** ${:.2}\n", vol));
        }
        if let Some(liq) = jupiter.liquidity {
            output.push_str(&format!("- **Liquidity:** ${:.2}\n", liq));
        }
        if let Some(score) = jupiter.organic_score {
            output.push_str(&format!("- **Organic Score:** {}/100\n", score));
        }
        if jupiter.has_rugpull_indicators {
            output.push_str("- **⚠️ Rugpull Indicators:** Detected\n");
        }
        output.push('\n');
    }

    // Overall Summary
    if !critical_warnings.is_empty() {
        output.push_str("## 🚨 CRITICAL WARNINGS\n");
        for warning in &critical_warnings {
            output.push_str(&format!("- **{}**\n", warning));
        }
        output.push_str("\n**Recommendation:** DO NOT TRADE - High risk of scam/rugpull\n\n");
    } else if overall_risk_score >= 50 {
        output.push_str("## ⚠️ MODERATE RISK DETECTED\n");
        output.push_str(
            "**Recommendation:** Exercise caution - research thoroughly before trading\n\n",
        );
    } else if overall_risk_score >= 25 {
        output.push_str("## ℹ️ MINOR RISKS DETECTED\n");
        output.push_str("**Recommendation:** Proceed with normal caution\n\n");
    }

    if data.rugcheck.is_none() && data.jupiter.is_none() && data.insider_analysis.is_none() {
        output.push_str("⚠️ No reputation data available for this token\n");
    }

    Ok(output)
}

/// Check a specific transaction for security threats
pub async fn check_transaction(
    signature: &str,
    rpc_url: &str,
    custom_rules: Option<Vec<RuleDefinition>>,
) -> Result<String> {
    use base64::engine::general_purpose;
    use base64::Engine;
    use solana_client::rpc_config::{RpcTransactionConfig, UiTransactionEncoding};
    use solana_client::rpc_response::{EncodedTransaction, EncodedTransactionWithStatusMeta};
    use solana_sdk::signature::Signature;
    use solana_sdk::transaction::VersionedTransaction;

    let mut output = String::new();
    output.push_str(&format!("# Transaction Analysis: {}\n\n", signature));

    // Parse signature
    let sig = Signature::from_str(signature)?;

    // Create RPC client
    let rpc_client =
        RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());

    // Fetch transaction
    output.push_str("## Fetching Transaction...\n");
    let tx_response = rpc_client
        .get_transaction_with_config(
            &sig,
            RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::Base64),
                commitment: Some(CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
            },
        )
        .map_err(|e| anyhow::anyhow!("Failed to fetch transaction: {}", e))?;

    // Parse transaction
    let tx_with_meta = tx_response.transaction;
    let encoded_tx = match &tx_with_meta {
        EncodedTransactionWithStatusMeta {
            transaction: EncodedTransaction::LegacyBinary(encoded_data),
            ..
        } => encoded_data,
        EncodedTransactionWithStatusMeta {
            transaction: EncodedTransaction::Binary(encoded_data, _),
            ..
        } => encoded_data,
        _ => return Err(anyhow::anyhow!("Unexpected transaction encoding")),
    };

    let decoded_tx_data = general_purpose::STANDARD
        .decode(encoded_tx)
        .map_err(|e| anyhow::anyhow!("Failed to decode transaction: {}", e))?;

    let versioned_tx: VersionedTransaction = bincode::deserialize(&decoded_tx_data)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize transaction: {}", e))?;

    // Convert to legacy for analysis
    use solana_sdk::message::{Message, VersionedMessage};
    use solana_sdk::transaction::Transaction;

    let transaction = match versioned_tx.message {
        VersionedMessage::Legacy(legacy_message) => Transaction {
            signatures: versioned_tx.signatures,
            message: legacy_message,
        },
        VersionedMessage::V0(v0_message) => {
            let message = Message {
                header: v0_message.header,
                account_keys: v0_message.account_keys,
                recent_blockhash: v0_message.recent_blockhash,
                instructions: v0_message.instructions,
            };
            Transaction {
                signatures: versioned_tx.signatures,
                message,
            }
        }
        VersionedMessage::V1(_) => {
            return Err(anyhow::anyhow!(
                "V1 transaction format is not supported in this MCP tool yet"
            ));
        }
    };

    output.push_str("✓ Transaction fetched successfully\n\n");

    // Extract program IDs
    output.push_str("## Programs Invoked\n");
    let program_ids: Vec<String> = transaction
        .message
        .instructions
        .iter()
        .filter_map(|inst| {
            transaction
                .message
                .account_keys
                .get(inst.program_id_index as usize)
                .map(|pk| pk.to_string())
        })
        .collect();

    for (i, program_id) in program_ids.iter().enumerate() {
        output.push_str(&format!("{}. `{}`\n", i + 1, program_id));
    }
    output.push('\n');

    // Initialize analyzers and rules
    output.push_str("## Security Analysis\n");
    let (registry, engine) = initialize_analyzers_and_rules(None, custom_rules).await?;

    // Registry initialized and available for future extensions
    log::debug!(
        "Transaction analysis initialized with registry: {:p}",
        registry.as_ref()
    );

    // Evaluate transaction through rule engine
    let rule_decision = engine.evaluate(&transaction).await?;

    output.push_str(&format!("- **Action:** {:?}\n", rule_decision.action));
    output.push_str(&format!(
        "- **Risk Score:** {}/100\n",
        rule_decision.total_risk
    ));

    if !rule_decision.matched_rules.is_empty() {
        output.push_str(&format!(
            "\n**Matched Rules ({}):**\n",
            rule_decision.matched_rules.len()
        ));
        for matched in &rule_decision.matched_rules {
            output.push_str(&format!(
                "- [{}] {} (weight: {})\n",
                matched.action, matched.rule_name, matched.weight
            ));
            if !matched.message.is_empty() {
                output.push_str(&format!("  {}\n", matched.message));
            }
        }
    } else {
        output.push_str("\n✅ No security rules triggered - transaction appears safe\n");
    }

    output.push_str("\n## Recommendation\n");
    match rule_decision.action {
        parapet_core::rules::RuleAction::Block => {
            output.push_str("🚨 **BLOCK** - This transaction exhibits dangerous patterns and should not be executed.\n");
        }
        parapet_core::rules::RuleAction::Alert => {
            output.push_str("⚠️ **ALERT** - This transaction shows suspicious patterns. Review carefully before proceeding.\n");
        }
        parapet_core::rules::RuleAction::Pass => {
            output.push_str("✅ **PASS** - Transaction analysis complete. No major security concerns detected.\n");
        }
    }

    Ok(output)
}

/// Fetch and decode a confirmed transaction for rule evaluation.
pub async fn fetch_transaction_for_analysis(
    signature: &str,
    rpc_url: &str,
) -> Result<Transaction> {
    use base64::engine::general_purpose;
    use base64::Engine;
    use solana_client::rpc_config::{RpcTransactionConfig, UiTransactionEncoding};
    use solana_client::rpc_response::{EncodedTransaction, EncodedTransactionWithStatusMeta};
    use solana_sdk::message::{Message, VersionedMessage};
    use solana_sdk::signature::Signature;
    use solana_sdk::transaction::VersionedTransaction;

    let sig = Signature::from_str(signature)?;
    let rpc_client =
        RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());

    let tx_response = rpc_client
        .get_transaction_with_config(
            &sig,
            RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::Base64),
                commitment: Some(CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
            },
        )
        .map_err(|e| anyhow::anyhow!("Failed to fetch transaction: {}", e))?;

    let encoded_tx = match &tx_response.transaction {
        EncodedTransactionWithStatusMeta {
            transaction: EncodedTransaction::LegacyBinary(encoded_data),
            ..
        } => encoded_data,
        EncodedTransactionWithStatusMeta {
            transaction: EncodedTransaction::Binary(encoded_data, _),
            ..
        } => encoded_data,
        _ => return Err(anyhow::anyhow!("Unexpected transaction encoding")),
    };

    let decoded_tx_data = general_purpose::STANDARD
        .decode(encoded_tx)
        .map_err(|e| anyhow::anyhow!("Failed to decode transaction: {}", e))?;

    let versioned_tx: VersionedTransaction = bincode::deserialize(&decoded_tx_data)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize transaction: {}", e))?;

    Ok(match versioned_tx.message {
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
        VersionedMessage::V1(_) => {
            return Err(anyhow::anyhow!(
                "V1 transaction format is not supported yet"
            ));
        }
    })
}

pub fn rules_loaded_labels() -> Vec<String> {
    if let Ok(urls) = std::env::var("RULES_FEED_URLS") {
        return urls
            .split(',')
            .filter_map(|u| {
                let u = u.trim();
                if u.is_empty() {
                    return None;
                }
                Some(
                    u.rsplit('/')
                        .next()
                        .unwrap_or(u)
                        .trim_end_matches(".json")
                        .to_string(),
                )
            })
            .collect();
    }
    std::env::var("RULES_PATH")
        .ok()
        .map(|p| vec![p])
        .unwrap_or_else(|| vec!["default".to_string()])
}

pub fn rules_snapshot_from_engine(engine: &RuleEngine) -> crate::scan_coverage::RulesSnapshot {
    let rule_count = engine.rules().len();
    let feeds = if let Ok(urls) = std::env::var("RULES_FEED_URLS") {
        urls.split(',')
            .enumerate()
            .map(|(i, url)| {
                let url = url.trim().to_string();
                let name = url
                    .rsplit('/')
                    .next()
                    .unwrap_or("feed")
                    .trim_end_matches(".json")
                    .to_string();
                FeedSnapshotEntry {
                    url: Some(url),
                    name,
                    version: "1.0".to_string(),
                    published_at: None,
                    rule_count: rule_count / urls.split(',').count().max(1),
                }
            })
            .collect()
    } else {
        vec![FeedSnapshotEntry {
            url: std::env::var("RULES_PATH").ok(),
            name: "local-rules".to_string(),
            version: "1.0".to_string(),
            published_at: None,
            rule_count,
        }]
    };
    build_rules_snapshot(None, feeds, rule_count)
}

/// Structured transaction scan for REST API and JSON consumers.
pub async fn check_transaction_structured(
    scan_id: String,
    signature: &str,
    rpc_url: &str,
    custom_rules: Option<Vec<RuleDefinition>>,
    include_analyzer_fields: bool,
) -> Result<TransactionScanResult> {
    let start = Instant::now();
    let transaction = fetch_transaction_for_analysis(signature, rpc_url).await?;

    let programs: Vec<String> = transaction
        .message
        .instructions
        .iter()
        .filter_map(|inst| {
            transaction
                .message
                .account_keys
                .get(inst.program_id_index as usize)
                .map(|pk| pk.to_string())
        })
        .collect();

    let (registry, engine) = initialize_analyzers_and_rules(None, custom_rules).await?;
    let required = engine.get_required_analyzers();
    let analyzer_field_map = registry
        .analyze_selected(&transaction, &required)
        .await
        .unwrap_or_default();
    let rule_decision = engine.evaluate(&transaction).await?;
    let rules_loaded = rules_loaded_labels();
    let scan_coverage = build_scan_coverage(
        &required,
        &analyzer_field_map,
        registry.as_ref(),
        rules_loaded.clone(),
    );
    let rules_snapshot = rules_snapshot_from_engine(engine.as_ref());

    let analyzer_fields = if include_analyzer_fields {
        Some(
            analyzer_field_map
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
    } else {
        None
    };

    Ok(TransactionScanResult {
        scan_id,
        signature: signature.to_string(),
        decision: DecisionSummary::from_decision(&rule_decision),
        programs,
        analysis_time_ms: start.elapsed().as_millis() as u64,
        scan_coverage,
        rules_snapshot,
        analyzer_fields,
    })
}

/// Get program verification status
pub async fn verify_program_status(program_address: &str) -> Result<String> {
    let enrichment = EnrichmentService::new();
    let data = enrichment.enrich_program(program_address).await?;

    let mut output = String::new();
    output.push_str(&format!("# Program Verification: {}\n\n", program_address));

    if let Some(ref helius) = data.helius {
        output.push_str("## Helius Identity\n");
        output.push_str(&format!(
            "- **Verified:** {}\n",
            if helius.is_verified {
                "✅ Yes"
            } else {
                "❌ No"
            }
        ));
        if let Some(ref verifier) = helius.verifier {
            output.push_str(&format!("- **Verifier:** {}\n", verifier));
        }
        if let Some(ref label) = helius.label {
            output.push_str(&format!("- **Label:** {}\n", label));
        }
        if let Some(risk) = helius.risk_score {
            output.push_str(&format!("- **Risk Score:** {}/100\n", risk));
        }
        output.push('\n');
    }

    if let Some(ref ottersec) = data.ottersec {
        output.push_str("## OtterSec Verification\n");
        output.push_str(&format!(
            "- **Verified:** {}\n",
            if ottersec.is_verified {
                "✅ Yes"
            } else {
                "❌ No"
            }
        ));
        if let Some(ref level) = ottersec.verification_level {
            output.push_str(&format!("- **Level:** {}\n", level));
        }
        if let Some(ref date) = ottersec.audit_date {
            output.push_str(&format!("- **Audit Date:** {}\n", date));
        }
        output.push_str(&format!(
            "- **Source Available:** {}\n",
            if ottersec.source_available {
                "✅ Yes"
            } else {
                "❌ No"
            }
        ));
        output.push('\n');
    }

    if data.helius.is_none() && data.ottersec.is_none() {
        output.push_str("⚠️ No verification data available for this program\n");
    }

    Ok(output)
}
