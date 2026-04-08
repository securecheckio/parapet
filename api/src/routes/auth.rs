use crate::types::{NonceRequest, NonceResponse};
use crate::ApiStateAccess;
use axum::{extract::State, http::StatusCode, Json};
use redis::AsyncCommands;
use std::time::{SystemTime, UNIX_EPOCH};

/// Generate a nonce for signature replay prevention
pub async fn generate_nonce<S>(
    State(state): State<S>,
    Json(req): Json<NonceRequest>,
) -> Result<Json<NonceResponse>, (StatusCode, String)>
where
    S: ApiStateAccess,
{
    // Generate random nonce
    let nonce = uuid::Uuid::new_v4().to_string();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let expires_at = now + state.config().nonce_ttl_seconds; // Configurable TTL

    // Check Redis availability and get connection
    let redis_conn = match state.redis().as_ref() {
        Some(conn) => conn,
        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                "Redis unavailable".to_string(),
            ))
        }
    };

    // Store nonce in Redis with TTL
    let key = format!("nonce:{}:{}", req.wallet, nonce);

    let mut redis = redis_conn.clone();
    redis
        .set_ex::<_, _, ()>(&key, "1", state.config().nonce_ttl_seconds)
        .await
        .map_err(|e| {
            log::error!("Failed to store nonce: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to generate nonce".to_string(),
            )
        })?;

    Ok(Json(NonceResponse { nonce, expires_at }))
}
