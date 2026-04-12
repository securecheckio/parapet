use crate::auth::{is_wallet_authorized, verify_timestamp, verify_wallet_signature};
use crate::types::*;
use crate::ApiStateAccess;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use redis::AsyncCommands;
use std::time::{SystemTime, UNIX_EPOCH};

/// Create a new dynamic rule
pub async fn create_rule<S>(
    State(state): State<S>,
    Json(req): Json<CreateRuleRequest>,
) -> Result<Json<CreateRuleResponse>, (StatusCode, String)>
where
    S: ApiStateAccess,
{
    // Verify timestamp
    verify_timestamp(req.timestamp).map_err(|e| {
        (StatusCode::BAD_REQUEST, format!("Invalid timestamp: {}", e))
    })?;
    
    // Verify signature
    let message = format!(
        "parapet:create_rule:{}:{}",
        req.signed_rule, req.timestamp
    );
    
    verify_wallet_signature(&req.wallet, &message, &req.signature).map_err(|e| {
        (StatusCode::UNAUTHORIZED, format!("Signature verification failed: {}", e))
    })?;
    
    // Check authorization
    if !is_wallet_authorized(&req.wallet, &state.config().authorized_wallets) {
        return Err((
            StatusCode::FORBIDDEN,
            format!("Wallet {} is not authorized to create rules", req.wallet),
        ));
    }
    
    // Check Redis availability
    let redis_conn = match state.redis().as_ref() {
        Some(conn) => conn,
        None => return Err((StatusCode::SERVICE_UNAVAILABLE, "Redis unavailable".to_string())),
    };
    
    // Store rule in Redis
    let rule_key = format!("dynamic_rules:{}", req.rule.id);
    let rule_json = serde_json::to_string(&req.rule).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize rule: {}", e))
    })?;
    
    let mut redis = redis_conn.clone();
    
    // Set expiry if specified
    if let Some(expires_at) = req.rule.expires_at {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let ttl = expires_at.saturating_sub(now);
        
        redis
            .set_ex::<_, _, ()>(&rule_key, &rule_json, ttl as u64)
            .await
            .map_err(|e| {
                log::error!("Failed to store rule in Redis: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Failed to store rule".to_string())
            })?;
    } else {
        redis
            .set::<_, _, ()>(&rule_key, &rule_json)
            .await
            .map_err(|e| {
                log::error!("Failed to store rule in Redis: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Failed to store rule".to_string())
            })?;
    }
    
    // Publish event for cache invalidation across instances
    redis
        .publish::<_, _, ()>("dynamic_rules:updated", &req.rule.id)
        .await
        .ok();
    
    log::info!("✅ Rule created: {} by {}", req.rule.id, req.wallet);
    
    Ok(Json(CreateRuleResponse {
        rule_id: req.rule.id.clone(),
        message: "Rule created successfully".to_string(),
    }))
}

/// List all rules for a wallet
pub async fn list_rules<S>(
    State(state): State<S>,
    Json(req): Json<ListRulesRequest>,
) -> Result<Json<ListRulesResponse>, (StatusCode, String)>
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
    
    // Check authorization
    if !is_wallet_authorized(&req.wallet, &state.config().authorized_wallets) {
        return Err((
            StatusCode::FORBIDDEN,
            "Wallet not authorized".to_string(),
        ));
    }
    
    // Check Redis availability
    let redis_conn = match state.redis().as_ref() {
        Some(conn) => conn,
        None => return Err((StatusCode::SERVICE_UNAVAILABLE, "Redis unavailable".to_string())),
    };
    
    // Get all dynamic rules from Redis
    let mut redis = redis_conn.clone();
    let keys: Vec<String> = redis
        .keys("dynamic_rules:*")
        .await
        .map_err(|e| {
            log::error!("Failed to list rules: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to list rules".to_string())
        })?;
    
    let mut rules = Vec::new();
    
    for key in keys {
        let rule_json: String = redis.get(&key).await.map_err(|e| {
            log::error!("Failed to get rule {}: {}", key, e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to retrieve rule".to_string())
        })?;
        
        if let Ok(rule) = serde_json::from_str::<DynamicRuleDefinition>(&rule_json) {
            rules.push(rule);
        }
    }
    
    Ok(Json(ListRulesResponse { rules }))
}

