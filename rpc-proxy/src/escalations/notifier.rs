use anyhow::Result;

/// Trait for escalation notifiers (pluggable notification system)
#[async_trait::async_trait]
pub trait EscalationNotifier: Send + Sync {
    fn name(&self) -> &str;
    async fn send_notification(&self, notif: &EscalationNotification) -> Result<()>;
}

/// Notification data for escalations
pub struct EscalationNotification {
    pub escalation_id: String,
    pub canonical_hash: String,
    pub risk_score: u32,
    pub warnings: Vec<String>,
    pub requester_wallet: String,
    pub approver_wallet: String,
    pub dashboard_url: String,
    pub timestamp: u64,
}

/// Registry for notification providers
pub struct NotifierRegistry {
    notifiers: Vec<Box<dyn EscalationNotifier>>,
}

impl NotifierRegistry {
    pub fn new() -> Self {
        Self {
            notifiers: Vec::new(),
        }
    }

    /// Register a notification provider
    pub fn register(&mut self, notifier: Box<dyn EscalationNotifier>) {
        log::info!("📬 Registered notification provider: {}", notifier.name());
        self.notifiers.push(notifier);
    }

    /// Send notification via all registered providers
    pub async fn notify_all(&self, notification: &EscalationNotification) -> Result<()> {
        if self.notifiers.is_empty() {
            log::debug!("No external notification providers configured");
            return Ok(());
        }

        let futs: Vec<_> = self
            .notifiers
            .iter()
            .map(|n| n.send_notification(notification))
            .collect();

        let results: Vec<Result<()>> = futures::future::join_all(futs).await;

        // Log failures but don't fail the operation
        for (idx, result) in results.iter().enumerate() {
            if let Err(e) = result {
                log::warn!("Notification provider {} failed: {}", idx, e);
            }
        }

        Ok(())
    }
}

impl Default for NotifierRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Example webhook notifier (reference implementation)
#[cfg(feature = "example_notifiers")]
pub struct WebhookNotifier {
    url: String,
}

#[cfg(feature = "example_notifiers")]
impl WebhookNotifier {
    pub fn new(url: String) -> Self {
        Self { url }
    }
}

#[cfg(feature = "example_notifiers")]
#[async_trait::async_trait]
impl EscalationNotifier for WebhookNotifier {
    fn name(&self) -> &str {
        "webhook"
    }

    async fn send_notification(&self, notif: &EscalationNotification) -> Result<()> {
        let client = reqwest::Client::new();
        client
            .post(&self.url)
            .json(&serde_json::json!({
                "escalation_id": notif.escalation_id,
                "risk_score": notif.risk_score,
                "warnings": notif.warnings,
                "dashboard_url": notif.dashboard_url,
            }))
            .send()
            .await?;
        Ok(())
    }
}
