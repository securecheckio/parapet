use anyhow::{anyhow, Context, Result};
use base64::Engine;
use clap::{Parser, ValueEnum};
use colored::Colorize;
use parapet_core::rules::{AnalyzerRegistry, RuleEngine};
use parapet_scanner::{ScanConfig, ScanReport, Severity, ThreatType, WalletScanner};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, message::Message, pubkey::Pubkey,
    signature::read_keypair_file, transaction::Transaction,
};
use spl_token::instruction as token_instruction;
use std::{
    io::{self, Write},
    str::FromStr,
    sync::Arc,
};

#[derive(Parser, Debug)]
#[command(name = "wallet-scanner")]
#[command(about = "Scan Solana wallets for security threats and compromises", long_about = None)]
struct Args {
    /// Wallet address to scan
    #[arg(value_name = "WALLET_ADDRESS")]
    wallet: String,

    /// Solana RPC endpoint URL
    #[arg(short, long, default_value = "https://api.mainnet-beta.solana.com")]
    rpc_url: String,

    /// Maximum number of transactions to analyze
    #[arg(short = 't', long, default_value = "100")]
    max_transactions: usize,

    /// Time window in days to scan back
    #[arg(short = 'd', long, default_value = "30")]
    days: u32,

    /// Delay between RPC requests in milliseconds (prevent rate limiting)
    /// Delay between transactions (ms) - auto-calculated from analyzers if 0
    /// Or specify explicit delay to override analyzer recommendations
    #[arg(long, default_value = "0")]
    rpc_delay_ms: u64,

    /// Custom known-safe programs JSON file (merges with defaults)
    #[arg(long)]
    safe_programs_file: Option<String>,

    /// Output format
    #[arg(short = 'f', long, value_enum, default_value = "pretty")]
    format: OutputFormat,

    /// Network (mainnet-beta, devnet, testnet)
    #[arg(short = 'n', long, default_value = "mainnet-beta")]
    network: String,

    /// Revoke dangerous approvals after scan
    #[arg(long)]
    revoke: bool,

    /// Automatically revoke without prompting (requires --revoke)
    #[arg(long, requires = "revoke")]
    auto_revoke: bool,

    /// Minimum severity to revoke: critical, high, medium, or low
    #[arg(long, default_value = "high")]
    severity_threshold: String,

    /// Path to wallet keypair file for signing revoke transactions
    #[arg(long)]
    keypair: Option<String>,
}

#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    /// Human-readable colored output
    Pretty,
    /// JSON output
    Json,
    /// Brief summary
    Brief,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let args = Args::parse();

    println!();
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════"
            .bright_blue()
            .bold()
    );
    println!(
        "{}",
        "            Parapet Wallet Security Scanner"
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

    // Validate wallet address
    if args.wallet.len() < 32 || args.wallet.len() > 44 {
        anyhow::bail!("Invalid wallet address format");
    }

    println!("🔍 Scanning wallet: {}", args.wallet.bright_cyan());
    println!("🌐 Network: {}", args.network.bright_yellow());
    println!("📡 RPC: {}", args.rpc_url.bright_black());
    println!(
        "📊 Scope: {} transactions, {} days back",
        args.max_transactions.to_string().bright_yellow(),
        args.days.to_string().bright_yellow()
    );
    println!();

    // Initialize analyzers and rule engine (just like the proxy does)
    println!("⚙️  Initializing analyzers and rule engine...");
    let (registry, engine) = initialize_analyzers_and_rules(args.safe_programs_file.clone())?;

    // Calculate recommended delay based on active analyzers (dynamic!)
    let analyzer_delay = registry.get_recommended_delay_ms();
    let effective_delay = if args.rpc_delay_ms == 0 {
        // Use analyzer recommendation if no explicit delay set
        analyzer_delay
    } else {
        // Use the slower of user-specified or analyzer-recommended
        args.rpc_delay_ms.max(analyzer_delay)
    };

    if effective_delay > 0 {
        println!(
            "⏱️  Rate coordination: {}ms delay between transactions",
            effective_delay.to_string().bright_black()
        );
        if analyzer_delay > 0 && args.rpc_delay_ms == 0 {
            println!("    (auto-calculated from active analyzers)",);
        } else if args.rpc_delay_ms > analyzer_delay {
            println!(
                "    (user override, analyzer recommends {}ms)",
                analyzer_delay
            );
        }
    }
    println!();

    // Create scanner WITH analyzers for full capability
    let scanner = WalletScanner::with_analyzers(args.rpc_url.clone(), registry, engine)
        .context("Failed to create wallet scanner")?;

    // Configure scan - full historical analysis
    let config = ScanConfig {
        max_transactions: Some(args.max_transactions),
        time_window_days: Some(args.days),
        rpc_delay_ms: effective_delay, // Use calculated effective delay
        check_active_threats: true,
        check_historical: true,
        commitment: CommitmentConfig::confirmed(),
    };

    // Run scan (progress output will be shown by the scanner itself)
    let report = match scanner.scan(&args.wallet, config).await {
        Ok(r) => {
            eprintln!("✅ Scan complete ({}ms total)\n", r.stats.scan_duration_ms);
            r
        }
        Err(e) => {
            eprintln!("❌ Scan failed: {}", e);
            anyhow::bail!("Scan failed: {}", e);
        }
    };

    // Output results
    match args.format {
        OutputFormat::Pretty => print_pretty_report(&report),
        OutputFormat::Json => print_json_report(&report)?,
        OutputFormat::Brief => print_brief_report(&report),
    }

    // Handle revoke if requested
    if args.revoke {
        handle_revoke(&args, &report).await?;
    }

    // Exit with appropriate code
    std::process::exit(if report.security_score < 50 { 1 } else { 0 });
}

