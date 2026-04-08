use crate::auth::AuthContext;
use crate::output::{emit_event, EventBuilder};
use crate::types::AppState;
use axum::{
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use parapet_core::rules::types::RuleDecision;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Vec<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

pub async fn handle_rpc(
    State(state): State<Arc<AppState>>,
    Query(query_params): Query<HashMap<String, String>>,
    mut headers: HeaderMap,
    Json(req): Json<JsonRpcRequest>,
) -> (StatusCode, Json<JsonRpcResponse>) {
    log::debug!("📨 Received RPC request: method={}", req.method);

    // Support URL query parameter authentication
    // If ?api-key= is present and no Authorization header, inject it as X-API-Key header
    if let Some(api_key) = query_params.get("api-key") {
        if !headers.contains_key("Authorization") && !headers.contains_key("X-API-Key") {
            if let Ok(header_value) = HeaderValue::from_str(api_key) {
                headers.insert("X-API-Key", header_value);
                log::debug!("🔑 API key from query parameter");
            }
        }
    }

    // NEW: Authentication with auth provider
    let auth_context = if let Some(auth_provider) = &state.auth_provider {
        match auth_provider.authenticate(&headers, &req.method).await {
            Ok(result) => {
                log::debug!(
                    "✅ Authenticated: {} (tier: {})",
                    result.context.identity,
                    result.context.tier.as_ref().unwrap_or(&"none".to_string())
                );

                // Log quota info if available
                if let (Some(remaining), Some(reset)) = (result.quota_remaining, result.quota_reset)
                {
                    log::debug!("📊 Quota: {} remaining, resets in {}s", remaining, reset);
                }

                result.context
            }
            Err(e) => {
                log::warn!("🚫 Authentication failed: {}", e);

                // Notify auth provider of failure
                let _ = auth_provider
                    .on_failure(None, &req.method, &e.to_string())
                    .await;

                return (
                    StatusCode::UNAUTHORIZED,
                    Json(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32001,
                            message: format!("Authentication failed: {}", e),
                            data: None,
                        }),
                    }),
                );
            }
        }
    } else {
        // No auth provider configured, use anonymous context
        AuthContext::anonymous()
    };

    // Extract wallet address from request (if available)
    let wallet_address = extract_wallet_address(&req);

    // NEW: Check if wallet belongs to authenticated user (TEMPORARILY DISABLED FOR TESTING)
    if let Some(wallet) = &wallet_address {
        if !auth_context.owns_wallet(wallet)
            && !auth_context.has_scope("admin")
            && auth_context.identity != "anonymous"
        {
            log::warn!(
                "⚠️  Wallet {} not owned by {} (owned: {:?}) - ALLOWING FOR DEV/TESTING",
                wallet,
                auth_context.identity,
                auth_context.wallets
            );
            // TEMPORARILY DISABLED - Re-enable for production
            // return (
            //     StatusCode::FORBIDDEN,
            //     Json(JsonRpcResponse {
            //         jsonrpc: "2.0".to_string(),
            //         id: req.id,
            //         result: None,
            //         error: Some(JsonRpcError {
            //             code: -32002,
            //             message: "Transaction wallet does not match authenticated identity"
            //                 .to_string(),
            //             data: None,
            //         }),
            //     }),
            // );
        }
    }

    // Wallet allowlist check
    if let (Some(allowlist), Some(wallet)) = (&state.allowed_wallets, &wallet_address) {
        if !allowlist.contains(wallet) {
            log::warn!("🚫 Wallet not on allowlist: {}", wallet);
            return create_allowlist_error(&req, wallet);
        }
    }

    // Usage tracking and rate limiting
    if let (Some(tracker), Some(wallet)) = (&state.usage_tracker, &wallet_address) {
        match tracker.check_rate_limit(wallet).await {
            Ok(false) => {
                log::warn!("🚫 Rate limit exceeded for wallet: {}", wallet);
                return create_rate_limit_error(&req, wallet);
            }
            Err(e) => {
                log::error!("❌ Error checking rate limit: {}", e);
                // Fail open - allow request if rate limit check fails
            }
            Ok(true) => {
                // Within limits, increment counter
                if let Err(e) = tracker.increment_usage(wallet).await {
                    log::error!("❌ Error incrementing usage: {}", e);
                }
            }
        }
    }

    // Check if this is a transaction send method
    if req.method == "sendTransaction" || req.method == "sendRawTransaction" {
        return handle_transaction_send(state, req, auth_context).await;
    }

    // Check if this is a transaction simulation
    if req.method == "simulateTransaction" {
        return handle_simulate_transaction(state, req, auth_context).await;
    }

    // For all other methods, forward directly to upstream
    match state.upstream_client.forward(&req).await {
        Ok(response) => (StatusCode::OK, Json(response)),
        Err(e) => {
            log::error!("❌ Error forwarding to upstream: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32603,
                        message: format!("Internal error: {}", e),
                        data: None,
                    }),
                }),
            )
        }
    }
}

