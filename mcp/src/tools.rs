use anyhow::Result;
use parapet_scanner::{ScanReport, ThreatType};
use parapet_core::rules::{AnalyzerRegistry, RuleEngine};
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;

/// Initialize analyzers and rule engine (same as wallet-scanner binary)
pub fn initialize_analyzers_and_rules(
    safe_programs_file: Option<String>,
) -> Result<(Arc<AnalyzerRegistry>, Arc<RuleEngine>)> {
    use parapet_core::rules::analyzers::*;

    // Helper to register analyzers
    fn register_all_analyzers(
        registry: &mut AnalyzerRegistry,
        safe_programs_file: Option<String>,
    ) {
        // Register built-in core analyzers
        registry.register(Arc::new(BasicAnalyzer::new()));
        registry.register(Arc::new(CoreSecurityAnalyzer::new(
            std::collections::HashSet::new(),
        )));
        registry.register(Arc::new(TokenInstructionAnalyzer::new()));
        registry.register(Arc::new(SystemProgramAnalyzer::new()));
        registry.register(Arc::new(ProgramComplexityAnalyzer::new()));

        // Deep scanning: Inner instruction (CPI) analysis
        let inner_analyzer = if let Some(ref path) = safe_programs_file {
            match InnerInstructionAnalyzer::with_custom_list(path) {
                Ok(analyzer) => {
                    log::info!("Loaded custom safe programs from: {}", path);
                    analyzer
                }
                Err(e) => {
                    log::warn!(
                        "Failed to load custom safe programs from {}: {}",
                        path,
                        e
                    );
                    InnerInstructionAnalyzer::new()
                }
            }
        } else {
            InnerInstructionAnalyzer::new()
        };
        registry.register(Arc::new(inner_analyzer));

        // Register third-party analyzers
        registry.register(Arc::new(HeliusIdentityAnalyzer::new()));
        registry.register(Arc::new(HeliusTransferAnalyzer::new()));
        registry.register(Arc::new(HeliusFundingAnalyzer::new()));
        registry.register(Arc::new(OtterSecVerifiedAnalyzer::new()));
        registry.register(Arc::new(JupiterTokenAnalyzer::new()));
        registry.register(Arc::new(RugcheckAnalyzer::new()));
    }

    // Create registry for rule engine
    let mut engine_registry = AnalyzerRegistry::new();
    register_all_analyzers(&mut engine_registry, safe_programs_file.clone());

    // Create rule engine
    let mut engine = RuleEngine::new(engine_registry);

    // Load rules from default location or environment
    let rules_path = std::env::var("RULES_PATH")
        .ok()
        .or_else(|| {
            let enhanced_candidates = vec![
                "proxy/rules/presets/wallet-scan-enhanced.json",
                "../proxy/rules/presets/wallet-scan-enhanced.json",
                "../../proxy/rules/presets/wallet-scan-enhanced.json",
            ];

            let fallback_candidates = vec![
                "proxy/rules/presets/bot-essentials.json",
                "../proxy/rules/presets/bot-essentials.json",
                "../../proxy/rules/presets/bot-essentials.json",
            ];

            enhanced_candidates
                .iter()
                .chain(fallback_candidates.iter())
                .find(|p| std::path::Path::new(p).exists())
                .map(|s| s.to_string())
        });

    if let Some(path) = rules_path {
        engine.load_rules_from_file(&path)?;
        log::info!("Loaded rules from: {}", path);
    } else {
        log::warn!("No rules file found, using minimal built-in protection");
    }

    // Create separate registry for scanner
    let mut scanner_registry = AnalyzerRegistry::new();
    register_all_analyzers(&mut scanner_registry, safe_programs_file);

    Ok((Arc::new(scanner_registry), Arc::new(engine)))
}

