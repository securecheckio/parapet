use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{sse::Event, IntoResponse, Response, Sse},
    Json,
};
use futures::stream::{self, Stream};
use parapet_scanner::{ScanConfig, WalletScanner};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_commitment_config::CommitmentConfig;
use std::{convert::Infallible, time::Duration};

use crate::ApiStateAccess;

/// MCP Protocol version
const MCP_VERSION: &str = "2024-11-05";

#[derive(Debug, Deserialize)]
pub struct McpRequest {
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
pub struct McpResponse {
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpError>,
}

#[derive(Debug, Serialize)]
pub struct McpError {
    code: i32,
    message: String,
}

/// SSE endpoint for MCP - Main entry point for MCP clients
/// URL: GET /mcp/sse
pub async fn mcp_sse_handler<S>(
    Query(_params): Query<std::collections::HashMap<String, String>>,
    headers: HeaderMap,
    State(_state): State<S>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, Response>
where
    S: ApiStateAccess,
{
    // Verify API key
    if let Err(response) = verify_api_key(&headers) {
        return Err(response);
    }

    log::info!("MCP SSE connection established");

    // Create SSE stream
    let stream = stream::iter(vec![
        // Send initial capabilities
        Ok(Event::default()
            .json_data(json!({
                "jsonrpc": "2.0",
                "method": "initialize",
                "result": {
                    "protocolVersion": MCP_VERSION,
                    "serverInfo": {
                        "name": "parapet",
                        "version": env!("CARGO_PKG_VERSION")
                    },
                    "capabilities": {
                        "tools": {},
                        "resources": {}
                    }
                }
            }))
            .unwrap()),
    ]);

    Ok(Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    ))
}

/// JSON-RPC endpoint for MCP tools
/// URL: POST /mcp/message
pub async fn mcp_message_handler<S>(
    headers: HeaderMap,
    State(state): State<S>,
    Json(request): Json<McpRequest>,
) -> Result<Json<McpResponse>, Response>
where
    S: ApiStateAccess,
{
    // Verify API key and get it
    let api_key = match verify_api_key(&headers) {
        Ok(key) => key,
        Err(response) => return Err(response),
    };

    log::info!("MCP request: method={}", request.method);

    let result = match request.method.as_str() {
        "resources/list" => handle_list_resources(),
        "resources/read" => handle_resource_read(request.params),
        "tools/list" => handle_list_tools(),
        "tools/call" => handle_tool_call(request.params, &state, &api_key).await,
        "ping" => Ok(json!({"status": "ok"})),
        _ => Err(McpError {
            code: -32601,
            message: format!("Method not found: {}", request.method),
        }),
    };

    match result {
        Ok(value) => Ok(Json(McpResponse {
            result: Some(value),
            error: None,
        })),
        Err(error) => Err((
            StatusCode::BAD_REQUEST,
            Json(McpResponse {
                result: None,
                error: Some(error),
            }),
        )
            .into_response()),
    }
}

fn handle_list_resources() -> Result<Value, McpError> {
    Ok(json!({
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
    }))
}

fn handle_resource_read(params: Value) -> Result<Value, McpError> {
    let uri = params
        .get("uri")
        .and_then(|u| u.as_str())
        .ok_or_else(|| McpError {
            code: -32602,
            message: "Missing required parameter: uri".to_string(),
        })?;

    let content = match uri {
        "parapet://guide" => include_str!("../../../mcp/resources/guide.md"),
        "parapet://risk-scoring" => include_str!("../../../mcp/resources/risk-scoring.md"),
        "parapet://examples" => include_str!("../../../mcp/resources/examples.md"),
        _ => {
            return Err(McpError {
                code: -32602,
                message: format!("Unknown resource URI: {}", uri),
            })
        }
    };

    Ok(json!({
        "contents": [{
            "uri": uri,
            "mimeType": "text/markdown",
            "text": content
        }]
    }))
}

fn handle_list_tools() -> Result<Value, McpError> {
    Ok(json!({
        "tools": [
            {
                "name": "scan_wallet",
                "description": "Comprehensive wallet scan: on-chain analysis + reputation data from Rugcheck, Helius, Jupiter. Takes 5-10 minutes for 100 transactions.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "wallet_address": {
                            "type": "string",
                            "description": "The Solana wallet address to scan"
                        },
                        "max_transactions": {
                            "type": "number",
                            "description": "Maximum transactions to analyze (default: 100, max: 500)"
                        },
                        "time_window_days": {
                            "type": "number",
                            "description": "Days to scan back (default: 30, max: 90)"
                        },
                        "format": {
                            "type": "string",
                            "description": "Output format: summary, detailed, or json",
                            "enum": ["summary", "detailed", "json"]
                        }
                    },
                    "required": ["wallet_address"]
                }
            },
            {
                "name": "analyze_program",
                "description": "Analyze a Solana program: on-chain data + verification status from Helius and OtterSec",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "program_id": {
                            "type": "string",
                            "description": "The Solana program ID to analyze"
                        },
                        "network": {
                            "type": "string",
                            "description": "Network name: mainnet-beta, devnet, or testnet",
                            "enum": ["mainnet-beta", "devnet", "testnet"]
                        }
                    },
                    "required": ["program_id"]
                }
            },
            {
                "name": "check_token_reputation",
                "description": "Get token reputation and risk data from Rugcheck and Jupiter (fast, no transaction scan)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "token_address": {
                            "type": "string",
                            "description": "The SPL token mint address to check"
                        }
                    },
                    "required": ["token_address"]
                }
            },
            {
                "name": "verify_program",
                "description": "Check program verification status from Helius and OtterSec (fast, direct lookup)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "program_address": {
                            "type": "string",
                            "description": "The Solana program address to verify"
                        }
                    },
                    "required": ["program_address"]
                }
            },
            {
                "name": "generate_revoke_transaction",
                "description": "Generate an unsigned transaction to revoke a token approval/delegation. Returns base64-encoded transaction for user to sign.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "wallet_address": {
                            "type": "string",
                            "description": "The wallet address (owner of the token account)"
                        },
                        "token_account": {
                            "type": "string",
                            "description": "The token account address with the delegation to revoke"
                        },
                        "token_symbol": {
                            "type": "string",
                            "description": "Optional token symbol for display (e.g., 'USDC', 'SOL')"
                        }
                    },
                    "required": ["wallet_address", "token_account"]
                }
            },
            {
                "name": "generate_batch_revoke",
                "description": "Generate unsigned transaction(s) to revoke multiple token approvals at once. Returns base64-encoded transactions for user to sign.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "wallet_address": {
                            "type": "string",
                            "description": "The wallet address (owner of the token accounts)"
                        },
                        "token_accounts": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            },
                            "description": "Array of token account addresses with delegations to revoke"
                        }
                    },
                    "required": ["wallet_address", "token_accounts"]
                }
            },
            {
                "name": "build_emergency_lockdown",
                "description": "Scan wallet for dangerous approvals and generate revoke transactions. One-stop solution for wallet security.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "wallet_address": {
                            "type": "string",
                            "description": "The wallet address to secure"
                        },
                        "severity_threshold": {
                            "type": "string",
                            "description": "Minimum severity to revoke: critical, high, medium, or low",
                            "enum": ["critical", "high", "medium", "low"]
                        }
                    },
                    "required": ["wallet_address"]
                }
            }
        ]
    }))
}

