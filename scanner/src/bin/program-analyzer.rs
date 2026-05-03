use anyhow::{Context, Result};
use clap::Parser;
use owo_colors::OwoColorize;
use solana_client::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[derive(Parser)]
#[command(name = "program-analyzer")]
#[command(about = "Analyze Solana programs for security and verification", long_about = None)]
struct Args {
    /// Program address to analyze
    #[arg(value_name = "PROGRAM_ID")]
    program_id: String,

    /// RPC URL
    #[arg(short, long)]
    rpc_url: Option<String>,

    /// Network (mainnet-beta, devnet, testnet)
    #[arg(short, long, default_value = "mainnet-beta")]
    network: String,

    /// Analysis tier: superficial (fast), deep (medium), ai (slow, requires API key)
    #[arg(short = 't', long, default_value = "deep")]
    tier: String,

    /// Enable deep bytecode analysis
    #[arg(long)]
    deep: bool,

    /// Enable AI-powered analysis (requires AI_PROVIDER env var and API key)
    #[arg(long)]
    ai: bool,

    /// Output format (text, json)
    #[arg(short = 'f', long, default_value = "text")]
    format: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    // Get RPC URL from args, environment, or derive from network flag
    let rpc_url = if let Some(url) = &args.rpc_url {
        url.clone()
    } else if let Ok(url) = std::env::var("SOLANA_RPC_URL") {
        url
    } else {
        match args.network.as_str() {
            "devnet" => "https://api.devnet.solana.com".to_string(),
            "testnet" => "https://api.testnet.solana.com".to_string(),
            _ => "https://api.mainnet-beta.solana.com".to_string(),
        }
    };

    println!("\n{}", "═".repeat(60).bright_black());
    println!(
        "{}",
        "        Parapet Program Security Analyzer"
            .bright_cyan()
            .bold()
    );
    println!("{}\n", "═".repeat(60).bright_black());

    // Validate program ID
    let program_pubkey = Pubkey::from_str(&args.program_id).context("Invalid program ID format")?;

    println!(
        "🔍 Analyzing program: {}",
        args.program_id.bright_white().bold()
    );
    println!("🌐 Network: {}", args.network.bright_yellow());
    println!("📡 RPC: {}\n", rpc_url.bright_black());

    // Initialize RPC client
    let rpc_client = RpcClient::new_with_commitment(rpc_url.clone(), CommitmentConfig::confirmed());

    // 1. Check if program exists and is executable
    println!("{}", "⚙️  Checking on-chain program data...".bright_cyan());
    match rpc_client.get_account(&program_pubkey) {
        Ok(account) => {
            println!("  ✅ Program exists on-chain");
            println!("     Owner: {}", account.owner.to_string().bright_white());
            println!(
                "     Executable: {}",
                if account.executable {
                    "Yes ✓".green().to_string()
                } else {
                    "No ✗".red().to_string()
                }
            );
            println!(
                "     Data size: {} bytes",
                account.data.len().to_string().bright_white()
            );
            println!(
                "     Lamports: {}",
                account.lamports.to_string().bright_white()
            );
        }
        Err(e) => {
            println!(
                "  ⚠️  Could not fetch program account: {}",
                e.to_string().yellow()
            );
        }
    }

    // 2. OtterSec Verification Check
    println!("\n{}", "🔐 OtterSec Verification Check...".bright_cyan());
    if let Err(e) = check_ottersec_verification(&args.program_id).await {
        println!(
            "  ⚠️  {}",
            format!("Could not check verification: {}", e).yellow()
        );
    }

    // 3. Helius Identity Check (if API key available)
    if std::env::var("HELIUS_API_KEY").is_ok() {
        println!("\n{}", "🏷️  Helius Identity Check...".bright_cyan());
        if let Err(e) = check_helius_identity(&args.program_id).await {
            println!(
                "  ⚠️  {}",
                format!("Could not check identity: {}", e).yellow()
            );
        }
    } else {
        println!(
            "\n{}",
            "💡 Set HELIUS_API_KEY for identity checks".bright_black()
        );
    }

