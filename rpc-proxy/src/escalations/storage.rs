use super::types::*;
use anyhow::Result;
use redis::AsyncCommands;

/// Storage for escalations in Redis
pub struct EscalationStorage {
    redis_url: String,
}

impl EscalationStorage {
    pub fn new(redis_url: String) -> Self {
        Self { redis_url }
    }

    /// Get escalation by ID
    pub async fn get_escalation(&self, escalation_id: &str) -> Result<Option<Escalation>> {
        let client = redis::Client::open(self.redis_url.as_str())?;
        let mut conn = client.get_multiplexed_async_connection().await?;

        let key = format!("escalation:pending:{}", escalation_id);
        let escalation_json: Option<String> = conn.get(&key).await?;

        if let Some(json) = escalation_json {
            let escalation: Escalation = serde_json::from_str(&json)?;
            Ok(Some(escalation))
        } else {
            Ok(None)
        }
    }

    /// Update escalation status
    pub async fn update_status(&self, escalation_id: &str, status: EscalationStatus) -> Result<()> {
        let client = redis::Client::open(self.redis_url.as_str())?;
        let mut conn = client.get_multiplexed_async_connection().await?;

        let key = format!("escalation:pending:{}", escalation_id);

        if let Some(mut escalation) = self.get_escalation(escalation_id).await? {
            escalation.status = status;
            let escalation_json = serde_json::to_string(&escalation)?;
            conn.set::<_, _, ()>(&key, &escalation_json).await?;
        }

        Ok(())
    }

    /// List pending escalations for an approver
    pub async fn list_pending(&self, approver_wallet: &str) -> Result<Vec<Escalation>> {
        let client = redis::Client::open(self.redis_url.as_str())?;
        let mut conn = client.get_multiplexed_async_connection().await?;

        let approver_key = format!("escalation:pending:approver:{}", approver_wallet);
        let escalation_ids: Vec<String> = conn.smembers(&approver_key).await?;

        let mut escalations = Vec::new();

        for escalation_id in escalation_ids {
            if let Some(escalation) = self.get_escalation(&escalation_id).await? {
                if matches!(escalation.status, EscalationStatus::Pending) {
                    escalations.push(escalation);
                }
            }
        }

        Ok(escalations)
    }
}