async fn handle_transaction_send(
    state: Arc<AppState>,
    req: JsonRpcRequest,
    auth_context: AuthContext,
) -> (StatusCode, Json<JsonRpcResponse>) {
    log::info!("🔍 Intercepting transaction send");

    // Extract wallet address for event emission
    let wallet_address = extract_wallet_address(&req);

    // Extract transaction data from params
    if req.params.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: "Invalid params: transaction data required".to_string(),
                    data: None,
                }),
            }),
        );
    }

    let tx_data = &req.params[0];

    // Decode transaction (supports both legacy and v0)
    let transaction = match decode_transaction(tx_data) {
        Ok(tx) => Some(tx),
        Err(e) => {
            log::warn!("⚠️  Unable to decode transaction: {}", e);
            log::info!("📤 Forwarding transaction without analysis");
            None
        }
    };

    // Store the rule decision for later use in ALLOWED event
    let mut rule_decision_for_event: Option<RuleDecision> = None;

    // Evaluate rules if transaction was decoded
    if let Some(ref transaction) = &transaction {
        // Get user's blocking threshold from auth context
        // Falls back to server's default (OSS) or 70 if not configured
        let threshold = auth_context
            .metadata
            .get("blocking_threshold")
            .and_then(|v| v.as_i64())
            .map(|v| v as u8)
            .unwrap_or(state.default_blocking_threshold);

        let engine = state.rule_engine.read().await;
        match engine.evaluate_versioned_with_threshold(transaction, threshold).await {
            Ok(decision) => {
                if decision.matched {
                    log::info!(
                        "📋 Rules evaluated: {} risk (threshold: {}), action: {:?}",
                        decision.total_risk,
                        threshold,
                        decision.action
                    );

                    match decision.action {
                        parapet_core::rules::types::RuleAction::Block => {
                            log::warn!("🚫 Transaction BLOCKED: {}", decision.message);

                            // Emit forensic event
                            let event = EventBuilder::new(
                                wallet_address
                                    .clone()
                                    .unwrap_or_else(|| "unknown".to_string()),
                                req.method.clone(),
                            )
                            .with_auth_context(&auth_context)
                            .with_rule_decision(&decision)
                            .build();
                            emit_event(&state.output_manager, event).await;

                            // Check if escalations are enabled
                            if let Some(escalation_config) = &state.escalation_config {
                                // Compute canonical transaction hash
                                use parapet_core::rules::analyzers::core::CanonicalTransactionAnalyzer;
                                let canonical_hash = match CanonicalTransactionAnalyzer::compute_canonical_hash_versioned(transaction) {
                                    Ok(hash) => hash,
                                    Err(e) => {
                                        log::error!("Failed to compute canonical hash: {}", e);
                                        format!("error:{}", e)
                                    }
                                };

                                // Decode transaction for display
                                use crate::escalations::DecoderRegistry;
                                let decoder = DecoderRegistry::with_defaults();
                                let decoded_instructions = decoder.decode_versioned_transaction(transaction);

                                // Get requester wallet (fee payer)
                                let requester_wallet = wallet_address
                                    .clone()
                                    .unwrap_or_else(|| "unknown".to_string());

                                // Create escalation
                                match crate::escalations::create_escalation(
                                    transaction,
                                    canonical_hash.clone(),
                                    decoded_instructions,
                                    decision.rule_id.clone(),
                                    decision.rule_name.clone(),
                                    decision.message.clone(),
                                    decision.total_risk,
                                    requester_wallet,
                                    escalation_config.approver_wallet.clone(),
                                    &escalation_config.redis_url,
                                ).await {
                                    Ok(escalation) => {
                                        log::info!("🚨 Escalation created: {}", escalation.escalation_id);

                                        // Publish escalation event
                                        if let Err(e) = crate::escalations::publish_escalation_event(
                                            &escalation,
                                            &escalation_config.redis_url,
                                        ).await {
                                            log::error!("Failed to publish escalation event: {}", e);
                                        }

                                        // Return EscalationRequired error
                                        return (
                                            StatusCode::FORBIDDEN,
                                            Json(JsonRpcResponse {
                                                jsonrpc: "2.0".to_string(),
                                                id: req.id,
                                                result: None,
                                                error: Some(JsonRpcError {
                                                    code: -32005,
                                                    message: "EscalationRequired".to_string(),
                                                    data: Some(serde_json::json!({
                                                        "escalation_id": escalation.escalation_id,
                                                        "rule_id": decision.rule_id,
                                                        "rule_name": decision.rule_name,
                                                        "message": decision.message,
                                                        "canonical_hash": canonical_hash,
                                                        "risk_score": decision.total_risk,
                                                    })),
                                                }),
                                            }),
                                        );
                                    }
                                    Err(e) => {
                                        log::error!("Failed to create escalation: {}", e);
                                        // Fall through to regular block response
                                    }
                                }
                            }

                            // Regular block response (no escalations or escalation creation failed)
                            return (
                                StatusCode::FORBIDDEN,
                                Json(JsonRpcResponse {
                                    jsonrpc: "2.0".to_string(),
                                    id: req.id,
                                    result: None,
                                    error: Some(JsonRpcError {
                                        code: -32004,
                                        message: decision.message.clone(),
                                        data: Some(serde_json::json!({
                                            "rule_id": decision.rule_id,
                                            "rule_name": decision.rule_name,
                                            "action": "blocked",
                                        })),
                                    }),
                                }),
                            );
                        }
                        parapet_core::rules::types::RuleAction::Alert => {
                            log::warn!("⚠️  Transaction ALERT: {}", decision.message);
                            // Store decision to include in event after send attempt (with signature)
                            rule_decision_for_event = Some(decision.clone());
                        }
                        parapet_core::rules::types::RuleAction::Pass => {
                            log::info!("✅ Transaction passed security checks");
                            // Store decision to include in ALLOWED event after send attempt
                            rule_decision_for_event = Some(decision.clone());
                        }
                    }
                } else {
                    log::info!("✅ No rules matched, transaction passes");
                }
            }
            Err(e) => {
                log::error!("❌ Error evaluating rules: {}", e);
            }
        }
    }

    // Forward to upstream
    log::info!("📤 Forwarding transaction to upstream");
    match state.upstream_client.forward(&req).await {
        Ok(response) => {
            // Extract signature from response if available
            let signature = response
                .result
                .as_ref()
                .and_then(|r| r.as_str())
                .map(|s| s.to_string());

            // ALWAYS emit event for complete audit trail - with or without signature
            log::info!("📝 Emitting ALLOWED event (signature: {})", 
                signature.as_ref().map(|s| s.as_str()).unwrap_or("none"));
            
            let mut event_builder = EventBuilder::new(
                wallet_address.unwrap_or_else(|| "unknown".to_string()),
                req.method.clone(),
            )
            .with_auth_context(&auth_context);
            
            // Add signature if we got one from upstream
            if let Some(sig) = &signature {
                event_builder = event_builder.with_signature(sig.clone(), None);
            }
            
            // Add rule decision if we evaluated security rules
            if let Some(decision) = rule_decision_for_event {
                event_builder = event_builder.with_rule_decision(&decision);
            }
            
            let event = event_builder.build();
            emit_event(&state.output_manager, event).await;
            log::info!("✅ ALLOWED event emitted (audit trail complete)");

            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            log::error!("❌ Error forwarding to upstream: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32603,
                        message: format!("Internal error: {}", e),
                        data: None,
                    }),
                }),
            )
        }
    }
}

