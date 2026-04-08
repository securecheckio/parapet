pub mod decoder;
pub mod notifier;
pub mod policy;
pub mod storage;
pub mod types;

pub use decoder::{DecodedInstruction, DecoderRegistry, ProgramDecoder};
pub use notifier::{EscalationNotifier, NotifierRegistry};
pub use policy::ConsentPolicyConfig;
pub use storage::EscalationStorage;
pub use types::*;

use anyhow::Result;
use redis::AsyncCommands;
use solana_sdk::transaction::VersionedTransaction;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Create an escalation for a blocked transaction
pub async fn create_escalation(
    transaction: &VersionedTransaction,
    canonical_hash: String,
    decoded_instructions: Vec<decoder::DecodedInstruction>,
    rule_id: String,
    rule_name: String,
    rule_message: String,
    risk_score: u8,
    requester_wallet: String,
    approver_wallet: String,
    redis_url: &str,
) -> Result<Escalation> {
    let escalation_id = format!("esc_{}", Uuid::new_v4());

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    let warnings = vec![format!("{} - {}", rule_name, rule_message)];

    let suggested_rules = vec![SuggestedRule {
        rule_type: "whitelist".to_string(),
        name: format!("Allow {} (from escalation)", canonical_hash),
        description: format!(
            "Whitelists transactions matching canonical hash {} based on manual approval",
            canonical_hash
        ),
        conditions: serde_json::json!({
            "canonical_transaction_hash": canonical_hash,
        }),
    }];

    let escalation = Escalation {
        escalation_id: escalation_id.clone(),
        canonical_hash,
        requester_wallet,
        approver_wallet: approver_wallet.clone(),
        risk_score,
        warnings,
        decoded_instructions,
        suggested_rules,
        status: EscalationStatus::Pending,
        created_at: now,
        expires_at: now + 300, // 5 minutes
    };

    // Store escalation in Redis
    let client = redis::Client::open(redis_url)?;
    let mut conn = client.get_multiplexed_async_connection().await?;

    // Store escalation data
    let escalation_key = format!("escalation:pending:{}", escalation_id);
    let escalation_json = serde_json::to_string(&escalation)?;
    conn.set_ex::<_, _, ()>(&escalation_key, &escalation_json, 300)
        .await?;

    // Store transaction bytes for fast-path (50 second TTL)
    let tx_bytes = bincode::serialize(transaction)?;
    let tx_key = format!("pending_tx:{}", escalation_id);
    conn.set_ex::<_, _, ()>(&tx_key, &tx_bytes, 50).await?;

    // Add to approver's pending set
    let approver_key = format!("escalation:pending:approver:{}", approver_wallet);
    conn.sadd::<_, _, ()>(&approver_key, &escalation_id).await?;
    conn.expire::<_, ()>(&approver_key, 300).await?;

    log::info!(
        "📋 Escalation created: {} (rule: {} / {}) for wallet {}",
        escalation_id,
        rule_id,
        rule_name,
        approver_wallet
    );

    Ok(escalation)
}

/// Publish escalation event to WebSocket subscribers
pub async fn publish_escalation_event(escalation: &Escalation, redis_url: &str) -> Result<()> {
    let client = redis::Client::open(redis_url)?;
    let mut conn = client.get_multiplexed_async_connection().await?;

    let event = EscalationEvent::Created {
        escalation: escalation.clone(),
    };

    let event_json = serde_json::to_string(&event)?;
    let channel = format!("escalation:events:{}", escalation.approver_wallet);

    conn.publish::<_, _, ()>(&channel, &event_json).await?;

    log::debug!("📡 Published escalation event to channel: {}", channel);

    Ok(())
}
