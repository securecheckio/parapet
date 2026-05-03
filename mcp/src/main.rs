#!/usr/bin/env cargo
//! Parapet MCP Server
//!
//! Provides wallet scanning and program analysis via Model Context Protocol
//!
//! Environment Variables:
//!   - SOLANA_RPC_URL: single RPC endpoint
//!   - SOLANA_RPC_URLS: comma-separated list (first URL used for blocking RpcClient; use parapet-rpc-proxy for failover)
//!   - Default: https://api.mainnet-beta.solana.com
//!   - HELIUS_API_KEY: Optional, for enhanced identity checks
//!   - RULES_PATH: Optional, custom rules file path
//!
//! Usage:
//!   Add to MCP client config (e.g., Cursor, Claude Desktop):
//!   {
//!     "mcpServers": {
//!       "parapet": {
//!         "command": "parapet-mcp",
//!         "env": {
//!           "SOLANA_RPC_URL": "https://api.mainnet-beta.solana.com",
//!           "HELIUS_API_KEY": "your-key-here"
//!         }
//!       }
//!     }
//!   }

use anyhow::Result;
use parapet_scanner::{ScanConfig, WalletScanner};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_commitment_config::CommitmentConfig;
use std::io::{self, BufRead, Write};

fn default_solana_rpc_primary() -> String {
    let raw = std::env::var("SOLANA_RPC_URLS")
        .or_else(|_| std::env::var("SOLANA_RPC_URL"))
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    let urls = parapet_upstream::parse_upstream_urls_list(&raw);
    if urls.len() > 1 {
        log::info!(
            "Multiple SOLANA_RPC_URL* endpoints ({}); using first for blocking RpcClient. Prefer a parapet-rpc-proxy URL for failover.",
            urls.len()
        );
    }
    urls.into_iter()
        .next()
        .unwrap_or_else(|| "https://api.mainnet-beta.solana.com".to_string())
}

mod helius_tools;
mod rugcheck_tools;

// Import shared tools from the library
use parapet_mcp_server::tools;

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Value,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to stderr (stdout is for JSON-RPC)
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(env_logger::Target::Stderr)
        .init();

    log::info!("Starting Parapet MCP Server");

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                log::error!("Failed to parse request: {}", e);
                continue;
            }
        };

        log::debug!("Received request: method={}", request.method);

        let response = handle_request(request).await;
        let response_json = serde_json::to_string(&response)?;

        writeln!(stdout, "{}", response_json)?;
        stdout.flush()?;
    }

    Ok(())
}

fn handle_resource_read(uri: &str) -> Result<Value> {
    let content = match uri {
        "parapet://guide" => include_str!("../resources/guide.md"),
        "parapet://risk-scoring" => include_str!("../resources/risk-scoring.md"),
        "parapet://examples" => include_str!("../resources/examples.md"),
        _ => return Err(anyhow::anyhow!("Unknown resource URI: {}", uri)),
    };

    Ok(json!({
        "contents": [{
            "uri": uri,
            "mimeType": "text/markdown",
            "text": content
        }]
    }))
}

