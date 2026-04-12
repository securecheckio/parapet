use axum::{extract::State, http::StatusCode, Json};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use web_push::*;

use crate::state::PlatformState;

#[derive(Debug, Serialize, Deserialize)]
pub struct PushSubscription {
    pub endpoint: String,
    pub keys: PushKeys,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PushKeys {
    pub p256dh: String,
    pub auth: String,
}

#[derive(Debug, Deserialize)]
pub struct InternalPushRequest {
    pub user_id: String,
    pub title: String,
    pub body: String,
    pub require_interaction: bool,
}

pub async fn subscribe_push(
    State(state): State<PlatformState>,
    jar: CookieJar,
    Json(subscription): Json<PushSubscription>,
) -> Result<StatusCode, StatusCode> {
    // Get session from cookie
    let session_id = jar.get("session_id").ok_or(StatusCode::UNAUTHORIZED)?;
    let session = state.sessions.get_session(session_id.value()).await
        .map_err(|_| StatusCode::UNAUTHORIZED)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    let user_id = uuid::Uuid::parse_str(&session.user_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Store subscription in database
    match sqlx::query(
        "INSERT INTO push_subscriptions (user_id, endpoint, p256dh_key, auth_key) 
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (user_id, endpoint) DO UPDATE 
         SET p256dh_key = $3, auth_key = $4, created_at = NOW()"
    )
    .bind(&user_id)
    .bind(&subscription.endpoint)
    .bind(&subscription.keys.p256dh)
    .bind(&subscription.keys.auth)
    .execute(&state.db)
    .await
    {
        Ok(_) => {
            log::info!("✅ Stored push subscription for user {}", user_id);
            Ok(StatusCode::OK)
        }
        Err(e) => {
            log::error!("❌ Failed to store push subscription: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn internal_send_push(
    State(state): State<PlatformState>,
    Json(req): Json<InternalPushRequest>,
) -> Result<StatusCode, StatusCode> {
    log::info!("📨 Received push request for user {}: {}", req.user_id, req.title);
    match send_push_notification(
        &state.db,
        &req.user_id,
        &req.title,
        &req.body,
        req.require_interaction,
    ).await {
        Ok(_) => {
            log::info!("✅ Push notification sent successfully");
            Ok(StatusCode::OK)
        }
        Err(e) => {
            log::error!("❌ Push notification failed: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn send_push_notification(
    db: &PgPool,
    user_id: &str,
    title: &str,
    body: &str,
    require_interaction: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("🔔 Sending push notification to user {}", user_id);
    
    // Get VAPID PEM key from environment
    let vapid_private_key_pem = std::env::var("VAPID_PRIVATE_KEY_PEM")
        .map_err(|e| {
            log::error!("❌ VAPID_PRIVATE_KEY_PEM not set: {}", e);
            "VAPID_PRIVATE_KEY_PEM not set"
        })?;
    log::info!("✅ VAPID PEM key loaded");

    // Get user's push subscriptions
    let user_uuid = uuid::Uuid::parse_str(user_id).map_err(|e| {
        log::error!("❌ Invalid user_id format: {}", e);
        e
    })?;
    
    log::info!("🔍 Querying push subscriptions for user {}", user_uuid);
    let subscriptions = sqlx::query_as::<_, (String, String, String)>(
        "SELECT endpoint, p256dh_key, auth_key FROM push_subscriptions WHERE user_id = $1"
    )
    .bind(user_uuid)
    .fetch_all(db)
    .await
    .map_err(|e| {
        log::error!("❌ Database query failed: {}", e);
        e
    })?;

    log::info!("📊 Found {} push subscription(s)", subscriptions.len());
    
    if subscriptions.is_empty() {
        log::warn!("⚠️ No push subscriptions found for user {}", user_id);
        return Ok(());
    }

    // Prepare push message
    let payload = serde_json::json!({
        "title": title,
        "body": body,
        "requireInteraction": require_interaction,
        "tag": format!("securecheck-{}", chrono::Utc::now().timestamp()),
    });

    log::info!("📦 Payload: {}", payload);
    let payload_bytes = serde_json::to_vec(&payload).map_err(|e| {
        log::error!("❌ Failed to serialize payload: {}", e);
        e
    })?;

    // Create web push client
    log::info!("🔧 Creating WebPush client...");
    let client = WebPushClient::new().map_err(|e| {
        log::error!("❌ Failed to create WebPush client: {:?}", e);
        e
    })?;
    
    log::info!("📤 Sending to {} subscription(s)...", subscriptions.len());
    for (i, (endpoint, p256dh, auth)) in subscriptions.iter().enumerate() {
        log::info!("📨 [{}] Sending to endpoint: {}...", i + 1, &endpoint[..50]);
        
        // Build subscription info
        let subscription_info = SubscriptionInfo::new(endpoint, p256dh, auth);
        log::info!("✅ [{}] Subscription info created", i + 1);

        // Build VAPID signature for this specific subscription
        log::info!("🔐 [{}] Building VAPID signature...", i + 1);
        
        let sig_builder = VapidSignatureBuilder::from_pem(
            std::io::Cursor::new(vapid_private_key_pem.as_bytes()),
            &subscription_info,
        ).map_err(|e| {
            log::error!("❌ [{}] Failed to build VAPID signature: {:?}", i + 1, e);
            e
        })?;

        // Build message
        log::info!("📝 [{}] Building message...", i + 1);
        let mut builder = WebPushMessageBuilder::new(&subscription_info).map_err(|e| {
            log::error!("❌ [{}] Failed to create message builder: {:?}", i + 1, e);
            e
        })?;
        
        builder.set_payload(ContentEncoding::Aes128Gcm, &payload_bytes);
        builder.set_vapid_signature(sig_builder.build().map_err(|e| {
            log::error!("❌ [{}] Failed to build VAPID signature: {:?}", i + 1, e);
            e
        })?);

        let message = builder.build().map_err(|e| {
            log::error!("❌ [{}] Failed to build message: {:?}", i + 1, e);
            e
        })?;
        
        log::info!("🚀 [{}] Sending message...", i + 1);
        match client.send(message).await {
            Ok(response) => {
                log::info!("✅ [{}] Push notification sent successfully! Response: {:?}", i + 1, response);
                // Update last_used_at
                let _ = sqlx::query("UPDATE push_subscriptions SET last_used_at = NOW() WHERE endpoint = $1")
                    .bind(endpoint)
                    .execute(db)
                    .await;
            }
            Err(e) => {
                log::error!("❌ [{}] Failed to send push notification: {:?}", i + 1, e);
                // If subscription is invalid, remove it
                if matches!(e, WebPushError::EndpointNotValid | WebPushError::EndpointNotFound) {
                    log::warn!("🗑️ [{}] Removing invalid subscription", i + 1);
                    let _ = sqlx::query("DELETE FROM push_subscriptions WHERE endpoint = $1")
                        .bind(endpoint)
                        .execute(db)
                        .await;
                }
            }
        }
    }
    
    log::info!("🎉 Push notification process complete");


    Ok(())
}
