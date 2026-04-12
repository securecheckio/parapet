use crate::auth::{verify_timestamp, verify_wallet_signature};
use crate::ApiStateAccess;
use crate::types::*;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use redis::AsyncCommands;
use std::time::{SystemTime, UNIX_EPOCH};

/// Get escalation details
pub async fn get_escalation<S>(
    State(state): State<S>,
    Path(escalation_id): Path<String>,
) -> Result<Json<Escalation>, (StatusCode, String)>
where
    S: ApiStateAccess,
{
    let redis_conn = match state.redis().as_ref() {
        Some(conn) => conn,
        None => return Err((StatusCode::SERVICE_UNAVAILABLE, "Redis unavailable".to_string())),
    };
    let mut redis = redis_conn.clone();
    
    // Get escalation from Redis
    let key = format!("escalation:pending:{}", escalation_id);
    let escalation_json: String = redis.get(&key).await.map_err(|e| {
        log::error!("Failed to get escalation {}: {}", escalation_id, e);
        (StatusCode::NOT_FOUND, "Escalation not found".to_string())
    })?;
    
    let escalation: Escalation = serde_json::from_str(&escalation_json).map_err(|e| {
        log::error!("Failed to parse escalation: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Invalid escalation data".to_string())
    })?;
    
    Ok(Json(escalation))
}

/// Approve an escalation
pub async fn approve_escalation<S>(
    State(state): State<S>,
    Path(escalation_id): Path<String>,
    Json(req): Json<ApproveEscalationRequest>,
) -> Result<Json<ApprovalResponse>, (StatusCode, String)>
where
    S: ApiStateAccess,
{
    // Verify timestamp
    verify_timestamp(req.timestamp).map_err(|e| {
        (StatusCode::BAD_REQUEST, format!("Invalid timestamp: {}", e))
    })?;
    
    // Verify signature
    verify_wallet_signature(&req.approver_wallet, &req.message, &req.signature).map_err(|e| {
        (StatusCode::UNAUTHORIZED, format!("Signature verification failed: {}", e))
    })?;
    
    // Check Redis availability
    let redis_conn = match state.redis().as_ref() {
        Some(conn) => conn,
        None => return Err((StatusCode::SERVICE_UNAVAILABLE, "Redis unavailable".to_string())),
    };
    
    // Verify nonce
    let mut redis = redis_conn.clone();
    let nonce_key = format!("nonce:{}:{}", req.approver_wallet, req.nonce);
    
    let nonce_exists: bool = redis.exists(&nonce_key).await.map_err(|e| {
        log::error!("Failed to check nonce: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to verify nonce".to_string())
    })?;
    
    if !nonce_exists {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid or expired nonce".to_string(),
        ));
    }
    
    // Delete nonce (one-time use)
    redis.del::<_, ()>(&nonce_key).await.ok();
    
    // Get escalation
    let escalation_key = format!("escalation:pending:{}", escalation_id);
    let escalation_json: String = redis.get(&escalation_key).await.map_err(|_| {
        (StatusCode::NOT_FOUND, "Escalation not found".to_string())
    })?;
    
    let mut escalation: Escalation = serde_json::from_str(&escalation_json).map_err(|e| {
        log::error!("Failed to parse escalation: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Invalid escalation data".to_string())
    })?;
    
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    // FAST PATH: Check if original transaction is still in Redis (< 60s)
    let tx_key = format!("pending_tx:{}", escalation_id);
    
    if let Ok(tx_bytes) = redis.get::<_, Vec<u8>>(&tx_key).await {
        log::info!("⚡ Fast path: forwarding original transaction for {}", escalation_id);
        
        // Forward transaction to Solana network
        match forward_transaction_to_network(&tx_bytes, &state.config().solana_rpc_url).await {
            Ok(signature) => {
                // Update escalation status
                escalation.status = EscalationStatus::ApprovedFastPath;
                let escalation_json = serde_json::to_string(&escalation).unwrap();
                redis.set::<_, _, ()>(&escalation_key, &escalation_json).await.ok();
                
                // Publish event
                let event = EscalationEvent::Forwarded {
                    escalation_id: escalation_id.clone(),
                    signature: signature.clone(),
                    forwarded_at: now,
                };
                
                let event_channel = format!("escalation:events:{}", escalation.approver_wallet);
                redis
                    .publish::<_, _, ()>(&event_channel, serde_json::to_string(&event).unwrap())
                    .await
                    .ok();
                
                log::info!("✅ Transaction forwarded: {}", signature);
                
                return Ok(Json(ApprovalResponse::TransactionForwarded {
                    signature,
                    fast_path: true,
                    message: "Transaction forwarded immediately (fast path)".to_string(),
                }));
            }
            Err(e) => {
                log::error!("Failed to forward transaction: {}", e);
                // Fall through to slow path
            }
        }
    }
    
    // SLOW PATH: Create dynamic rule
    log::info!("🐢 Slow path: creating rule for {}", escalation_id);
    
    let rule_key = format!("dynamic_rules:{}", req.rule.id);
    let rule_json = serde_json::to_string(&req.rule).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize rule: {}", e))
    })?;
    
    // Store rule with expiry if specified
    if let Some(expires_at) = req.rule.expires_at {
        let ttl = expires_at.saturating_sub(now);
        redis
            .set_ex::<_, _, ()>(&rule_key, &rule_json, ttl as u64)
            .await
            .map_err(|e| {
                log::error!("Failed to store rule: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create rule".to_string())
            })?;
    } else {
        redis
            .set::<_, _, ()>(&rule_key, &rule_json)
            .await
            .map_err(|e| {
                log::error!("Failed to store rule: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create rule".to_string())
            })?;
    }
    
    // Update escalation status
    escalation.status = EscalationStatus::ApprovedSlowPath;
    let escalation_json = serde_json::to_string(&escalation).unwrap();
    redis.set::<_, _, ()>(&escalation_key, &escalation_json).await.ok();
    
    // Publish event
    let event = EscalationEvent::Approved {
        escalation_id: escalation_id.clone(),
        approved_by: req.approver_wallet.clone(),
        approved_at: now,
        rule: req.rule.clone(),
    };
    
    let event_channel = format!("escalation:events:{}", escalation.approver_wallet);
    redis
        .publish::<_, _, ()>(&event_channel, serde_json::to_string(&event).unwrap())
        .await
        .ok();
    
    // Publish rule update event
    redis
        .publish::<_, _, ()>("dynamic_rules:updated", &req.rule.id)
        .await
        .ok();
    
    log::info!("✅ Rule created: {} for escalation {}", req.rule.id, escalation_id);
    
    Ok(Json(ApprovalResponse::RuleCreated {
        rule_id: req.rule.id.clone(),
        fast_path: false,
        message: "Rule created, agent must retry with fresh blockhash".to_string(),
    }))
}