/// Initialize analyzers and rule engine (same as proxy does)
fn initialize_analyzers_and_rules(
    safe_programs_file: Option<String>,
) -> Result<(Arc<AnalyzerRegistry>, Arc<RuleEngine>)> {
    use parapet_core::rules::analyzers::*;

    // Helper to register analyzers
    fn register_all_analyzers(registry: &mut AnalyzerRegistry, safe_programs_file: Option<String>) {
        // Register built-in core analyzers
        registry.register(Arc::new(BasicAnalyzer::new()));
        registry.register(Arc::new(CoreSecurityAnalyzer::new(
            std::collections::HashSet::new(),
        )));
        registry.register(Arc::new(TokenInstructionAnalyzer::new()));
        registry.register(Arc::new(SystemProgramAnalyzer::new()));
        registry.register(Arc::new(ProgramComplexityAnalyzer::new()));

        // Register instruction padding analyzer (protection against padding attacks)
        registry.register(Arc::new(
            parapet_core::rules::analyzers::core::InstructionPaddingAnalyzer::new(),
        ));

        // Deep scanning: Inner instruction (CPI) analysis
        // Load with custom safe programs list if provided
        let inner_analyzer = if let Some(ref path) = safe_programs_file {
            match InnerInstructionAnalyzer::with_custom_list(path) {
                Ok(analyzer) => {
                    println!("✅ Loaded custom safe programs from: {}", path);
                    analyzer
                }
                Err(e) => {
                    eprintln!(
                        "⚠️  Failed to load custom safe programs from {}: {}",
                        path, e
                    );
                    eprintln!("   Using default safe programs list");
                    InnerInstructionAnalyzer::new()
                }
            }
        } else {
            InnerInstructionAnalyzer::new()
        };
        registry.register(Arc::new(inner_analyzer));

        // Register third-party analyzers (always enabled for comprehensive analysis)
        // Helius Identity: Provides program names, categories, and reputation
        registry.register(Arc::new(HeliusIdentityAnalyzer::new()));

        // Helius Transfer: Detects velocity and counterparty patterns (active drains)
        registry.register(Arc::new(HeliusTransferAnalyzer::new()));

        // Helius Funding: Detects sybil attacks and bot farms
        registry.register(Arc::new(HeliusFundingAnalyzer::new()));

        // OtterSec Verification: Checks if programs are cryptographically verified
        registry.register(Arc::new(OtterSecVerifiedAnalyzer::new()));

        // Jupiter Token: Provides token metadata and verification
        registry.register(Arc::new(JupiterTokenAnalyzer::new()));

        // Rugcheck: Scam/rugpull detection (FREE - no API key required)
        registry.register(Arc::new(RugcheckAnalyzer::new()));
    }

    // Create registry for rule engine
    let mut engine_registry = AnalyzerRegistry::new();
    register_all_analyzers(&mut engine_registry, safe_programs_file.clone());

    // Create rule engine (consumes the registry)
    let mut engine = RuleEngine::new(engine_registry);

    // Load rules from default location or environment
    let rules_path = std::env::var("RULES_PATH").ok().or_else(|| {
        // Try enhanced ruleset first (uses Helius/OtterSec to reduce false positives)
        let enhanced_candidates = vec![
            "../../proxy/rules/presets/wallet-scan-enhanced.json",
            "../proxy/rules/presets/wallet-scan-enhanced.json",
            "./rules/presets/wallet-scan-enhanced.json",
            "rules/presets/wallet-scan-enhanced.json",
        ];

        // Fallback to bot-essentials if enhanced not found
        let fallback_candidates = vec![
            "../../proxy/rules/presets/bot-essentials.json",
            "../proxy/rules/presets/bot-essentials.json",
            "./rules/presets/bot-essentials.json",
            "rules/presets/bot-essentials.json",
        ];

        enhanced_candidates
            .iter()
            .chain(fallback_candidates.iter())
            .find(|p| std::path::Path::new(p).exists())
            .map(|s| s.to_string())
    });

    if let Some(path) = rules_path {
        engine.load_rules_from_file(&path)?;
        log::info!("📋 Loaded rules from: {}", path);
    } else {
        log::warn!("⚠️  No rules file found, using minimal built-in protection");
    }

    // Create separate registry for scanner (needs it to call analyzers directly)
    let mut scanner_registry = AnalyzerRegistry::new();
    register_all_analyzers(&mut scanner_registry, safe_programs_file);

    Ok((Arc::new(scanner_registry), Arc::new(engine)))
}