/// Format scan report as summary
pub fn format_scan_summary(report: &ScanReport) -> String {
    let mut output = String::new();

    output.push_str(&format!("# Wallet Security Scan: {}\n\n", report.wallet));
    output.push_str(&format!("**Security Score:** {}/100\n", report.security_score));
    output.push_str(&format!("**Risk Level:** {}\n\n", report.risk_level));

    // Statistics
    output.push_str("## Statistics\n");
    output.push_str(&format!("- Time Range: {} days\n", report.stats.time_range_days));
    output.push_str(&format!(
        "- Transactions Analyzed: {}\n",
        report.stats.transactions_analyzed
    ));
    output.push_str(&format!("- Threats Found: {}\n", report.stats.threats_found));

    if report.stats.threats_found > 0 {
        output.push_str(&format!(
            "  - Critical: {}\n",
            report.stats.critical_count
        ));
        output.push_str(&format!("  - High: {}\n", report.stats.high_count));
        output.push_str(&format!("  - Medium: {}\n", report.stats.medium_count));
        output.push_str(&format!("  - Low: {}\n", report.stats.low_count));
    }

    // Threats
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
                ThreatType::PossibleExploitedDelegation {
                    token_account,
                    delegate,
                    ..
                } => {
                    output.push_str("**Type:** Possibly Exploited Delegation\n");
                    output.push_str(&format!("- Token: `{}`\n", token_account));
                    output.push_str(&format!("- Delegate: `{}`\n", delegate));
                }
                ThreatType::CompromisedAuthority {
                    account,
                    expected_owner,
                    actual_owner,
                } => {
                    output.push_str("**Type:** Compromised Authority\n");
                    output.push_str(&format!("- Account: `{}`\n", account));
                    output.push_str(&format!(
                        "- Expected: `{}` → Actual: `{}`\n",
                        expected_owner, actual_owner
                    ));
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
                ThreatType::UnusualPattern {
                    pattern_description,
                    occurrences,
                    ..
                } => {
                    output.push_str("**Type:** Unusual Pattern\n");
                    output.push_str(&format!("- Description: {}\n", pattern_description));
                    output.push_str(&format!("- Occurrences: {}\n", occurrences));
                }
            }
            output.push_str(&format!("\n**Recommendation:** {}\n", threat.recommendation));
        }
    }

    // Suspicious Programs
    if !report.suspicious_programs.is_empty() {
        output.push_str("\n## Suspicious Programs\n");
        for program in &report.suspicious_programs {
            output.push_str(&format!("\n### {}\n", program.program_id));
            output.push_str(&format!("- Risk Score: {}/100\n", program.risk_score));
            output.push_str(&format!("- Type: {}\n", program.threat_type));
            output.push_str(&format!("- Occurrences: {}\n", program.occurrence_count));
            output.push_str(&format!("- Summary: {}\n", program.analysis_summary));
            output.push_str(&format!("- **Recommendation:** {}\n", program.recommendation));
        }
    }

    // Final Recommendation
    output.push_str("\n## Overall Recommendation\n");
    match report.security_score {
        0..=30 => {
            output.push_str("🚨 **CRITICAL:** This wallet shows signs of compromise!\n\n");
            output.push_str("Immediate Actions:\n");
            output.push_str("1. Stop using this wallet immediately\n");
            output.push_str("2. Create a new wallet with a new seed phrase\n");
            output.push_str("3. Transfer remaining funds to the new wallet\n");
            output.push_str("4. Revoke all token delegations\n");
        }
        31..=50 => {
            output.push_str("⚠️ **HIGH RISK:** Multiple security concerns detected\n\n");
            output.push_str("Recommended Actions:\n");
            output.push_str("1. Review all detected threats carefully\n");
            output.push_str("2. Revoke suspicious token delegations\n");
            output.push_str("3. Consider moving funds to a new wallet\n");
        }
        51..=75 => {
            output.push_str("⚠️ **MODERATE RISK:** Some security concerns found\n\n");
            output.push_str("Recommended Actions:\n");
            output.push_str("1. Review the detected issues\n");
            output.push_str("2. Revoke unnecessary delegations\n");
            output.push_str("3. Be cautious with future transactions\n");
        }
        76..=90 => {
            output.push_str("✓ **LOW RISK:** Minor concerns detected\n\n");
            output.push_str("Suggested Actions:\n");
            output.push_str("1. Review low-priority items\n");
            output.push_str("2. Continue monitoring wallet activity\n");
        }
        _ => {
            output.push_str("✅ **SAFE:** No security threats detected\n\n");
            output.push_str("Your wallet appears secure. Continue best practices.\n");
        }
    }

    output
}