/// Delete a rule
pub async fn delete_rule<S>(
    State(state): State<S>,
    Path(rule_id): Path<String>,
    Json(req): Json<DeleteRuleRequest>,
) -> Result<StatusCode, (StatusCode, String)>
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
    
    // Check authorization
    if !is_wallet_authorized(&req.wallet, &state.config().authorized_wallets) {
        return Err((
            StatusCode::FORBIDDEN,
            "Wallet not authorized".to_string(),
        ));
    }
    
    // Check Redis availability
    let redis_conn = match state.redis().as_ref() {
        Some(conn) => conn,
        None => return Err((StatusCode::SERVICE_UNAVAILABLE, "Redis unavailable".to_string())),
    };
    
    // Delete rule from Redis
    let rule_key = format!("dynamic_rules:{}", rule_id);
    let mut redis = redis_conn.clone();
    
    redis
        .del::<_, ()>(&rule_key)
        .await
        .map_err(|e| {
            log::error!("Failed to delete rule: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete rule".to_string())
        })?;
    
    // Publish event
    redis
        .publish::<_, _, ()>("dynamic_rules:deleted", &rule_id)
        .await
        .ok();
    
    log::info!("🗑️  Rule deleted: {} by {}", rule_id, req.wallet);
    
    Ok(StatusCode::NO_CONTENT)
}

/// Export rules in rules.json format
pub async fn export_rules<S>(
    State(state): State<S>,
    Json(req): Json<ExportRulesRequest>,
) -> Result<Json<Vec<DynamicRuleDefinition>>, (StatusCode, String)>
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
    
    // Check authorization
    if !is_wallet_authorized(&req.wallet, &state.config().authorized_wallets) {
        return Err((
            StatusCode::FORBIDDEN,
            "Wallet not authorized".to_string(),
        ));
    }
    
    // Check Redis availability
    let redis_conn = match state.redis().as_ref() {
        Some(conn) => conn,
        None => return Err((StatusCode::SERVICE_UNAVAILABLE, "Redis unavailable".to_string())),
    };
    
    // Get all rules
    let mut redis = redis_conn.clone();
    let keys: Vec<String> = redis
        .keys("dynamic_rules:*")
        .await
        .map_err(|e| {
            log::error!("Failed to list rules: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to list rules".to_string())
        })?;
    
    let mut rules = Vec::new();
    
    for key in keys {
        let rule_json: String = redis.get(&key).await.map_err(|e| {
            log::error!("Failed to get rule {}: {}", key, e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to retrieve rule".to_string())
        })?;
        
        if let Ok(rule) = serde_json::from_str::<DynamicRuleDefinition>(&rule_json) {
            rules.push(rule);
        }
    }
    
    Ok(Json(rules))
}

/// Import rules
pub async fn import_rules<S>(
    State(state): State<S>,
    Json(req): Json<ImportRulesRequest>,
) -> Result<Json<CreateRuleResponse>, (StatusCode, String)>
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
    
    // Check authorization
    if !is_wallet_authorized(&req.wallet, &state.config().authorized_wallets) {
        return Err((
            StatusCode::FORBIDDEN,
            "Wallet not authorized".to_string(),
        ));
    }
    
    // Check Redis availability
    let redis_conn = match state.redis().as_ref() {
        Some(conn) => conn,
        None => return Err((StatusCode::SERVICE_UNAVAILABLE, "Redis unavailable".to_string())),
    };
    
    let mut redis = redis_conn.clone();
    let mut imported_count = 0;
    
    for rule in req.rules {
        let rule_key = format!("dynamic_rules:{}", rule.id);
        let rule_json = serde_json::to_string(&rule).map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize rule: {}", e))
        })?;
        
        if let Some(expires_at) = rule.expires_at {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let ttl = expires_at.saturating_sub(now);
            
            redis
                .set_ex::<_, _, ()>(&rule_key, &rule_json, ttl as u64)
                .await
                .ok();
        } else {
            redis
                .set::<_, _, ()>(&rule_key, &rule_json)
                .await
                .ok();
        }
        
        imported_count += 1;
    }
    
    // Publish event
    redis
        .publish::<_, _, ()>("dynamic_rules:imported", imported_count.to_string())
        .await
        .ok();
    
    log::info!("📥 Imported {} rules by {}", imported_count, req.wallet);
    
    Ok(Json(CreateRuleResponse {
        rule_id: "bulk".to_string(),
        message: format!("Imported {} rules successfully", imported_count),
    }))
}