    // 4. Common Explorer Links
    println!("\n{}", "🔗 Explorer Links:".bright_cyan());
    println!(
        "  Solscan: {}",
        format!("https://solscan.io/account/{}", args.program_id)
            .bright_blue()
            .underline()
    );
    println!(
        "  Solana Explorer: {}",
        format!("https://explorer.solana.com/address/{}", args.program_id)
            .bright_blue()
            .underline()
    );
    println!(
        "  SolanaFM: {}",
        format!("https://solana.fm/address/{}", args.program_id)
            .bright_blue()
            .underline()
    );

    // 5. Advanced Program Analysis (if enabled with --deep or --ai flags)
    #[cfg(feature = "program-analysis")]
    if args.deep || args.ai {
        println!("\n{}", "🧠 Advanced Program Analysis...".bright_cyan());
        match run_program_analysis(&args, &rpc_url).await {
            Ok(result) => {
                if args.format == "json" {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    print_analysis_result(&result);
                }
            }
            Err(e) => {
                println!("  ⚠️  {}", format!("Analysis failed: {}", e).yellow());
            }
        }
    } else {
        println!(
            "\n💡 Use {} or {} for advanced analysis",
            "--deep".bright_yellow(),
            "--ai".bright_yellow()
        );
    }

    println!("\n{}\n", "═".repeat(60).bright_black());

    Ok(())
}

#[cfg(feature = "program-analysis")]
async fn run_program_analysis(
    args: &Args,
    rpc_url: &str,
) -> Result<parapet_core::program_analysis::ProgramAnalysisResult> {
    use parapet_core::program_analysis::{AnalysisMode, AnalysisTier, ProgramAnalysisService};

    let tier = if args.ai {
        AnalysisTier::AI
    } else if args.deep {
        AnalysisTier::Deep
    } else {
        // Parse from --tier flag
        match args.tier.to_lowercase().as_str() {
            "superficial" => AnalysisTier::Superficial,
            "deep" => AnalysisTier::Deep,
            "ai" => AnalysisTier::AI,
            _ => AnalysisTier::Deep,
        }
    };

    println!("  Analysis tier: {:?}", tier);

    #[cfg(feature = "ai-analysis")]
    let service = if tier == AnalysisTier::AI {
        use parapet_core::program_analysis::AiProviderConfig;
        ProgramAnalysisService::new_with_ai(rpc_url.to_string(), AiProviderConfig::default())
    } else {
        ProgramAnalysisService::new(rpc_url.to_string())
    };

    #[cfg(not(feature = "ai-analysis"))]
    let service = ProgramAnalysisService::new(rpc_url.to_string());

    service
        .analyze_program(&args.program_id, tier, AnalysisMode::Synchronous)
        .await
        .map_err(anyhow::Error::from)
}