async fn handle_tool_call<S>(params: Value, state: &S, api_key: &str) -> Result<Value, McpError>
where
    S: ApiStateAccess,
{
    let tool_name = params
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| McpError {
            code: -32602,
            message: "Missing tool name".to_string(),
        })?;

    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    match tool_name {
        "scan_wallet" => scan_wallet_tool(arguments, state, api_key).await,
        "analyze_program" => analyze_program_tool(arguments, state).await,
        "check_token_reputation" => check_token_reputation_tool(arguments).await,
        "verify_program" => verify_program_tool(arguments).await,
        "generate_revoke_transaction" => generate_revoke_transaction_tool(arguments, state).await,
        "generate_batch_revoke" => generate_batch_revoke_tool(arguments, state).await,
        "build_emergency_lockdown" => {
            build_emergency_lockdown_tool(arguments, state, api_key).await
        }
        _ => Err(McpError {
            code: -32602,
            message: format!("Unknown tool: {}", tool_name),
        }),
    }
}

async fn scan_wallet_tool<S>(args: Value, state: &S, api_key: &str) -> Result<Value, McpError>
where
    S: ApiStateAccess,
{
    // Check rate limit quota
    let quota_permit = state
        .mcp_rate_limiter()
        .check_quota(api_key)
        .await
        .map_err(|e| match e {
            crate::middleware::RateLimitError::QuotaExceeded {
                limit,
                reset_in_seconds,
            } => McpError {
                code: 429,
                message: format!(
                    "Quota exceeded: {} scans per hour. Resets in {} minutes.",
                    limit,
                    reset_in_seconds / 60
                ),
            },
            crate::middleware::RateLimitError::TooManyConcurrentScans { max_concurrent } => {
                McpError {
                    code: 503,
                    message: format!(
                        "Too many concurrent scans (max: {}). Please try again in a moment.",
                        max_concurrent
                    ),
                }
            }
        })?;

    let wallet = args
        .get("wallet_address")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError {
            code: -32602,
            message: "Missing wallet_address".to_string(),
        })?;

    let max_tx = args
        .get("max_transactions")
        .and_then(|v| v.as_u64())
        .unwrap_or(100)
        .min(500) as usize;

    let days = args
        .get("time_window_days")
        .and_then(|v| v.as_u64())
        .unwrap_or(30)
        .min(90) as u32;

    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("summary");

    log::info!(
        "Scanning wallet {} (tx: {}, days: {})",
        wallet,
        max_tx,
        days
    );

    // Initialize scanner with analyzers
    let (registry, engine) = crate::routes::mcp_tools::initialize_analyzers_and_rules(None)
        .await
        .map_err(|e| McpError {
            code: -32603,
            message: format!("Failed to initialize analyzers: {}", e),
        })?;

    let scanner =
        WalletScanner::with_analyzers(state.config().solana_rpc_url.clone(), registry, engine)
            .map_err(|e| McpError {
                code: -32603,
                message: format!("Failed to create scanner: {}", e),
            })?;

    let config = ScanConfig {
        max_transactions: Some(max_tx),
        time_window_days: Some(days),
        rpc_delay_ms: 0,
        check_active_threats: true,
        check_historical: true,
        commitment: CommitmentConfig::confirmed(),
    };

    let report = scanner.scan(wallet, config).await.map_err(|e| McpError {
        code: -32603,
        message: format!("Scan failed: {}", e),
    })?;

    let output = match format {
        "json" => serde_json::to_string_pretty(&report).map_err(|e| McpError {
            code: -32603,
            message: format!("Failed to serialize: {}", e),
        })?,
        "detailed" => crate::routes::mcp_tools::format_scan_detailed(&report),
        _ => crate::routes::mcp_tools::format_scan_summary(&report),
    };

    // Add quota info to response
    let quota_info = format!(
        "\n\n---\n💡 **API Usage:** {} scans remaining this hour (resets in {} minutes)",
        quota_permit.scans_remaining,
        quota_permit.reset_in_seconds / 60
    );

    Ok(json!({
        "content": [{
            "type": "text",
            "text": output + &quota_info
        }]
    }))
}