fn print_pretty_report(report: &ScanReport) {
    // Security Score Header
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black()
    );
    println!("{}", "  SECURITY ASSESSMENT".bright_white().bold());
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black()
    );
    println!();

    // Security Score with color coding
    let (score_color, risk_color, icon) = match report.security_score {
        0..=30 => ("red", "red", "🚨"),
        31..=50 => ("bright red", "red", "⚠️ "),
        51..=75 => ("yellow", "yellow", "⚠️ "),
        76..=90 => ("bright green", "yellow", "✓ "),
        _ => ("bright green", "green", "✅"),
    };

    println!(
        "  {} Security Score: {} / 100",
        icon,
        format!("{}", report.security_score)
            .color(score_color)
            .bold()
    );
    println!(
        "  Risk Level: {}",
        report.risk_level.color(risk_color).bold()
    );
    println!();

    // Statistics
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black()
    );
    println!("{}", "  SCAN STATISTICS".bright_white().bold());
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black()
    );
    println!();

    println!("  📅 Time Range: {} days", report.stats.time_range_days);
    if report.stats.transactions_analyzed > 0 {
        println!(
            "  📝 Transactions Analyzed: {}",
            report.stats.transactions_analyzed
        );
    } else {
        println!(
            "  📝 Transactions Analyzed: {} (no recent transactions found)",
            report.stats.transactions_analyzed
        );
    }

    println!("  ⚠️  Total Threats Found: {}", report.stats.threats_found);
    println!();

    if report.stats.threats_found > 0 {
        println!("  Threat Breakdown:");
        if report.stats.critical_count > 0 {
            println!(
                "    {} Critical",
                format!("{:>3}", report.stats.critical_count)
                    .bright_red()
                    .bold()
            );
        }
        if report.stats.high_count > 0 {
            println!(
                "    {} High",
                format!("{:>3}", report.stats.high_count).red()
            );
        }
        if report.stats.medium_count > 0 {
            println!(
                "    {} Medium",
                format!("{:>3}", report.stats.medium_count).yellow()
            );
        }
        if report.stats.low_count > 0 {
            println!(
                "    {} Low",
                format!("{:>3}", report.stats.low_count).bright_black()
            );
        }
        println!();
    }

    // Threats Detail
    if !report.threats.is_empty() {
        println!(
            "{}",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black()
        );
        println!("{}", "  DETECTED THREATS".bright_white().bold());
        println!(
            "{}",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black()
        );
        println!();

        for (idx, threat) in report.threats.iter().enumerate() {
            let (severity_icon, severity_color) = match threat.severity {
                Severity::Critical => ("🚨", "bright red"),
                Severity::High => ("⚠️ ", "red"),
                Severity::Medium => ("⚠️ ", "yellow"),
                Severity::Low => ("ℹ️ ", "bright black"),
            };

            println!(
                "  {} {} {}",
                format!("[{}]", idx + 1).bright_black(),
                severity_icon,
                format!("{:?}", threat.severity)
                    .color(severity_color)
                    .bold()
            );

            // Extract details based on threat type
            match &threat.threat_type {
                ThreatType::ActiveUnlimitedDelegation {
                    token_account,
                    delegate,
                    ..
                } => {
                    println!("     Type: Active Unlimited Delegation");
                    println!("     Token: {}", token_account.bright_black());
                    println!("     Delegate: {}", delegate.bright_black());
                }
                ThreatType::PossibleExploitedDelegation {
                    token_account,
                    delegate,
                    ..
                } => {
                    println!("     Type: Possibly Exploited Delegation");
                    println!("     Token: {}", token_account.bright_black());
                    println!("     Delegate: {}", delegate.bright_black());
                }
                ThreatType::CompromisedAuthority {
                    account,
                    expected_owner,
                    actual_owner,
                } => {
                    println!("     Type: Compromised Authority");
                    println!("     Account: {}", account.bright_black());
                    println!(
                        "     Expected: {} → Actual: {}",
                        expected_owner.bright_black(),
                        actual_owner.bright_red()
                    );
                }
                ThreatType::SuspiciousTransaction {
                    signature,
                    threat_description,
                    risk_score,
                    ..
                } => {
                    println!("     Type: Suspicious Transaction");
                    println!("     Description: {}", threat_description);
                    println!("     Risk Score: {}/100", risk_score);
                    println!("     Transaction: {}", signature.bright_black());
                }
                ThreatType::UnusualPattern {
                    pattern_description,
                    occurrences,
                    ..
                } => {
                    println!("     Type: Unusual Pattern");
                    println!("     Description: {}", pattern_description);
                    println!("     Occurrences: {}", occurrences);
                }
            }

            println!("     📌 Action: {}", threat.recommendation.bright_yellow());
            println!();
        }
    }

    // Suspicious Programs
    if !report.suspicious_programs.is_empty() {
        println!(
            "{}",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black()
        );
        println!("{}", "  SUSPICIOUS PROGRAMS".bright_white().bold());
        println!(
            "{}",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black()
        );
        println!();

        for program in &report.suspicious_programs {
            let risk_color = if program.risk_score > 70 {
                "red"
            } else if program.risk_score > 40 {
                "yellow"
            } else {
                "bright black"
            };

            println!("  Program: {}", program.program_id.bright_cyan());
            println!(
                "    Risk Score: {}/100",
                format!("{}", program.risk_score).color(risk_color).bold()
            );
            println!("    Type: {}", program.threat_type);
            println!("    Occurrences: {}", program.occurrence_count);

            // Show transaction signatures (up to 3, then indicate if there are more)
            if !program.transaction_signatures.is_empty() {
                print!("    Transactions: ");
                let max_to_show = 3;
                let to_display = program.transaction_signatures.iter().take(max_to_show);
                for (i, sig) in to_display.enumerate() {
                    if i > 0 {
                        print!(", ");
                    }
                    print!("{}", sig.bright_black());
                }
                if program.transaction_signatures.len() > max_to_show {
                    print!(
                        " {} {} more",
                        "+".bright_yellow(),
                        (program.transaction_signatures.len() - max_to_show)
                            .to_string()
                            .bright_yellow()
                    );
                }
                println!();
            }

            println!("    Summary: {}", program.analysis_summary);
            println!(
                "    📌 Recommendation: {}",
                program.recommendation.bright_yellow()
            );
            println!();
        }
    }

    // Final recommendation
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black()
    );
    println!("{}", "  RECOMMENDATION".bright_white().bold());
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black()
    );
    println!();

    match report.security_score {
        0..=30 => {
            println!(
                "  {} {}",
                "🚨".bright_red(),
                "CRITICAL: This wallet shows signs of compromise!"
                    .bright_red()
                    .bold()
            );
            println!();
            println!("  Immediate Actions:");
            println!("    1. Stop using this wallet immediately");
            println!("    2. Create a new wallet with a new seed phrase");
            println!("    3. Transfer remaining funds to the new wallet");
            println!("    4. Revoke all token delegations");
            println!("    5. Review how the compromise occurred");
        }
        31..=50 => {
            println!(
                "  {} {}",
                "⚠️ ".bright_red(),
                "HIGH RISK: Multiple security concerns detected"
                    .red()
                    .bold()
            );
            println!();
            println!("  Recommended Actions:");
            println!("    1. Review all detected threats carefully");
            println!("    2. Revoke suspicious token delegations");
            println!("    3. Consider moving funds to a new wallet");
            println!("    4. Enable additional security measures");
        }
        51..=75 => {
            println!(
                "  {} {}",
                "⚠️ ".yellow(),
                "MODERATE RISK: Some security concerns found"
                    .yellow()
                    .bold()
            );
            println!();
            println!("  Recommended Actions:");
            println!("    1. Review the detected issues");
            println!("    2. Revoke unnecessary delegations");
            println!("    3. Be cautious with future transactions");
        }
        76..=90 => {
            println!(
                "  {} {}",
                "✓".bright_green(),
                "LOW RISK: Minor concerns detected".green()
            );
            println!();
            println!("  Suggested Actions:");
            println!("    1. Review low-priority items");
            println!("    2. Continue monitoring wallet activity");
        }
        _ => {
            println!(
                "  {} {}",
                "✅".bright_green(),
                "SAFE: No security threats detected".bright_green().bold()
            );
            println!();
            println!("  Your wallet appears secure. Continue best practices:");
            println!("    • Only connect to trusted dApps");
            println!("    • Review transactions before signing");
            println!("    • Monitor for unexpected activity");
        }
    }

    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black()
    );
    println!();
}