async fn handle_request(request: JsonRpcRequest) -> JsonRpcResponse {
    let result = match request.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "serverInfo": {
                "name": "parapet",
                "version": env!("CARGO_PKG_VERSION")
            },
            "capabilities": {
                "tools": {},
                "resources": {}
            }
        })),
        "resources/list" => Ok(json!({
            "resources": [
                {
                    "uri": "parapet://guide",
                    "name": "Parapet Guide",
                    "description": "Complete guide to using Parapet for wallet and program security analysis",
                    "mimeType": "text/markdown"
                },
                {
                    "uri": "parapet://risk-scoring",
                    "name": "Risk Scoring System",
                    "description": "Explanation of how Parapet calculates risk scores and threat levels",
                    "mimeType": "text/markdown"
                },
                {
                    "uri": "parapet://examples",
                    "name": "Usage Examples",
                    "description": "Example workflows for scanning wallets and analyzing programs",
                    "mimeType": "text/markdown"
                }
            ]
        })),
        "resources/read" => {
            let uri = request
                .params
                .as_ref()
                .and_then(|p| p.get("uri"))
                .and_then(|u| u.as_str())
                .unwrap_or("");

            handle_resource_read(uri)
        }
        "tools/list" => Ok(json!({
            "tools": [
                {
                    "name": "scan_wallet",
                    "description": "Scan a Solana wallet for security threats, compromised accounts, and suspicious activity",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "wallet_address": {
                                "type": "string",
                                "description": "The Solana wallet address to scan"
                            },
                            "rpc_url": {
                                "type": "string",
                                "description": "Solana RPC URL (optional, defaults to env or mainnet)"
                            },
                            "max_transactions": {
                                "type": "number",
                                "description": "Maximum transactions to analyze (default: 100)"
                            },
                            "time_window_days": {
                                "type": "number",
                                "description": "Days to scan back (default: 30)"
                            },
                            "format": {
                                "type": "string",
                                "description": "Output format: summary, detailed, or json (default: summary)",
                                "enum": ["summary", "detailed", "json"]
                            }
                        },
                        "required": ["wallet_address"]
                    }
                },
                {
                    "name": "analyze_program",
                    "description": "Analyze a Solana program for security, verification status, and identity information",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "program_id": {
                                "type": "string",
                                "description": "The Solana program ID to analyze"
                            },
                            "rpc_url": {
                                "type": "string",
                                "description": "Solana RPC URL (optional, defaults to env or mainnet)"
                            },
                            "network": {
                                "type": "string",
                                "description": "Network name: mainnet-beta, devnet, or testnet (default: mainnet-beta)"
                            }
                        },
                        "required": ["program_id"]
                    }
                },
                {
                    "name": "check_token_security",
                    "description": "Check token security using Rugcheck - returns risk score (0-100), risk level, detailed risks, market data",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "mint_address": {
                                "type": "string",
                                "description": "Token mint address to analyze (base58 encoded Solana address)"
                            }
                        },
                        "required": ["mint_address"]
                    }
                },
                {
                    "name": "check_insider_risk",
                    "description": "Analyze insider trading patterns - detects wash trading, holder inflation, coordinated networks",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "mint_address": {
                                "type": "string",
                                "description": "Token mint address to analyze"
                            }
                        },
                        "required": ["mint_address"]
                    }
                },
                {
                    "name": "check_liquidity_lock",
                    "description": "Check liquidity locks and rugpull risk - verifies locked liquidity percentage and unlock dates",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "mint_address": {
                                "type": "string",
                                "description": "Token mint address to analyze"
                            }
                        },
                        "required": ["mint_address"]
                    }
                },
                {
                    "name": "get_wallet_history",
                    "description": "Get transaction history for a Solana wallet using Helius API. Returns up to 100 transactions with pagination support. Filter by transaction type.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "wallet_address": {
                                "type": "string",
                                "description": "Solana wallet address to query"
                            },
                            "transaction_type": {
                                "type": "string",
                                "description": "Filter by transaction type (optional)",
                                "enum": ["TRANSFER", "SWAP", "NFT_SALE", "NFT_MINT", "TOKEN_MINT", "BURN", "APPROVE", "REVOKE", "UNKNOWN"]
                            },
                            "limit": {
                                "type": "number",
                                "description": "Maximum number of transactions to return (1-100, default: 100)"
                            },
                            "before_cursor": {
                                "type": "string",
                                "description": "Pagination cursor (transaction signature) to fetch transactions before this point"
                            }
                        },
                        "required": ["wallet_address"]
                    }
                },
                {
                    "name": "get_token_accounts_by_delegate",
                    "description": "Get all SPL Token accounts that have approved a specific address as delegate. Useful for finding delegation actions and auditing delegated authorities.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "delegate_address": {
                                "type": "string",
                                "description": "The delegate address to query (the address that has been granted spending authority)"
                            },
                            "program_id": {
                                "type": "string",
                                "description": "Token program ID (optional, defaults to TokenkegQfeZyiNwAJbNbGKPFXCwuBvf9Ss623VQ5DA)"
                            }
                        },
                        "required": ["delegate_address"]
                    }
                }
            ]
        })),
        "tools/call" => {
            let params = request.params.as_ref().and_then(|p| p.as_object());
            if params.is_none() {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: "Invalid params".to_string(),
                    }),
                };
            }

            let tool_name = params.and_then(|p| p.get("name")).and_then(|n| n.as_str());
            let arguments = params.and_then(|p| p.get("arguments"));

            match tool_name {
                Some("scan_wallet") => {
                    handle_scan_wallet(arguments.cloned().unwrap_or(json!({}))).await
                }
                Some("analyze_program") => {
                    handle_analyze_program(arguments.cloned().unwrap_or(json!({}))).await
                }
                Some("check_token_security") => {
                    handle_check_token_security(arguments.cloned().unwrap_or(json!({}))).await
                }
                Some("check_insider_risk") => {
                    handle_check_insider_risk(arguments.cloned().unwrap_or(json!({}))).await
                }
                Some("check_liquidity_lock") => {
                    handle_check_liquidity_lock(arguments.cloned().unwrap_or(json!({}))).await
                }
                Some("get_wallet_history") => {
                    handle_get_wallet_history(arguments.cloned().unwrap_or(json!({}))).await
                }
                Some("get_token_accounts_by_delegate") => {
                    handle_get_token_accounts_by_delegate(arguments.cloned().unwrap_or(json!({})))
                        .await
                }
                _ => Err(anyhow::anyhow!("Unknown tool: {:?}", tool_name)),
            }
        }
        _ => Err(anyhow::anyhow!("Unknown method: {}", request.method)),
    };

    match result {
        Ok(value) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(value),
            error: None,
        },
        Err(e) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: None,
            error: Some(JsonRpcError {
                code: -32603,
                message: format!("{}", e),
            }),
        },
    }
}