#[cfg(feature = "program-analysis")]
fn print_analysis_result(result: &parapet_core::program_analysis::ProgramAnalysisResult) {
    use owo_colors::OwoColorize;
    use parapet_core::program_analysis::RiskLevel;

    println!("\n  📊 Analysis Summary:");
    println!("     Tier: {}", result.tier_used.bright_white());
    println!(
        "     Risk Score: {} / 100",
        match result.risk_level {
            RiskLevel::VeryLow => format!("{:.1}", result.risk_score).green().to_string(),
            RiskLevel::Low => format!("{:.1}", result.risk_score)
                .bright_green()
                .to_string(),
            RiskLevel::Medium => format!("{:.1}", result.risk_score).yellow().to_string(),
            RiskLevel::High => format!("{:.1}", result.risk_score).bright_red().to_string(),
            RiskLevel::Critical => format!("{:.1}", result.risk_score).red().bold().to_string(),
        }
    );
    println!(
        "     Risk Level: {}",
        match result.risk_level {
            RiskLevel::VeryLow => "Very Low ✓".green().to_string(),
            RiskLevel::Low => "Low ✓".bright_green().to_string(),
            RiskLevel::Medium => "Medium ⚠".yellow().to_string(),
            RiskLevel::High => "High ⚠⚠".bright_red().to_string(),
            RiskLevel::Critical => "CRITICAL ⚠⚠⚠".red().bold().to_string(),
        }
    );
    println!(
        "     Safe: {}",
        if result.is_safe {
            "Yes ✓".green().to_string()
        } else {
            "No ✗".red().to_string()
        }
    );
    println!("     Analysis Time: {}ms", result.analysis_time_ms);

    if let Some(ref bytecode) = result.bytecode_analysis {
        println!("\n  🔬 Bytecode Analysis:");
        println!(
            "     Instructions: {}",
            bytecode.total_instructions.to_string().bright_white()
        );
        println!(
            "     Suspicious: {}",
            bytecode.suspicious_instruction_count.to_string().yellow()
        );
        println!("     Complexity: {:.2}", bytecode.complexity_score);
        println!("     Entropy: {:.2}", bytecode.entropy_score);
    }

    if !result.suspicious_patterns.is_empty() {
        println!("\n  ⚠️  Suspicious Patterns:");
        for pattern in &result.suspicious_patterns {
            println!("     • {}", pattern.yellow());
        }
    }

    if let Some(ref ai) = result.ai_analysis {
        println!("\n  🤖 AI Analysis:");
        println!("     Model: {}", ai.model_used.bright_white());
        println!("     Confidence: {:.0}%", ai.confidence_score * 100.0);
        println!("     {}", ai.behavioral_analysis.bright_black());
    }

    if !result.vulnerabilities.is_empty() {
        println!("\n  🚨 Vulnerabilities:");
        for vuln in &result.vulnerabilities {
            let severity_color = match vuln.severity.as_str() {
                "Critical" => "red",
                "High" => "bright_red",
                "Medium" => "yellow",
                _ => "white",
            };
            let severity_display = match severity_color {
                "red" => vuln.severity.as_str().red().to_string(),
                "bright_red" => vuln.severity.as_str().bright_red().to_string(),
                "yellow" => vuln.severity.as_str().yellow().to_string(),
                "white" => vuln.severity.as_str().white().to_string(),
                _ => vuln.severity.to_string(),
            };
            println!(
                "     • [{}] {}: {}",
                severity_display,
                vuln.category.bright_white(),
                vuln.description
            );
        }
    }

    if !result.recommendations.is_empty() {
        println!("\n  💡 Recommendations:");
        for rec in &result.recommendations {
            println!("     • {}", rec.bright_cyan());
        }
    }
}

async fn check_ottersec_verification(program_id: &str) -> Result<()> {
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

                if is_verified {
                    println!("  ✅ {}", "Program is VERIFIED".green().bold());
                    if let Some(repo_url) = body["repo_url"].as_str() {
                        println!("     Source: {}", repo_url.bright_blue().underline());
                    }
                    if let Some(verified_at) = body["last_verified_at"].as_str() {
                        println!("     Last verified: {}", verified_at.bright_white());
                    }
                } else {
                    println!("  ⚠️  {}", "Program is NOT verified".yellow().bold());
                    println!("     {}", message.bright_black());
                }
            } else if response.status().as_u16() == 404 {
                println!(
                    "  ⚠️  {}",
                    "Program not found in OtterSec database".yellow()
                );
                println!("     This program has not been submitted for verification");
            } else {
                println!("  ⚠️  API error: {}", response.status());
            }
        }
        Err(e) => {
            println!("  ⚠️  Could not reach OtterSec API: {}", e);
        }
    }

    Ok(())
}

async fn check_helius_identity(program_id: &str) -> Result<()> {
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

    if response.status().is_success() {
        let body: serde_json::Value = response.json().await?;
        if let Some(identity) = body.as_array().and_then(|arr| arr.first()) {
            if let Some(name) = identity["name"].as_str() {
                println!("  ✅ Identified: {}", name.green().bold());
            }
            if let Some(category) = identity["category"].as_str() {
                println!("     Category: {}", category.bright_white());
            }
            if let Some(identity_type) = identity["type"].as_str() {
                println!("     Type: {}", identity_type.bright_white());
            }
            if let Some(tags) = identity["tags"].as_array() {
                let tag_strings: Vec<String> = tags
                    .iter()
                    .filter_map(|t| t.as_str())
                    .map(|s| s.to_string())
                    .collect();
                if !tag_strings.is_empty() {
                    println!("     Tags: {}", tag_strings.join(", ").bright_yellow());
                }
            }

            // If no identity data found
            if identity["name"].is_null() && identity["category"].is_null() {
                println!("  ℹ️  No identity information available for this program");
            }
        }
    } else {
        println!("  ⚠️  Helius API error: {}", response.status());
    }

    Ok(())
}