async fn analyze_program_tool<S>(args: Value, state: &S) -> Result<Value, McpError>
where
    S: ApiStateAccess,
{
    let program_id = args
        .get("program_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError {
            code: -32602,
            message: "Missing program_id".to_string(),
        })?;

    let network = args
        .get("network")
        .and_then(|v| v.as_str())
        .unwrap_or("mainnet-beta");

    log::info!("Analyzing program {} on {}", program_id, network);

    let result = crate::routes::mcp_tools::analyze_program(
        program_id,
        &state.config().solana_rpc_url,
        network,
    )
    .await
    .map_err(|e| McpError {
        code: -32603,
        message: format!("Analysis failed: {}", e),
    })?;

    Ok(json!({
        "content": [{
            "type": "text",
            "text": result
        }]
    }))
}

async fn check_token_reputation_tool(args: Value) -> Result<Value, McpError> {
    let token_address = args
        .get("token_address")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError {
            code: -32602,
            message: "Missing token_address".to_string(),
        })?;

    let output = crate::routes::mcp_tools::check_token_reputation(token_address)
        .await
        .map_err(|e| McpError {
            code: -32603,
            message: format!("Failed to check token reputation: {}", e),
        })?;

    Ok(json!({
        "content": [{
            "type": "text",
            "text": output
        }]
    }))
}

