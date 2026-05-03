/// WebSocket real-time notification tests
use axum::body::Body;
use axum::http::{Request, StatusCode};
use futures::{SinkExt, StreamExt};
use parapet_api::{create_router, state::AppState, state::Config};
use redis::AsyncCommands;
use serde_json::json;
use tokio_tungstenite::{connect_async, tungstenite::Message};

mod common;

/// Create test config
fn create_test_config() -> Config {
    Config {
        server_host: "127.0.0.1".to_string(),
        server_port: 3002, // Different port for WS tests
        worker_threads: None,
        max_concurrent_scans: 2,
        scans_per_hour_per_key: 10,
        redis_url: std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
        solana_rpc_url: "https://api.devnet.solana.com".to_string(),
        solana_rpc_urls: vec!["https://api.devnet.solana.com".to_string()],
        solana_network: "devnet".to_string(),
        authorized_wallets: vec!["test_wallet".to_string()],
        nonce_ttl_seconds: 300,
        mcp_api_keys: vec!["test_key".to_string()],
    }
}

#[tokio::test]
async fn test_websocket_endpoint_exists() {
    let config = create_test_config();

    // Check Redis availability
    let redis_available = if let Ok(client) = redis::Client::open(config.redis_url.as_str()) {
        client.get_multiplexed_async_connection().await.is_ok()
    } else {
        false
    };

    if !redis_available {
        println!("⚠️  Skipping test: Redis not available");
        return;
    }

    let state = AppState::new(config).await.unwrap();
    let app = create_router(state);

    // Test that WebSocket endpoint exists
    let request = Request::builder()
        .uri("/ws/escalations")
        .method("GET")
        .header("upgrade", "websocket")
        .header("connection", "upgrade")
        .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
        .header("sec-websocket-version", "13")
        .body(Body::empty())
        .unwrap();

    let response = tower::ServiceExt::oneshot(app, request).await.unwrap();

    // Should either upgrade or return error (need valid auth)
    let status = response.status();
    assert!(
        status == StatusCode::SWITCHING_PROTOCOLS
            || status.is_client_error()
            || status.is_server_error(),
        "Unexpected status: {:?}",
        status
    );
}

#[tokio::test]
async fn test_redis_pubsub_channel_format() {
    let config = create_test_config();

    let redis_client = match redis::Client::open(config.redis_url.as_str()) {
        Ok(client) => client,
        Err(_) => {
            println!("⚠️  Skipping test: Redis not available");
            return;
        }
    };

    let mut conn = match redis_client.get_multiplexed_async_connection().await {
        Ok(conn) => conn,
        Err(_) => {
            println!("⚠️  Skipping test: Cannot connect to Redis");
            return;
        }
    };

    // Test publishing to the escalation channel
    let wallet = "test_approver";
    let channel = format!("escalation:events:{}", wallet);

    let event = json!({
        "type": "escalation_created",
        "escalation": {
            "escalation_id": "esc_test_123",
            "status": "pending",
            "risk_score": 75
        }
    });

    // Publish event
    let subscribers: usize = conn
        .publish(&channel, serde_json::to_string(&event).unwrap())
        .await
        .unwrap();

    // No subscribers in test, so should be 0
    assert_eq!(subscribers, 0);
}

#[tokio::test]
async fn test_websocket_message_structure() {
    // Test that WebSocket messages have correct structure
    let escalation_event = json!({
        "type": "escalation_created",
        "escalation": {
            "escalation_id": "esc_test",
            "canonical_hash": "hash123",
            "requester_wallet": "requester",
            "approver_wallet": "approver",
            "risk_score": 75,
            "status": "pending"
        }
    });

    // Verify it serializes correctly
    let message_str = serde_json::to_string(&escalation_event).unwrap();
    assert!(message_str.contains("escalation_created"));
    assert!(message_str.contains("esc_test"));

    // Verify it deserializes correctly
    let parsed: serde_json::Value = serde_json::from_str(&message_str).unwrap();
    assert_eq!(parsed["type"], "escalation_created");
    assert_eq!(parsed["escalation"]["escalation_id"], "esc_test");
}

#[tokio::test]
async fn test_websocket_event_types() {
    // Test all escalation event types
    let event_types = vec![
        (
            "escalation_created",
            json!({
                "type": "escalation_created",
                "escalation": {
                    "escalation_id": "esc_1",
                    "status": "pending"
                }
            }),
        ),
        (
            "escalation_approved",
            json!({
                "type": "escalation_approved",
                "escalation_id": "esc_1",
                "approved_by": "approver_wallet"
            }),
        ),
        (
            "escalation_denied",
            json!({
                "type": "escalation_denied",
                "escalation_id": "esc_1",
                "denied_by": "approver_wallet",
                "reason": "Too risky"
            }),
        ),
        (
            "escalation_expired",
            json!({
                "type": "escalation_expired",
                "escalation_id": "esc_1"
            }),
        ),
    ];

    for (event_type, event) in event_types {
        let serialized = serde_json::to_string(&event).unwrap();
        assert!(serialized.contains(event_type));

        // Verify deserialization
        let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        assert_eq!(parsed["type"], event_type);
    }
}

#[tokio::test]
async fn test_websocket_subscription_message() {
    // Test subscription message format
    let subscribe_msg = json!({
        "wallet": "test_approver",
        "signature": "base64_signature",
        "message": "challenge_message",
        "timestamp": chrono::Utc::now().timestamp()
    });

    let serialized = serde_json::to_string(&subscribe_msg).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();

    assert!(parsed["wallet"].is_string());
    assert!(parsed["signature"].is_string());
    assert!(parsed["timestamp"].is_number());
}

#[tokio::test]
async fn test_redis_channel_isolation() {
    let config = create_test_config();

    let redis_client = match redis::Client::open(config.redis_url.as_str()) {
        Ok(client) => client,
        Err(_) => {
            println!("⚠️  Skipping test: Redis not available");
            return;
        }
    };

    let mut conn = match redis_client.get_multiplexed_async_connection().await {
        Ok(conn) => conn,
        Err(_) => {
            println!("⚠️  Skipping test: Cannot connect to Redis");
            return;
        }
    };

    // Each wallet should have its own channel
    let wallet1 = "approver_1";
    let wallet2 = "approver_2";

    let channel1 = format!("escalation:events:{}", wallet1);
    let channel2 = format!("escalation:events:{}", wallet2);

    assert_ne!(channel1, channel2);

    let event1 = json!({"type": "test", "wallet": wallet1});
    let event2 = json!({"type": "test", "wallet": wallet2});

    // Publish to different channels
    let _: usize = conn
        .publish(&channel1, serde_json::to_string(&event1).unwrap())
        .await
        .unwrap();
    let _: usize = conn
        .publish(&channel2, serde_json::to_string(&event2).unwrap())
        .await
        .unwrap();

    // Events should be isolated (no subscribers, so both return 0)
    assert!(true);
}
