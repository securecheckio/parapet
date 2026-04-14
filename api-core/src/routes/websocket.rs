use crate::auth::{verify_timestamp, verify_wallet_signature};
use crate::ApiStateAccess;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};

/// WebSocket endpoint for escalation notifications
pub async fn escalation_websocket_handler<S>(
    ws: WebSocketUpgrade,
    State(state): State<S>,
) -> Response
where
    S: ApiStateAccess,
{
    ws.on_upgrade(|socket| handle_escalation_websocket(socket, state))
}

async fn handle_escalation_websocket<S>(socket: WebSocket, state: S)
where
    S: ApiStateAccess,
{
    let (mut sender, mut receiver) = socket.split();

    // Wait for signed subscription message
    let sub = match receiver.next().await {
        Some(Ok(Message::Text(text))) => match serde_json::from_str::<SubscribeRequest>(&text) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("Invalid subscription request JSON: {}", e);
                return;
            }
        },
        _ => {
            log::warn!("Expected text message for subscription");
            return;
        }
    };

    if let Err(e) = verify_subscribe_request(&sub) {
        log::warn!("WebSocket subscribe rejected: {}", e);
        let (code, detail) = if let Some(rest) = e.strip_prefix("invalid_timestamp:") {
            ("invalid_timestamp", rest.to_string())
        } else if e == "invalid_message" {
            ("invalid_message", "message must match challenge".to_string())
        } else if let Some(rest) = e.strip_prefix("unauthorized:") {
            ("unauthorized", rest.to_string())
        } else {
            ("unauthorized", e)
        };
        let _ = sender
            .send(Message::Text(
                serde_json::json!({"error": code, "detail": detail}).to_string(),
            ))
            .await;
        return;
    }

    let wallet = sub.wallet;

    log::info!("📡 Dashboard connected for escalations: {}", wallet);

    // Subscribe to Redis pubsub channel
    let channel = format!("escalation:events:{}", wallet);

    // Create Redis connection for pubsub
    let client = redis::Client::open(state.config().redis_url.as_str()).unwrap();
    let mut pubsub_conn = match client.get_async_pubsub().await {
        Ok(conn) => conn,
        Err(e) => {
            log::error!("Failed to create pubsub connection: {}", e);
            return;
        }
    };

    if let Err(e) = pubsub_conn.subscribe(&channel).await {
        log::error!("Failed to subscribe to channel: {}", e);
        return;
    }

    log::info!("✅ Subscribed to channel: {}", channel);

    // Forward messages from Redis to WebSocket
    let mut message_stream = pubsub_conn.on_message();

    while let Some(msg) = message_stream.next().await {
        let payload: String = match msg.get_payload() {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to get payload: {}", e);
                continue;
            }
        };

        // Forward to WebSocket client
        if let Err(e) = sender.send(Message::Text(payload)).await {
            log::error!("Failed to send WebSocket message: {}", e);
            break;
        }
    }

    log::info!("📴 Dashboard disconnected: {}", wallet);
}

#[derive(serde::Deserialize)]
struct SubscribeRequest {
    wallet: String,
    message: String,
    signature: String,
    timestamp: u64,
}

fn verify_subscribe_request(sub: &SubscribeRequest) -> Result<(), String> {
    verify_timestamp(sub.timestamp).map_err(|e| format!("invalid_timestamp:{e}"))?;
    let expected_message =
        format!("parapet:ws:escalations:subscribe:{}:{}", sub.wallet, sub.timestamp);
    if sub.message != expected_message {
        return Err("invalid_message".to_string());
    }
    verify_wallet_signature(&sub.wallet, &sub.message, &sub.signature)
        .map_err(|e| format!("unauthorized:{e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer as _;
    use sha2::Digest as _;

    fn signed_subscribe() -> SubscribeRequest {
        let seed = sha2::Sha256::digest(b"parapet-ws-test-wallet");
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&seed[..32]);
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&bytes);
        let wallet = bs58::encode(signing_key.verifying_key().to_bytes()).into_string();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let message = format!("parapet:ws:escalations:subscribe:{}:{}", wallet, timestamp);
        let sig = signing_key.sign(message.as_bytes());
        let signature = bs58::encode(sig.to_bytes()).into_string();
        SubscribeRequest {
            wallet,
            message,
            signature,
            timestamp,
        }
    }

    #[test]
    fn websocket_subscribe_auth_accepts_valid_signature() {
        let sub = signed_subscribe();
        assert!(verify_subscribe_request(&sub).is_ok());
    }

    #[test]
    fn websocket_subscribe_auth_rejects_bad_message() {
        let mut sub = signed_subscribe();
        sub.message = "bad".to_string();
        assert!(verify_subscribe_request(&sub).is_err());
    }
}