fn print_json_report(report: &ScanReport) -> Result<()> {
    let json = serde_json::to_string_pretty(report)?;
    println!("{}", json);
    Ok(())
}

fn print_brief_report(report: &ScanReport) {
    let status_icon = if report.security_score >= 75 {
        "✅"
    } else if report.security_score >= 50 {
        "⚠️ "
    } else {
        "🚨"
    };

    println!(
        "{} {} - Security Score: {}/100 - Risk: {}",
        status_icon, report.wallet, report.security_score, report.risk_level
    );

    if report.stats.threats_found > 0 {
        println!(
            "   Threats: {} critical, {} high, {} medium, {} low",
            report.stats.critical_count,
            report.stats.high_count,
            report.stats.medium_count,
            report.stats.low_count
        );
    }

    if !report.suspicious_programs.is_empty() {
        println!(
            "   Suspicious programs: {}",
            report.suspicious_programs.len()
        );
    }
}

/// Handle revoke functionality after scan
async fn handle_revoke(args: &Args, report: &ScanReport) -> Result<()> {
    // Parse severity threshold
    let min_severity = match args.severity_threshold.to_lowercase().as_str() {
        "critical" => Severity::Critical,
        "high" => Severity::High,
        "medium" => Severity::Medium,
        "low" => Severity::Low,
        _ => {
            eprintln!(
                "⚠️  Invalid severity threshold '{}', using 'high'",
                args.severity_threshold
            );
            Severity::High
        }
    };

    // Filter threats by severity and type (only approvals/delegations)
    let mut dangerous_approvals = Vec::new();
    for threat in &report.threats {
        // Check severity
        let severity_met = match (&threat.severity, min_severity) {
            (Severity::Critical, _) => true,
            (Severity::High, Severity::High | Severity::Medium | Severity::Low) => true,
            (Severity::Medium, Severity::Medium | Severity::Low) => true,
            (Severity::Low, Severity::Low) => true,
            _ => false,
        };

        if !severity_met {
            continue;
        }

        // Extract approval threats
        match &threat.threat_type {
            ThreatType::ActiveUnlimitedDelegation {
                token_account,
                delegate,
                amount,
                ..
            } => {
                dangerous_approvals.push((
                    token_account.clone(),
                    delegate.clone(),
                    *amount,
                    threat.severity.clone(),
                ));
            }
            ThreatType::PossibleExploitedDelegation {
                token_account,
                delegate,
                amount,
                ..
            } => {
                dangerous_approvals.push((
                    token_account.clone(),
                    delegate.clone(),
                    *amount,
                    threat.severity.clone(),
                ));
            }
            _ => {}
        }
    }

    if dangerous_approvals.is_empty() {
        println!();
        println!(
            "{}",
            "═══════════════════════════════════════════════════════════".bright_green()
        );
        println!(
            "  ✅ No dangerous approvals found meeting {} severity threshold",
            args.severity_threshold
        );
        println!(
            "{}",
            "═══════════════════════════════════════════════════════════".bright_green()
        );
        println!();
        return Ok(());
    }

    // Show dangerous approvals
    println!();
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════"
            .bright_red()
            .bold()
    );
    println!("  {} DANGEROUS APPROVALS DETECTED", "🚨".bright_red());
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════"
            .bright_red()
            .bold()
    );
    println!();
    println!(
        "Found {} dangerous approvals meeting '{}' severity threshold:",
        dangerous_approvals.len().to_string().bright_red().bold(),
        args.severity_threshold.bright_yellow()
    );
    println!();

    for (i, (token_account, delegate, amount, severity)) in dangerous_approvals.iter().enumerate() {
        let severity_str = match severity {
            Severity::Critical => "CRITICAL".bright_red().bold(),
            Severity::High => "HIGH".red().bold(),
            Severity::Medium => "MEDIUM".yellow().bold(),
            Severity::Low => "LOW".normal(),
        };

        let amount_str = if *amount == u64::MAX {
            "UNLIMITED".bright_red().bold().to_string()
        } else {
            amount.to_string()
        };

        println!(
            "  {}. [{}] Token Account: {}",
            (i + 1).to_string().bright_white().bold(),
            severity_str,
            token_account.bright_cyan()
        );
        println!("     Delegate: {}", delegate.bright_black());
        println!("     Amount: {}", amount_str);
        println!();
    }

    // Ask for confirmation unless auto-revoke
    if !args.auto_revoke {
        print!(
            "{} ",
            "Do you want to revoke these approvals? [y/N]:"
                .bright_yellow()
                .bold()
        );
        io::stdout().flush()?;

        let mut response = String::new();
        io::stdin().read_line(&mut response)?;

        if !response.trim().eq_ignore_ascii_case("y") {
            println!("Revoke cancelled.");
            return Ok(());
        }
    }

    println!();
    println!("{}", "Generating revoke transactions...".bright_blue());
    println!();

    // Extract token accounts
    let token_accounts: Vec<String> = dangerous_approvals
        .iter()
        .map(|(account, _, _, _)| account.clone())
        .collect();

    // Build revoke transactions
    let rpc_client = RpcClient::new(args.rpc_url.clone());
    let revoke_txs =
        build_batch_revoke_transactions(&args.wallet, &token_accounts, &rpc_client).await?;

    println!(
        "Generated {} transaction(s) to revoke {} approvals",
        revoke_txs.len().to_string().bright_green(),
        dangerous_approvals.len().to_string().bright_green()
    );
    println!();

    // If keypair provided, sign and submit
    if let Some(keypair_path) = &args.keypair {
        sign_and_submit_transactions(&revoke_txs, keypair_path, &rpc_client).await?;
    } else {
        // No keypair, show instructions for manual signing
        show_manual_signing_instructions(&revoke_txs);
    }

    Ok(())
}

