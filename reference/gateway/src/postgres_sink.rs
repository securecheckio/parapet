use anyhow::Result;
use async_trait::async_trait;
use parapet_proxy::output::sink::{OutputSink, SinkMetadata};
use redis::AsyncCommands;
use sqlx::PgPool;

/// PostgreSQL sink for security events
pub struct PostgresSecuritySink {
    db: PgPool,
    redis: redis::Client,
}

impl PostgresSecuritySink {
    pub fn new(db: PgPool, redis: redis::Client) -> Self {
        Self { db, redis }
    }
}

#[async_trait]
impl OutputSink for PostgresSecuritySink {
    async fn write(&self, data: &[u8], metadata: &SinkMetadata) -> Result<()> {
        // Only process JSON content
        if !metadata.content_type.contains("json") {
            log::debug!(
                "Skipping non-JSON content from formatter {}",
                metadata.formatter_name
            );
            return Ok(());
        }

        // Parse as generic JSON to handle case-insensitive fields
        let json_value: serde_json::Value = serde_json::from_slice(data)?;

        // Extract fields manually to handle case issues
        let event_id = json_value["event_id"].as_str().unwrap_or("unknown");
        let proxy_event_type = json_value["event_type"].as_str().unwrap_or("unknown");
        let risk_level_str = json_value["risk_level"].as_str().unwrap_or("low");
        let risk_score = json_value["risk_score"].as_i64().unwrap_or(0);
        let user_id = json_value["user_id"].as_str();

        log::debug!(
            "Processing event {} (proxy_event_type: {}, risk_level: {}, risk_score: {})",
            event_id,
            proxy_event_type,
            risk_level_str,
            risk_score
        );

        // Log all transactions for testing
        let should_log = true;

        if !should_log {
            return Ok(());
        }

        // Determine event type based on proxy event_type
        let event_type = if proxy_event_type.contains("blocked") {
            "blocked"
        } else if risk_score >= 50 {
            "warned"
        } else {
            "allowed"
        };

        // Normalize severity to lowercase for database
        let severity = risk_level_str.to_lowercase();

        // Extract rule matches
        let rule_matches = json_value["rule_matches"].as_array();
        let threat_category = rule_matches
            .and_then(|matches| matches.first())
            .and_then(|m| m["rule_id"].as_str())
            .map(String::from);

        // Build description
        let issues = json_value["issues"].as_array();
        let description = if let Some(issues_arr) = issues {
            if !issues_arr.is_empty() {
                issues_arr
                    .iter()
                    .filter_map(|i| i.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            } else {
                json_value["summary"]
                    .as_str()
                    .unwrap_or("No description")
                    .to_string()
            }
        } else {
            json_value["summary"]
                .as_str()
                .unwrap_or("No description")
                .to_string()
        };

        // Extract rule info
        let rule_id = rule_matches
            .and_then(|matches| matches.first())
            .and_then(|m| m["rule_id"].as_str())
            .map(String::from);

        // Parse user_id as UUID if present
        let user_uuid = user_id.and_then(|id| uuid::Uuid::parse_str(id).ok());

        // Insert into database
        sqlx::query(
            "INSERT INTO security_events (
                user_id, event_type, severity, threat_category, 
                description, transaction_data, rule_id, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())",
        )
        .bind(user_uuid)
        .bind(event_type)
        .bind(&severity)
        .bind(threat_category.as_deref())
        .bind(&description)
        .bind(&json_value)
        .bind(rule_id.as_deref())
        .execute(&self.db)
        .await?;

        log::info!(
            "✅ Logged security event {} to PostgreSQL ({}, severity: {})",
            event_id,
            event_type,
            severity
        );

        // Publish new event notification to Redis for WebSocket clients
        if let Some(user_id_str) = user_id {
            match self.redis.get_multiplexed_async_connection().await {
                Ok(mut conn) => {
                    let channel = format!("user:{}:updates", user_id_str);
                    let update = serde_json::json!({
                        "type": "new_event",
                        "event": {
                            "id": event_id,
                            "outcome": event_type,
                            "severity": severity,
                            "summary": description.clone(),
                            "message": json_value["summary"].as_str().unwrap_or(&description),
                            "rule_id": rule_id.as_deref(),
                        }
                    });
                    match conn
                        .publish::<_, _, i32>(&channel, serde_json::to_string(&update).unwrap())
                        .await
                    {
                        Ok(num_subscribers) => {
                            log::info!(
                                "📤 Published event notification to {} ({} subscriber(s))",
                                channel,
                                num_subscribers
                            );
                        }
                        Err(e) => {
                            log::error!("❌ Failed to publish event notification: {}", e);
                        }
                    }
                }
                Err(e) => {
                    log::error!(
                        "❌ Failed to get Redis connection for event notification: {}",
                        e
                    );
                }
            }

            // Send push notification for blocked or critical events
            if event_type == "blocked" || severity == "critical" {
                let user_id = user_id_str.to_string();
                let title = if event_type == "blocked" {
                    "🚫 Transaction Blocked"
                } else {
                    "⚠️ Critical Security Alert"
                };
                let body = description.clone();
                let require_interaction = event_type == "blocked";

                tokio::spawn(async move {
                    // Call auth-api to send push notification
                    let auth_api_url = std::env::var("AUTH_API_URL")
                        .unwrap_or_else(|_| "http://localhost:3001".to_string());

                    let payload = serde_json::json!({
                        "user_id": user_id,
                        "title": title,
                        "body": body,
                        "require_interaction": require_interaction,
                    });

                    let mut req = reqwest::Client::new()
                        .post(format!("{}/internal/push/send", auth_api_url))
                        .json(&payload);
                    if let Ok(secret) = std::env::var("INTERNAL_API_SECRET") {
                        req = req.header("X-Internal-Secret", secret);
                    } else {
                        log::warn!(
                            "INTERNAL_API_SECRET is not set; /internal/push/send may be rejected by the auth API"
                        );
                    }
                    match req.send().await
                    {
                        Ok(resp) if resp.status().is_success() => {
                            log::info!("✅ Push notification sent for {}", event_type);
                        }
                        Ok(resp) => {
                            log::warn!("⚠️ Push notification failed: {}", resp.status());
                        }
                        Err(e) => {
                            log::error!("❌ Failed to send push notification: {}", e);
                        }
                    }
                });
            }
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "postgres"
    }
}