/// Deny an escalation
pub async fn deny_escalation<S>(
    State(state): State<S>,
    Path(escalation_id): Path<String>,
    Json(req): Json<DenyEscalationRequest>,
) -> Result<StatusCode, (StatusCode, String)>
where
    S: ApiStateAccess,
{
    // Verify signature
    verify_wallet_signature(&req.wallet, &req.message, &req.signature).map_err(|e| {
        (StatusCode::UNAUTHORIZED, format!("Signature verification failed: {}", e))
    })?;
    
    // Check Redis availability
    let redis_conn = match state.redis().as_ref() {
        Some(conn) => conn,
        None => return Err((StatusCode::SERVICE_UNAVAILABLE, "Redis unavailable".to_string())),
    };
    
    // Verify nonce
    let mut redis = redis_conn.clone();
    let nonce_key = format!("nonce:{}:{}", req.wallet, req.nonce);
    
    let nonce_exists: bool = redis.exists(&nonce_key).await.map_err(|e| {
        log::error!("Failed to check nonce: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to verify nonce".to_string())
    })?;
    
    if !nonce_exists {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid or expired nonce".to_string(),
        ));
    }
    
    // Delete nonce
    redis.del::<_, ()>(&nonce_key).await.ok();
    
    // Get escalation
    let escalation_key = format!("escalation:pending:{}", escalation_id);
    let escalation_json: String = redis.get(&escalation_key).await.map_err(|_| {
        (StatusCode::NOT_FOUND, "Escalation not found".to_string())
    })?;
    
    let mut escalation: Escalation = serde_json::from_str(&escalation_json).map_err(|e| {
        log::error!("Failed to parse escalation: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Invalid escalation data".to_string())
    })?;
    
    // Update status
    escalation.status = EscalationStatus::Denied;
    let escalation_json = serde_json::to_string(&escalation).unwrap();
    redis.set::<_, _, ()>(&escalation_key, &escalation_json).await.ok();
    
    // Publish event
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let event = EscalationEvent::Denied {
        escalation_id: escalation_id.clone(),
        denied_by: req.wallet.clone(),
        denied_at: now,
        reason: req.reason.clone(),
    };
    
    let event_channel = format!("escalation:events:{}", escalation.approver_wallet);
    redis
        .publish::<_, _, ()>(&event_channel, serde_json::to_string(&event).unwrap())
        .await
        .ok();
    
    log::info!("❌ Escalation denied: {} by {}", escalation_id, req.wallet);
    
    Ok(StatusCode::NO_CONTENT)
}