/// Format scan report with full details
pub fn format_scan_detailed(report: &ScanReport) -> String {
    match serde_json::to_string_pretty(report) {
        Ok(json) => json,
        Err(_) => format_scan_summary(report),
    }
}

/// Analyze a program
pub async fn analyze_program(
    program_id: &str,
    rpc_url: &str,
    network: &str,
) -> Result<String> {
    let mut output = String::new();

    output.push_str(&format!("# Program Analysis: {}\n\n", program_id));
    output.push_str(&format!("**Network:** {}\n", network));
    output.push_str(&format!("**RPC:** {}\n\n", rpc_url));

    // Validate program ID
    let program_pubkey = Pubkey::from_str(program_id)?;

    // Initialize RPC client
    let rpc_client = RpcClient::new_with_commitment(
        rpc_url.to_string(),
        CommitmentConfig::confirmed(),
    );

    // Check on-chain data
    output.push_str("## On-Chain Data\n");
    match rpc_client.get_account(&program_pubkey) {
        Ok(account) => {
            output.push_str(&format!("- **Owner:** `{}`\n", account.owner));
            output.push_str(&format!(
                "- **Executable:** {}\n",
                if account.executable { "Yes ✓" } else { "No ✗" }
            ));
            output.push_str(&format!("- **Data Size:** {} bytes\n", account.data.len()));
            output.push_str(&format!("- **Lamports:** {}\n\n", account.lamports));
        }
        Err(e) => {
            output.push_str(&format!("⚠️ Could not fetch program account: {}\n\n", e));
        }
    }

    // OtterSec Verification Check
    output.push_str("## OtterSec Verification\n");
    match check_ottersec_verification(program_id).await {
        Ok(result) => output.push_str(&result),
        Err(e) => output.push_str(&format!("⚠️ Could not check verification: {}\n\n", e)),
    }

    // Helius Identity Check
    if std::env::var("HELIUS_API_KEY").is_ok() {
        output.push_str("## Helius Identity\n");
        match check_helius_identity(program_id).await {
            Ok(result) => output.push_str(&result),
            Err(e) => output.push_str(&format!("⚠️ Could not check identity: {}\n\n", e)),
        }
    } else {
        output.push_str("## Helius Identity\n");
        output.push_str("💡 Set HELIUS_API_KEY for identity checks\n\n");
    }

    // Explorer Links
    output.push_str("## Explorer Links\n");
    output.push_str(&format!(
        "- [Solscan](https://solscan.io/account/{})\n",
        program_id
    ));
    output.push_str(&format!(
        "- [Solana Explorer](https://explorer.solana.com/address/{})\n",
        program_id
    ));
    output.push_str(&format!(
        "- [SolanaFM](https://solana.fm/address/{})\n",
        program_id
    ));

    Ok(output)
}