/// Build batch revoke transactions (up to 10 per tx)
async fn build_batch_revoke_transactions(
    wallet: &str,
    token_accounts: &[String],
    rpc_client: &RpcClient,
) -> Result<Vec<Transaction>> {
    let owner = Pubkey::from_str(wallet).map_err(|e| anyhow!("Invalid wallet address: {}", e))?;

    let mut transactions = Vec::new();
    const MAX_REVOKES_PER_TX: usize = 10;

    for chunk in token_accounts.chunks(MAX_REVOKES_PER_TX) {
        let mut instructions = Vec::new();

        for token_account_str in chunk {
            let token_account_pubkey = Pubkey::from_str(token_account_str).map_err(|e| {
                anyhow!("Invalid token account address {}: {}", token_account_str, e)
            })?;

            let revoke_ix =
                token_instruction::revoke(&spl_token::id(), &token_account_pubkey, &owner, &[])?;

            instructions.push(revoke_ix);
        }

        let recent_blockhash = rpc_client
            .get_latest_blockhash()
            .map_err(|e| anyhow!("Failed to get recent blockhash: {}", e))?;

        let message = Message::new(&instructions, Some(&owner));
        let mut tx = Transaction::new_unsigned(message);
        tx.message.recent_blockhash = recent_blockhash;

        transactions.push(tx);
    }

    Ok(transactions)
}