async fn handle_scan_wallet(params: Value) -> Result<Value> {
    // Parse arguments
    let wallet = params
        .get("wallet_address")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing wallet_address"))?;

    let default_rpc = default_solana_rpc_primary();

    let rpc_url = params
        .get("rpc_url")
        .and_then(|v| v.as_str())
        .unwrap_or(default_rpc.as_str());

    let max_transactions = params
        .get("max_transactions")
        .and_then(|v| v.as_u64())
        .unwrap_or(100) as usize;

    let time_window_days = params
        .get("time_window_days")
        .and_then(|v| v.as_u64())
        .unwrap_or(30) as u32;

    let output_format = params
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("summary");

    log::info!(
        "Scanning wallet: {} (max_tx: {}, days: {})",
        wallet,
        max_transactions,
        time_window_days
    );

    // Initialize analyzers and rules
    let (registry, engine) = tools::initialize_analyzers_and_rules(None).await?;

    // Create scanner
    let scanner = WalletScanner::with_analyzers(rpc_url.to_string(), registry, engine)?;

    // Configure scan
    let config = ScanConfig {
        max_transactions: Some(max_transactions),
        time_window_days: Some(time_window_days),
        rpc_delay_ms: 0, // Auto-calculated
        check_active_threats: true,
        check_historical: true,
        commitment: CommitmentConfig::confirmed(),
    };

    // Run scan
    let report = scanner.scan(wallet, config).await?;

    // Format output
    let output = match output_format {
        "json" => serde_json::to_string_pretty(&report)?,
        "summary" => tools::format_scan_summary(&report),
        "detailed" => tools::format_scan_detailed(&report),
        _ => tools::format_scan_summary(&report),
    };

    Ok(json!({
        "content": [{
            "type": "text",
            "text": output
        }]
    }))
}

async fn handle_analyze_program(params: Value) -> Result<Value> {
    // Parse arguments
    let program_id = params
        .get("program_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing program_id"))?;

    let default_rpc = default_solana_rpc_primary();

    let rpc_url = params
        .get("rpc_url")
        .and_then(|v| v.as_str())
        .unwrap_or(default_rpc.as_str());

    let network = params
        .get("network")
        .and_then(|v| v.as_str())
        .unwrap_or("mainnet-beta");

    log::info!("Analyzing program: {} on {}", program_id, network);

    // Run analysis
    let result = tools::analyze_program(program_id, rpc_url, network).await?;

    Ok(json!({
        "content": [{
            "type": "text",
            "text": result
        }]
    }))
}

async fn handle_check_token_security(params: Value) -> Result<Value> {
    let mint_address = params
        .get("mint_address")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing mint_address"))?;

    log::info!("Checking token security for: {}", mint_address);
    rugcheck_tools::check_token_security(mint_address).await
}

async fn handle_check_insider_risk(params: Value) -> Result<Value> {
    let mint_address = params
        .get("mint_address")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing mint_address"))?;

    log::info!("Checking insider risk for: {}", mint_address);
    rugcheck_tools::check_insider_risk(mint_address).await
}

async fn handle_check_liquidity_lock(params: Value) -> Result<Value> {
    let mint_address = params
        .get("mint_address")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing mint_address"))?;

    log::info!("Checking liquidity lock for: {}", mint_address);
    rugcheck_tools::check_liquidity_lock(mint_address).await
}

async fn handle_get_wallet_history(params: Value) -> Result<Value> {
    let wallet_address = params
        .get("wallet_address")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing wallet_address"))?;

    let transaction_type = params.get("transaction_type").and_then(|v| v.as_str());

    let limit = params
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize);

    let before_cursor = params.get("before_cursor").and_then(|v| v.as_str());

    log::info!("Fetching wallet history for: {}", wallet_address);
    helius_tools::get_wallet_history(wallet_address, transaction_type, limit, before_cursor).await
}

async fn handle_get_token_accounts_by_delegate(params: Value) -> Result<Value> {
    let delegate_address = params
        .get("delegate_address")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing delegate_address"))?;

    let program_id = params.get("program_id").and_then(|v| v.as_str());

    log::info!(
        "Fetching delegated token accounts for: {}",
        delegate_address
    );
    helius_tools::get_token_accounts_by_delegate(delegate_address, program_id).await
}
