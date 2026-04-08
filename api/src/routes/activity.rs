use crate::auth::{verify_timestamp, verify_wallet_signature};
use crate::types::ActivityEvent;
use crate::ApiStateAccess;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ActivityQuery {
    wallet: String,
    message: String,
    signature: String,
    timestamp: u64,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    50
}

/// Get recent activity for a wallet
pub async fn get_recent_activity<S>(
    Query(query): Query<ActivityQuery>,
    State(state): State<S>,
) -> Result<Json<Vec<ActivityEvent>>, (StatusCode, String)>
where
    S: ApiStateAccess,
{
    // Verify timestamp
    verify_timestamp(query.timestamp).map_err(|e| {
        (
            StatusCode::UNAUTHORIZED,
            format!("Invalid timestamp: {}", e),
        )
    })?;

    // Verify message format
    let expected_message = format!("parapet:activity:{}:{}", query.wallet, query.timestamp);
    if query.message != expected_message {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid message format".to_string(),
        ));
    }

    // Verify signature
    verify_wallet_signature(&query.wallet, &query.message, &query.signature).map_err(|e| {
        (
            StatusCode::UNAUTHORIZED,
            format!("Invalid signature: {}", e),
        )
    })?;

    // Fetch recent activity from Redis
    let client = redis::Client::open(state.config().redis_url.as_str())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Get activity list for wallet (stored as JSON list in Redis)
    let key = format!("activity:wallet:{}", query.wallet);
    let activity_json: Vec<String> = redis::cmd("LRANGE")
        .arg(&key)
        .arg(0)
        .arg(query.limit as isize - 1)
        .query_async(&mut conn)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let activities: Vec<ActivityEvent> = activity_json
        .iter()
        .filter_map(|json| serde_json::from_str(json).ok())
        .collect();

    log::info!(
        "📊 Activity request from {} (found {} events)",
        query.wallet,
        activities.len()
    );

    Ok(Json(activities))
}
