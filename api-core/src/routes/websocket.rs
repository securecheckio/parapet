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

    // Wait for subscription message
    let wallet = match receiver.next().await {
        Some(Ok(Message::Text(text))) => {
            // Parse subscription request
            if let Ok(sub) = serde_json::from_str::<SubscribeRequest>(&text) {
                sub.wallet
            } else {
                log::warn!("Invalid subscription request");
                return;
            }
        }
        _ => {
            log::warn!("Expected text message for subscription");
            return;
        }
    };

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
}