async fn check_ottersec_verification(program_id: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    let url = format!("https://verify.osec.io/status/{}", program_id);

    match client.get(&url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                let body: serde_json::Value = response.json().await?;
                let is_verified = body["is_verified"].as_bool().unwrap_or(false);
                let message = body["message"].as_str().unwrap_or("Unknown");

                let mut result = String::new();
                if is_verified {
                    result.push_str("✅ **Program is VERIFIED**\n");
                    if let Some(repo_url) = body["repo_url"].as_str() {
                        result.push_str(&format!("- Source: {}\n", repo_url));
                    }
                    if let Some(verified_at) = body["last_verified_at"].as_str() {
                        result.push_str(&format!("- Last verified: {}\n", verified_at));
                    }
                } else {
                    result.push_str("⚠️ **Program is NOT verified**\n");
                    result.push_str(&format!("- {}\n", message));
                }
                result.push('\n');
                Ok(result)
            } else if response.status().as_u16() == 404 {
                Ok("⚠️ Program not found in OtterSec database\n\n".to_string())
            } else {
                Ok(format!("⚠️ API error: {}\n\n", response.status()))
            }
        }
        Err(e) => Ok(format!("⚠️ Could not reach OtterSec API: {}\n\n", e)),
    }
}

async fn check_helius_identity(program_id: &str) -> Result<String> {
    let api_key = std::env::var("HELIUS_API_KEY")?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    let url = format!(
        "https://api.helius.xyz/v1/wallet/batch-identity?api-key={}",
        api_key
    );

    let response = client
        .post(&url)
        .json(&serde_json::json!({
            "addresses": [program_id]
        }))
        .send()
        .await?;

    let mut result = String::new();
    if response.status().is_success() {
        let body: serde_json::Value = response.json().await?;
        if let Some(identity) = body.as_array().and_then(|arr| arr.first()) {
            if let Some(name) = identity["name"].as_str() {
                result.push_str(&format!("- **Name:** {}\n", name));
            }
            if let Some(category) = identity["category"].as_str() {
                result.push_str(&format!("- **Category:** {}\n", category));
            }
            if let Some(identity_type) = identity["type"].as_str() {
                result.push_str(&format!("- **Type:** {}\n", identity_type));
            }
            if let Some(tags) = identity["tags"].as_array() {
                let tag_strings: Vec<String> = tags
                    .iter()
                    .filter_map(|t| t.as_str())
                    .map(|s| s.to_string())
                    .collect();
                if !tag_strings.is_empty() {
                    result.push_str(&format!("- **Tags:** {}\n", tag_strings.join(", ")));
                }
            }

            if identity["name"].is_null() && identity["category"].is_null() {
                result.push_str("ℹ️ No identity information available\n");
            }
        }
    } else {
        result.push_str(&format!("⚠️ API error: {}\n", response.status()));
    }
    result.push('\n');
    Ok(result)
}

/// Handle analyze_phishing_site tool call
pub async fn handle_analyze_phishing_site(args: serde_json::Value) -> Result<serde_json::Value> {
    use std::process::Command;
    
    let url = args["url"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: url"))?;
    
    let timeout = args["timeout"].as_u64().unwrap_or(30000);
    let max_steps = args["max_steps"].as_u64().unwrap_or(10);
    
    // Check if sentinel is available
    let sentinel_path = std::env::var("SENTINEL_PATH")
        .unwrap_or_else(|_| "docker".to_string());
    
    let output = if sentinel_path == "docker" {
        // Run via Docker
        let sol_shield_rpc = std::env::var("PARAPET_RPC_URL")
            .unwrap_or_else(|_| "http://host.docker.internal:8899".to_string());
        
        let llm_api_key = std::env::var("LLM_API_KEY").ok();
        let llm_base_url = std::env::var("LLM_BASE_URL").ok();
        let llm_model = std::env::var("LLM_MODEL").ok();
        
        let mut cmd = Command::new("docker");
        cmd.arg("run")
            .arg("--rm")
            .arg("-e")
            .arg(format!("PARAPET_RPC_URL={}", sol_shield_rpc))
            .arg("-e")
            .arg(format!("NAVIGATION_TIMEOUT={}", timeout))
            .arg("-e")
            .arg(format!("MAX_STEPS={}", max_steps));
        
        if let Some(key) = llm_api_key {
            cmd.arg("-e").arg(format!("LLM_API_KEY={}", key));
        }
        if let Some(base_url) = llm_base_url {
            cmd.arg("-e").arg(format!("LLM_BASE_URL={}", base_url));
        }
        if let Some(model) = llm_model {
            cmd.arg("-e").arg(format!("LLM_MODEL={}", model));
        }
        
        cmd.arg("securecheck/sentinel")
            .arg(url)
            .output()?
    } else {
        // Run via local binary
        let mut cmd = Command::new(&sentinel_path);
        cmd.arg(url)
            .arg("--timeout")
            .arg(timeout.to_string())
            .arg("--max-steps")
            .arg(max_steps.to_string())
            .output()?
    };
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Sentinel failed: {}", stderr));
    }
    
    // Parse JSON output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout)?;
    
    // Format as MCP response
    let formatted = format_phishing_report(&report)?;
    
    Ok(serde_json::json!({
        "content": [{
            "type": "text",
            "text": formatted
        }]
    }))
}

