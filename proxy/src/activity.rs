use anyhow::Result;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Detect Solana network from RPC URL
pub fn detect_network(rpc_url: &str) -> String {
    let url_lower = rpc_url.to_lowercase();
    if url_lower.contains("devnet") {
        "devnet".to_string()
    } else if url_lower.contains("testnet") {
        "testnet".to_string()
    } else if url_lower.contains("mainnet") {
        "mainnet-beta".to_string()
    } else {
        // Default to mainnet if can't detect
        "mainnet-beta".to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEvent {
    pub activity_id: String,
    pub wallet: String,
    pub risk_score: u8,
    pub rule_id: String,
    pub rule_name: String,
    pub message: String,
    pub canonical_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    pub timestamp: u64,
    pub action: ActivityAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActivityAction {
    Allowed,
    Blocked,
    Flagged,
}

/// Publish activity event to Redis for dashboard consumption
pub async fn publish_activity_event(
    wallet: &str,
    risk_score: u8,
    rule_id: &str,
    rule_name: &str,
    message: &str,
    canonical_hash: &str,
    action: ActivityAction,
    redis_url: &str,
    max_events: usize,
    ttl: u64,
) -> Result<()> {
    publish_activity_event_with_details(
        wallet,
        risk_score,
        rule_id,
        rule_name,
        message,
        canonical_hash,
        None,
        None,
        action,
        redis_url,
        max_events,
        ttl,
    )
    .await
}

/// Publish activity event with optional signature and network
pub async fn publish_activity_event_with_details(
    wallet: &str,
    risk_score: u8,
    rule_id: &str,
    rule_name: &str,
    message: &str,
    canonical_hash: &str,
    signature: Option<String>,
    network: Option<String>,
    action: ActivityAction,
    redis_url: &str,
    max_events: usize,
    ttl: u64,
) -> Result<()> {
    let activity_id = format!("act_{}", Uuid::new_v4().simple());
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    let event = ActivityEvent {
        activity_id: activity_id.clone(),
        wallet: wallet.to_string(),
        risk_score,
        rule_id: rule_id.to_string(),
        rule_name: rule_name.to_string(),
        message: message.to_string(),
        canonical_hash: canonical_hash.to_string(),
        signature,
        network,
        timestamp: now,
        action,
    };

    let client = redis::Client::open(redis_url)?;
    let mut conn = client.get_multiplexed_async_connection().await?;

    let event_json = serde_json::to_string(&event)?;

    // Store in wallet-specific list (newest first)
    let list_key = format!("activity:wallet:{}", wallet);
    conn.lpush::<_, _, ()>(&list_key, &event_json).await?;
    
    // Trim to max events
    conn.ltrim::<_, ()>(&list_key, 0, (max_events - 1) as isize).await?;
    
    // Set TTL
    conn.expire::<_, ()>(&list_key, ttl as i64).await?;

    // Publish to WebSocket channel
    let channel = format!("activity:events:{}", wallet);
    conn.publish::<_, _, ()>(&channel, &event_json).await?;

    log::debug!(
        "📊 Activity event published: {} (wallet: {}, risk: {})",
        activity_id,
        wallet,
        risk_score
    );

    Ok(())
}