async fn handle_simulate_transaction(
    state: Arc<AppState>,
    req: JsonRpcRequest,
    auth_context: AuthContext,
) -> (StatusCode, Json<JsonRpcResponse>) {
    log::info!("🔬 Intercepting transaction simulation");

    // Extract transaction data from params
    if req.params.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: "Invalid params: transaction data required".to_string(),
                    data: None,
                }),
            }),
        );
    }

    let tx_data = &req.params[0];

    // Decode transaction (supports both legacy and v0)
    let transaction = match decode_transaction(tx_data) {
        Ok(tx) => Some(tx),
        Err(e) => {
            log::warn!("⚠️  Unable to decode transaction for simulation: {}", e);
            log::info!("📤 Forwarding simulation without analysis");
            None
        }
    };

    // Get user's blocking threshold
    let threshold = auth_context
        .metadata
        .get("blocking_threshold")
        .and_then(|v| v.as_i64())
        .map(|v| v as u8)
        .unwrap_or(state.default_blocking_threshold);

    // Forward to upstream RPC for simulation
    let mut response = match state.upstream_client.forward(&req).await {
        Ok(response) => response,
        Err(e) => {
            log::error!("❌ Error forwarding simulation to upstream: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32603,
                        message: format!("Internal error: {}", e),
                        data: None,
                    }),
                }),
            );
        }
    };

    // Analyze transaction and simulation result
    if let (Some(tx), Some(result)) = (&transaction, &mut response.result) {
        log::info!("🔍 Running Parapet analysis on simulation");

        // Get simulation value (nested under result.value for simulation responses)
        let simulation_value = result.get("value").cloned().unwrap_or_else(|| result.clone());

        let engine = state.rule_engine.read().await;
        match engine
            .evaluate_for_simulation(tx, &simulation_value, &state.simulation_registry, threshold)
            .await
        {
            Ok(decision) => {
                log::info!(
                    "📊 Simulation analysis complete: {} total risk (structural: {}, simulation: {})",
                    decision.total_risk,
                    decision.structural_risk.unwrap_or(0),
                    decision.simulation_risk.unwrap_or(0)
                );

                // Build Parapet metadata
                let sol_shield_data = build_sol_shield_metadata(&decision, threshold);

                // Inject solShield into response
                if let Some(result_obj) = result.as_object_mut() {
                    result_obj.insert("solShield".to_string(), sol_shield_data);
                    log::debug!("✅ Injected solShield metadata into simulation response");
                }
            }
            Err(e) => {
                log::error!("❌ Error analyzing simulation: {}", e);
            }
        }
    }

    log::info!("📥 Simulation completed with Parapet analysis");
    (StatusCode::OK, Json(response))
}