async fn verify_program_tool(args: Value) -> Result<Value, McpError> {
    let program_address = args
        .get("program_address")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError {
            code: -32602,
            message: "Missing program_address".to_string(),
        })?;

    let output = crate::routes::mcp_tools::verify_program_status(program_address)
        .await
        .map_err(|e| McpError {
            code: -32603,
            message: format!("Failed to verify program: {}", e),
        })?;

    Ok(json!({
        "content": [{
            "type": "text",
            "text": output
        }]
    }))
}

async fn generate_revoke_transaction_tool<S>(args: Value, state: &S) -> Result<Value, McpError>
where
    S: ApiStateAccess,
{
    let wallet = args
        .get("wallet_address")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError {
            code: -32602,
            message: "Missing wallet_address".to_string(),
        })?;

    let token_account = args
        .get("token_account")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError {
            code: -32602,
            message: "Missing token_account".to_string(),
        })?;

    let token_symbol = args.get("token_symbol").and_then(|v| v.as_str());

    log::info!(
        "Generating revoke transaction for {} on {}",
        wallet,
        token_account
    );

    // Create RPC client
    let rpc_client =
        solana_client::rpc_client::RpcClient::new(state.config().solana_rpc_url.clone());

    // Build revoke transaction
    let unsigned_tx =
        crate::tx_builder::build_revoke_approval_tx(wallet, token_account, &rpc_client)
            .await
            .map_err(|e| McpError {
                code: -32603,
                message: format!("Failed to build revoke transaction: {}", e),
            })?;

    // Generate description
    let description =
        crate::tx_builder::describe_revoke_transaction(wallet, token_account, token_symbol);

    let output = format!(
        "# Revoke Transaction Generated\n\n\
         {}\n\n\
         ## Transaction Details\n\
         **Base64 Encoded Transaction:**\n\
         ```\n{}\n```\n\n\
         ## Instructions for Signing\n\
         1. Copy the base64 transaction above\n\
         2. Paste it into your wallet (Phantom, Solflare, etc.)\n\
         3. Review the transaction details\n\
         4. Sign and submit\n\n\
         After signing, the approval will be revoked and your tokens will be safe from the delegate.",
        description,
        unsigned_tx
    );

    Ok(json!({
        "content": [{
            "type": "text",
            "text": output
        }]
    }))
}