/// Sign and submit transactions
async fn sign_and_submit_transactions(
    transactions: &[Transaction],
    keypair_path: &str,
    rpc_client: &RpcClient,
) -> Result<()> {
    println!("{}", "Signing and submitting transactions...".bright_blue());
    println!();

    let keypair = read_keypair_file(keypair_path)
        .map_err(|e| anyhow!("Failed to read keypair from {}: {}", keypair_path, e))?;

    for (i, tx) in transactions.iter().enumerate() {
        print!("  Transaction {}/{}: ", i + 1, transactions.len());
        io::stdout().flush()?;

        // Sign transaction
        let mut signed_tx = tx.clone();
        signed_tx.sign(&[&keypair], signed_tx.message.recent_blockhash);

        // Submit transaction
        match rpc_client.send_and_confirm_transaction(&signed_tx) {
            Ok(signature) => {
                println!(
                    "{} {}",
                    "✅".bright_green(),
                    signature.to_string().bright_black()
                );
            }
            Err(e) => {
                println!("{} {}", "❌".bright_red(), format!("Failed: {}", e).red());
                eprintln!("Error details: {}", e);
            }
        }
    }

    println!();
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════".bright_green()
    );
    println!(
        "  {} All approvals revoked successfully!",
        "✅".bright_green()
    );
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════".bright_green()
    );
    println!();

    Ok(())
}