/// Build structured Parapet metadata for simulation responses
fn build_sol_shield_metadata(decision: &RuleDecision, threshold: u8) -> Value {
    use serde_json::json;

    // Determine decision label
    let decision_label = if decision.total_risk >= threshold {
        "would_block"
    } else if decision.total_risk > 0 {
        "alert"
    } else {
        "safe"
    };

    // Build warnings array
    let warnings: Vec<Value> = decision
        .matched_rules
        .iter()
        .map(|rule| {
            let severity = if rule.weight >= 50 {
                "critical"
            } else if rule.weight >= 30 {
                "high"
            } else if rule.weight >= 15 {
                "medium"
            } else {
                "low"
            };

            json!({
                "severity": severity,
                "message": rule.message,
                "ruleId": rule.rule_id,
                "ruleName": rule.rule_name,
                "weight": rule.weight,
            })
        })
        .collect();

    json!({
        "version": "1.0",
        "riskScore": decision.total_risk,
        "structuralRisk": decision.structural_risk.unwrap_or(0),
        "simulationRisk": decision.simulation_risk.unwrap_or(0),
        "decision": decision_label,
        "threshold": threshold,
        "warnings": warnings,
        "analysis": {
            "matchedRules": decision.matched_rules.len(),
            "totalWeight": decision.total_risk,
            "wouldBlock": decision.total_risk >= threshold,
        }
    })
}