async fn generate_batch_revoke_tool<S>(args: Value, state: &S) -> Result<Value, McpError>
where
    S: ApiStateAccess,
{
    let wallet = args
        .get("wallet_address")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError {
            code: -32602,
            message: "Missing wallet_address".to_string(),
        })?;

    let token_accounts = args
        .get("token_accounts")
        .and_then(|v| v.as_array())
        .ok_or_else(|| McpError {
            code: -32602,
            message: "Missing or invalid token_accounts array".to_string(),
        })?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect::<Vec<String>>();

    if token_accounts.is_empty() {
        return Err(McpError {
            code: -32602,
            message: "token_accounts array is empty".to_string(),
        });
    }

    log::info!(
        "Generating batch revoke for {} on {} accounts",
        wallet,
        token_accounts.len()
    );

    // Create RPC client
    let rpc_client =
        solana_client::rpc_client::RpcClient::new(state.config().solana_rpc_url.clone());

    // Build batch revoke transactions
    let unsigned_txs =
        crate::tx_builder::build_batch_revoke_tx(wallet, &token_accounts, &rpc_client)
            .await
            .map_err(|e| McpError {
                code: -32603,
                message: format!("Failed to build batch revoke transactions: {}", e),
            })?;

    // Generate description
    let description =
        crate::tx_builder::describe_batch_revoke(wallet, &token_accounts, unsigned_txs.len());

    let mut output = format!(
        "# Batch Revoke Transactions Generated\n\n\
         {}\n\n\
         ## Transactions ({} total)\n\n",
        description,
        unsigned_txs.len()
    );

    for (i, tx) in unsigned_txs.iter().enumerate() {
        output.push_str(&format!(
            "### Transaction {} of {}\n\
             **Base64 Encoded:**\n\
             ```\n{}\n```\n\n",
            i + 1,
            unsigned_txs.len(),
            tx
        ));
    }

    output.push_str(
        "## Instructions for Signing\n\
         1. Copy each base64 transaction\n\
         2. Paste into your wallet (Phantom, Solflare, etc.)\n\
         3. Review the transaction details\n\
         4. Sign and submit **all transactions** in order\n\n\
         After signing all transactions, all approvals will be revoked and your tokens will be safe."
    );

    Ok(json!({
        "content": [{
            "type": "text",
            "text": output
        }]
    }))
}