/// Show instructions for manual signing (no keypair provided)
fn show_manual_signing_instructions(transactions: &[Transaction]) {
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════".bright_yellow()
    );
    println!("  {} Manual Signing Required", "📋".bright_yellow());
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════".bright_yellow()
    );
    println!();
    println!("No keypair provided. To sign these transactions:");
    println!();
    println!("Option 1: Provide keypair file (recommended for automation):");
    println!(
        "  {} --keypair ~/.config/solana/id.json",
        "wallet-scanner scan <WALLET> --revoke".bright_cyan()
    );
    println!();
    println!("Option 2: Use Solana CLI to sign:");
    println!();

    for (i, tx) in transactions.iter().enumerate() {
        let serialized = bincode::serialize(&tx).expect("Failed to serialize transaction");
        let base64 = base64::engine::general_purpose::STANDARD.encode(&serialized);

        println!("  Transaction {}/{}:", i + 1, transactions.len());
        println!("  {}", "solana sign-transaction <(echo '{}')".bright_cyan());
        println!("    (replace '{{}}' with: {})", base64.bright_black());
        println!();
    }

    println!("Option 3: Copy base64 and paste into wallet UI:");
    println!();

    for (i, tx) in transactions.iter().enumerate() {
        let serialized = bincode::serialize(&tx).expect("Failed to serialize transaction");
        let base64 = base64::engine::general_purpose::STANDARD.encode(&serialized);

        println!("  Transaction {}/{}:", i + 1, transactions.len());
        println!("  {}", base64.bright_black());
        println!();
    }

    println!(
        "{}",
        "═══════════════════════════════════════════════════════════".bright_yellow()
    );
    println!();
}
