use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query,
        State,
    },
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use futures::{SinkExt, StreamExt};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DashboardUpdate {
    #[serde(rename = "stats_update")]
    StatsUpdate {
        credits_balance: i64,
        credits_used_this_month: i64,
        credits_remaining: i64,
    },
    #[serde(rename = "new_event")]
    NewEvent {
        event_id: String,
        event_type: String,
        severity: String,
    },
    #[serde(rename = "ping")]
    Ping,
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    jar: CookieJar,
) -> impl IntoResponse {
    log::debug!("🔌 WebSocket upgrade request received");
    
    // Get session ID from HTTP-only cookie
    let session_id = jar.get("session_id").map(|c| c.value());

    if let Some(sid) = session_id {
        log::debug!("🔑 Found session cookie: {}...", &sid[..std::cmp::min(8, sid.len())]);
        if let Ok(Some(session)) = state.sessions.get_session(sid).await {
            let user_id = session.user_id.clone();
            log::info!("✅ WebSocket authenticated for user {}", user_id);
            return ws.on_upgrade(move |socket| handle_socket(socket, state, user_id));
        } else {
            log::warn!("🚫 Session not found in Redis");
        }
    } else {
        log::warn!("🚫 No session_id cookie in request");
    }

    log::warn!("🚫 WebSocket connection rejected: no valid session");
    ws.on_upgrade(|mut socket| async move {
        let _ = socket.send(Message::Close(None)).await;
    })
}

async fn handle_socket(socket: WebSocket, state: AppState, user_id: String) {
    let (mut sender, mut receiver) = socket.split();

    log::info!("✅ WebSocket connected for user {}", user_id);

    // Create Redis pub/sub connection
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let redis_client = match redis::Client::open(redis_url.as_str()) {
        Ok(client) => client,
        Err(e) => {
            log::error!("❌ Failed to create Redis client for WebSocket: {}", e);
            return;
        }
    };

    let conn = match redis_client.get_async_connection().await {
        Ok(conn) => conn,
        Err(e) => {
            log::error!("❌ Failed to get Redis connection: {}", e);
            return;
        }
    };

    let mut pubsub_conn = conn.into_pubsub();

    // Subscribe to user-specific channel
    let channel = format!("user:{}:updates", user_id);
    if let Err(e) = pubsub_conn.subscribe(&channel).await {
        log::error!("❌ Failed to subscribe to {}: {}", channel, e);
        return;
    }

    log::info!("📡 Subscribed to Redis channel: {}", channel);

    let mut pubsub_stream = pubsub_conn.on_message();

    // Send initial ping
    if sender.send(Message::Text(
        serde_json::to_string(&DashboardUpdate::Ping).unwrap()
    )).await.is_err() {
        return;
    }

    // Handle messages from both WebSocket client and Redis pub/sub
    loop {
        tokio::select! {
            // Messages from Redis pub/sub
            Some(msg) = pubsub_stream.next() => {
                let payload: String = match msg.get_payload::<String>() {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("❌ Failed to get Redis payload: {}", e);
                        continue;
                    }
                };

                log::debug!("📨 Received update from Redis: {}", payload);

                // Forward to WebSocket client
                if sender.send(Message::Text(payload)).await.is_err() {
                    log::info!("🔌 WebSocket connection closed (send failed)");
                    break;
                }
            }

            // Messages from WebSocket client (e.g., ping/pong)
            Some(msg) = receiver.next() => {
                match msg {
                    Ok(Message::Text(text)) => {
                        log::debug!("📨 Received from client: {}", text);
                        // Echo back or handle client messages if needed
                    }
                    Ok(Message::Close(_)) => {
                        log::info!("🔌 WebSocket closed by client");
                        break;
                    }
                    Ok(Message::Ping(data)) => {
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        log::error!("❌ WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }

            else => break
        }
    }

    log::info!("🔌 WebSocket disconnected for user {}", user_id);
}

pub async fn publish_stats_update(
    redis_client: &redis::Client,
    user_id: &str,
    credits_balance: i64,
    credits_used_this_month: i64,
) -> Result<(), redis::RedisError> {
    let mut conn = redis_client.get_multiplexed_async_connection().await?;
    
    let update = DashboardUpdate::StatsUpdate {
        credits_balance,
        credits_used_this_month,
        credits_remaining: credits_balance - credits_used_this_month,
    };

    let channel = format!("user:{}:updates", user_id);
    let payload = serde_json::to_string(&update).unwrap();
    
    conn.publish::<_, _, ()>(&channel, &payload).await?;
    
    log::debug!("📤 Published stats update to {}", channel);
    Ok(())
}

pub async fn publish_new_event(
    redis_client: &redis::Client,
    user_id: &str,
    event_id: &str,
    event_type: &str,
    severity: &str,
) -> Result<(), redis::RedisError> {
    let mut conn = redis_client.get_multiplexed_async_connection().await?;
    
    let update = DashboardUpdate::NewEvent {
        event_id: event_id.to_string(),
        event_type: event_type.to_string(),
        severity: severity.to_string(),
    };

    let channel = format!("user:{}:updates", user_id);
    let payload = serde_json::to_string(&update).unwrap();
    
    conn.publish::<_, _, ()>(&channel, &payload).await?;
    
    log::debug!("📤 Published new event to {}", channel);
    Ok(())
}