/// Get escalation status
pub async fn get_status<S>(
    State(state): State<S>,
    Path(escalation_id): Path<String>,
) -> Result<Json<EscalationStatusResponse>, (StatusCode, String)>
where
    S: ApiStateAccess,
{
    let redis_conn = match state.redis().as_ref() {
        Some(conn) => conn,
        None => return Err((StatusCode::SERVICE_UNAVAILABLE, "Redis unavailable".to_string())),
    };
    let mut redis = redis_conn.clone();
    
    let escalation_key = format!("escalation:pending:{}", escalation_id);
    let escalation_json: String = redis.get(&escalation_key).await.map_err(|_| {
        (StatusCode::NOT_FOUND, "Escalation not found".to_string())
    })?;
    
    let escalation: Escalation = serde_json::from_str(&escalation_json).map_err(|e| {
        log::error!("Failed to parse escalation: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Invalid escalation data".to_string())
    })?;
    
    // TODO: Get rule_id and transaction_signature from escalation metadata
    
    Ok(Json(EscalationStatusResponse {
        status: escalation.status,
        rule_id: None,
        transaction_signature: None,
        fast_path: false,
    }))
}

/// List pending escalations for a wallet
pub async fn list_pending<S>(
    State(state): State<S>,
    Json(req): Json<ListPendingRequest>,
) -> Result<Json<Vec<Escalation>>, (StatusCode, String)>
where
    S: ApiStateAccess,
{
    // Verify timestamp
    verify_timestamp(req.timestamp).map_err(|e| {
        (StatusCode::BAD_REQUEST, format!("Invalid timestamp: {}", e))
    })?;
    
    // Verify signature
    verify_wallet_signature(&req.wallet, &req.message, &req.signature).map_err(|e| {
        (StatusCode::UNAUTHORIZED, format!("Signature verification failed: {}", e))
    })?;
    
    // Check Redis availability
    let redis_conn = match state.redis().as_ref() {
        Some(conn) => conn,
        None => return Err((StatusCode::SERVICE_UNAVAILABLE, "Redis unavailable".to_string())),
    };
    
    let mut redis = redis_conn.clone();
    
    // Get all escalations for this approver
    let approver_key = format!("escalation:pending:approver:{}", req.wallet);
    let escalation_ids: Vec<String> = redis.smembers(&approver_key).await.map_err(|e| {
        log::error!("Failed to list escalations: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to list escalations".to_string())
    })?;
    
    let mut escalations = Vec::new();
    
    for escalation_id in escalation_ids {
        let key = format!("escalation:pending:{}", escalation_id);
        if let Ok(escalation_json) = redis.get::<_, String>(&key).await {
            if let Ok(escalation) = serde_json::from_str::<Escalation>(&escalation_json) {
                if matches!(escalation.status, EscalationStatus::Pending) {
                    escalations.push(escalation);
                }
            }
        }
    }
    
    Ok(Json(escalations))
}

/// Forward transaction to Solana network
async fn forward_transaction_to_network(
    tx_bytes: &[u8],
    rpc_url: &str,
) -> Result<String, String> {
    // Deserialize transaction
    let _transaction: solana_sdk::transaction::Transaction = bincode::deserialize(tx_bytes)
        .map_err(|e| format!("Failed to deserialize transaction: {}", e))?;
    
    // Encode to base64
    let tx_base64 = B64.encode(tx_bytes);
    
    // Send via RPC
    let client = reqwest::Client::new();
    let response = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendTransaction",
            "params": [
                tx_base64,
                {
                    "encoding": "base64",
                    "skipPreflight": false,
                    "maxRetries": 3
                }
            ]
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;
    
    let result: serde_json::Value = response.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    
    if let Some(signature) = result["result"].as_str() {
        Ok(signature.to_string())
    } else if let Some(error) = result["error"].as_object() {
        Err(format!("RPC error: {:?}", error))
    } else {
        Err("Unknown error forwarding transaction".to_string())
    }
}
