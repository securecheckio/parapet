use crate::state::AppState;
use crate::types::{NonceRequest, NonceResponse};
use axum::{extract::State, http::StatusCode, Json};
use redis::AsyncCommands;
use std::time::{SystemTime, UNIX_EPOCH};

/// Generate a nonce for signature replay prevention
pub async fn generate_nonce(
    State(state): State<AppState>,
    Json(req): Json<NonceRequest>,
) -> Result<Json<NonceResponse>, (StatusCode, String)> {
    // Generate random nonce
    let nonce = uuid::Uuid::new_v4().to_string();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let expires_at = now + 60; // 60 second expiry

    // Store nonce in Redis with TTL
    let key = format!("nonce:{}:{}", req.wallet, nonce);

    let mut redis = state.redis.as_ref().clone();
    redis.set_ex::<_, _, ()>(&key, "1", 60).await.map_err(|e| {
        log::error!("Failed to store nonce: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to generate nonce".to_string(),
        )
    })?;

    Ok(Json(NonceResponse { nonce, expires_at }))
}