fn format_phishing_report(report: &serde_json::Value) -> Result<String> {
    let mut output = String::new();
    
    output.push_str("# Sentinel Mission Report\n\n");
    output.push_str("*Your guardian threw itself on the grenade. Here's what it found:*\n\n");
    
    if let Some(url) = report["url"].as_str() {
        output.push_str(&format!("**URL:** {}\n", url));
    }
    
    if let Some(verdict) = report["verdict"].as_str() {
        let emoji = match verdict {
            "MALICIOUS" => "🚨",
            "SUSPICIOUS" => "⚠️",
            "SAFE" => "✅",
            _ => "❓",
        };
        output.push_str(&format!("**Verdict:** {} {}\n", emoji, verdict));
    }
    
    if let Some(risk_level) = report["risk_level"].as_str() {
        output.push_str(&format!("**Risk Level:** {}\n", risk_level.to_uppercase()));
    }
    
    output.push_str("\n## Transaction Analysis\n\n");
    
    if let Some(captured) = report["transaction_captured"].as_bool() {
        if captured {
            output.push_str("✓ Transaction intercepted successfully\n\n");
            
            if let Some(programs) = report["programs_invoked"].as_array() {
                if !programs.is_empty() {
                    output.push_str("### Programs Invoked\n\n");
                    for program in programs {
                        let address = program["address"].as_str().unwrap_or("unknown");
                        let known = program["known"].as_bool().unwrap_or(false);
                        let name = program["name"].as_str();
                        
                        let status = if known { "✓" } else { "⚠️" };
                        let display_name = name.unwrap_or("UNKNOWN");
                        
                        output.push_str(&format!("- {} `{}` - {}\n", status, address, display_name));
                    }
                    output.push('\n');
                }
            }
            
            if let Some(rules) = report["rules_matched"].as_array() {
                if !rules.is_empty() {
                    output.push_str("### Rules Matched\n\n");
                    for rule in rules {
                        let id = rule["id"].as_str().unwrap_or("unknown");
                        let action = rule["action"].as_str().unwrap_or("unknown");
                        let message = rule["message"].as_str().unwrap_or("");
                        
                        output.push_str(&format!("- **[{}]** `{}`: {}\n", action.to_uppercase(), id, message));
                    }
                    output.push('\n');
                }
            }
        } else {
            output.push_str("✗ No transaction captured\n\n");
            if let Some(error) = report["error"].as_str() {
                output.push_str(&format!("Error: {}\n\n", error));
            }
        }
    }
    
    if let Some(steps) = report["navigation_steps"].as_array() {
        if !steps.is_empty() {
            output.push_str("### Navigation Steps\n\n");
            for step in steps {
                if let Some(step_str) = step.as_str() {
                    output.push_str(&format!("- {}\n", step_str));
                }
            }
            output.push('\n');
        }
    }
    
    output.push_str("\n---\n\n");
    output.push_str("**Raw Report:**\n```json\n");
    output.push_str(&serde_json::to_string_pretty(report)?);
    output.push_str("\n```\n");
    
    Ok(output)
}