async fn build_emergency_lockdown_tool<S>(
    args: Value,
    state: &S,
    _api_key: &str,
) -> Result<Value, McpError>
where
    S: ApiStateAccess,
{
    use parapet_scanner::{Severity, ThreatType};

    let wallet = args
        .get("wallet_address")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError {
            code: -32602,
            message: "Missing wallet_address".to_string(),
        })?;

    let severity_threshold = args
        .get("severity_threshold")
        .and_then(|v| v.as_str())
        .unwrap_or("high");

    log::info!(
        "Building emergency lockdown for {} (threshold: {})",
        wallet,
        severity_threshold
    );

    // Parse severity threshold
    let min_severity = match severity_threshold {
        "critical" => Severity::Critical,
        "high" => Severity::High,
        "medium" => Severity::Medium,
        "low" => Severity::Low,
        _ => Severity::High,
    };

    // Scan wallet for threats (using scan_wallet_tool logic but without quota)
    let (registry, engine) = crate::routes::mcp_tools::initialize_analyzers_and_rules(None)
        .await
        .map_err(|e| McpError {
            code: -32603,
            message: format!("Failed to initialize analyzers: {}", e),
        })?;

    let scanner =
        WalletScanner::with_analyzers(state.config().solana_rpc_url.clone(), registry, engine)
            .map_err(|e| McpError {
                code: -32603,
                message: format!("Failed to create scanner: {}", e),
            })?;

    let config = ScanConfig {
        max_transactions: Some(100),
        time_window_days: Some(30),
        rpc_delay_ms: 0,
        check_active_threats: true,
        check_historical: false, // Only check active threats for speed
        commitment: CommitmentConfig::confirmed(),
    };

    let report = scanner.scan(wallet, config).await.map_err(|e| McpError {
        code: -32603,
        message: format!("Scan failed: {}", e),
    })?;

    // Filter for dangerous approvals that meet severity threshold
    let mut dangerous_approvals = Vec::new();
    for threat in &report.threats {
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

        match &threat.threat_type {
            ThreatType::ActiveUnlimitedDelegation {
                token_account,
                delegate,
                amount,
                ..
            } => {
                dangerous_approvals.push((token_account.clone(), delegate.clone(), *amount));
            }
            ThreatType::PossibleExploitedDelegation {
                token_account,
                delegate,
                amount,
                ..
            } => {
                dangerous_approvals.push((token_account.clone(), delegate.clone(), *amount));
            }
            _ => {}
        }
    }

    if dangerous_approvals.is_empty() {
        return Ok(json!({
            "content": [{
                "type": "text",
                "text": format!(
                    "# Wallet Security Check\n\n\
                     ✅ **No dangerous approvals found**\n\n\
                     Wallet `{}` has no token approvals meeting the {} severity threshold.\n\
                     Your wallet is safe from delegate-based attacks.",
                    wallet, severity_threshold
                )
            }]
        }));
    }

    // Extract just the token accounts
    let token_accounts: Vec<String> = dangerous_approvals
        .iter()
        .map(|(account, _, _)| account.clone())
        .collect();

    // Create RPC client
    let rpc_client =
        solana_client::rpc_client::RpcClient::new(state.config().solana_rpc_url.clone());

    // Build batch revoke transactions
    let unsigned_txs =
        crate::tx_builder::build_batch_revoke_tx(wallet, &token_accounts, &rpc_client)
            .await
            .map_err(|e| McpError {
                code: -32603,
                message: format!("Failed to build revoke transactions: {}", e),
            })?;

    // Generate detailed output
    let mut output = format!(
        "# 🚨 Emergency Lockdown for Wallet\n\n\
         **Wallet:** `{}`\n\
         **Threats Found:** {} dangerous approvals\n\
         **Severity Threshold:** {}\n\n\
         ## Dangerous Approvals Detected\n\n",
        wallet,
        dangerous_approvals.len(),
        severity_threshold
    );

    for (i, (token_account, delegate, amount)) in dangerous_approvals.iter().enumerate() {
        let amount_str = if *amount == u64::MAX {
            "UNLIMITED".to_string()
        } else {
            amount.to_string()
        };

        output.push_str(&format!(
            "{}. **Token Account:** `{}`\n\
             - Delegate: `{}`\n\
             - Amount: {}\n\n",
            i + 1,
            token_account,
            delegate,
            amount_str
        ));
    }

    output.push_str(&format!(
        "## Revoke Transactions ({} total)\n\n\
         The following transactions will revoke all dangerous approvals:\n\n",
        unsigned_txs.len()
    ));

    for (i, tx) in unsigned_txs.iter().enumerate() {
        output.push_str(&format!(
            "### Transaction {} of {}\n\
             **Base64 Encoded:**\n\
             ```\n{}\n```\n\n",
            i + 1,
            unsigned_txs.len(),
            tx
        ));
    }

    output.push_str(
        "## ⚠️ URGENT: Sign These Transactions Now\n\n\
         1. Copy each base64 transaction above\n\
         2. Paste into your wallet (Phantom, Solflare, etc.)\n\
         3. **Review carefully** - ensure you recognize the token accounts\n\
         4. Sign and submit **all transactions** in order\n\n\
         **After signing:**\n\
         - All dangerous approvals will be revoked\n\
         - Delegates will NO LONGER be able to transfer your tokens\n\
         - Your wallet will be secured\n\n\
         **Time is critical!** Sign these transactions as soon as possible to protect your assets.",
    );

    Ok(json!({
        "content": [{
            "type": "text",
            "text": output
        }]
    }))
}

/// Verify API key from Authorization header and return the key
fn verify_api_key(headers: &HeaderMap) -> Result<String, Response> {
    let api_key = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "Missing or invalid Authorization header. Use: Authorization: Bearer YOUR_API_KEY"
                })),
            )
                .into_response()
        })?
        .to_string();

    // Get valid API keys from environment
    let api_keys_env = std::env::var("MCP_API_KEYS").unwrap_or_default();
    let valid_keys = api_keys_env
        .split(',')
        .map(|k| k.trim())
        .filter(|k| !k.is_empty())
        .collect::<Vec<_>>();

    if valid_keys.is_empty() {
        log::warn!("⚠️  No MCP_API_KEYS configured - MCP access disabled for security");
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": "MCP service not configured. Contact administrator."
            })),
        )
            .into_response());
    }

    if !valid_keys.contains(&api_key.as_str()) {
        log::warn!("Invalid API key attempt");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "Invalid API key"
            })),
        )
            .into_response());
    }

    Ok(api_key)
}