fn extract_wallet_address(req: &JsonRpcRequest) -> Option<String> {
    // Check for wallet address in params (second parameter)
    if req.params.len() >= 2 {
        if let Some(config) = req.params[1].as_object() {
            if let Some(wallet) = config.get("walletAddress") {
                if let Some(addr) = wallet.as_str() {
                    return Some(addr.to_string());
                }
            }
        }
    }

    // Try to extract from transaction data for sendTransaction
    if req.method == "sendTransaction" || req.method == "sendRawTransaction" {
        if let Some(tx_data) = req.params.first() {
            // Try to decode and get fee payer
            if let Ok(tx) = decode_transaction_for_wallet(tx_data) {
                return Some(tx);
            }
        }
    }

    None
}

fn decode_transaction(
    tx_data: &Value,
) -> anyhow::Result<solana_sdk::transaction::VersionedTransaction> {
    let tx_bytes = if let Some(tx_str) = tx_data.as_str() {
        bs58::decode(tx_str).into_vec().or_else(|_| {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.decode(tx_str)
        })?
    } else {
        return Err(anyhow::anyhow!("Invalid transaction data"));
    };

    // Deserialize as VersionedTransaction (supports both v0 and legacy)
    bincode::deserialize::<solana_sdk::transaction::VersionedTransaction>(&tx_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize transaction: {}", e))
}

fn decode_transaction_for_wallet(tx_data: &Value) -> anyhow::Result<String> {
    use solana_sdk::message::VersionedMessage;

    let transaction = decode_transaction(tx_data)?;

    // Get fee payer (first account) - handle both v0 and legacy
    let account_keys = match &transaction.message {
        VersionedMessage::V0(v0_msg) => &v0_msg.account_keys,
        VersionedMessage::Legacy(legacy_msg) => &legacy_msg.account_keys,
    };

    if let Some(key) = account_keys.first() {
        return Ok(key.to_string());
    }

    Err(anyhow::anyhow!("No fee payer found"))
}

fn create_allowlist_error(
    req: &JsonRpcRequest,
    wallet_address: &str,
) -> (StatusCode, Json<JsonRpcResponse>) {
    (
        StatusCode::FORBIDDEN,
        Json(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id.clone(),
            result: None,
            error: Some(JsonRpcError {
                code: -32003,
                message: "Wallet not on allowlist".to_string(),
                data: Some(serde_json::json!({
                    "wallet": wallet_address,
                    "message": "This wallet address is not authorized to use this RPC endpoint.",
                })),
            }),
        }),
    )
}

fn create_rate_limit_error(
    req: &JsonRpcRequest,
    wallet_address: &str,
) -> (StatusCode, Json<JsonRpcResponse>) {
    (
        StatusCode::TOO_MANY_REQUESTS,
        Json(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id.clone(),
            result: None,
            error: Some(JsonRpcError {
                code: -32005,
                message: "Rate limit exceeded".to_string(),
                data: Some(serde_json::json!({
                    "wallet": wallet_address,
                    "message": "Monthly request limit reached for this wallet address.",
                })),
            }),
        }),
    )
}
