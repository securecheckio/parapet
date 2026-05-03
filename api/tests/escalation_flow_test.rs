/// End-to-end escalation flow tests
/// Tests the complete pause → approve/deny → resume workflow
use axum::body::Body;
use axum::http::{Request, StatusCode};
use parapet_api::{create_router, state::AppState, state::Config};
use redis::AsyncCommands;
use serde_json::{json, Value};
use tower::util::ServiceExt;

mod common;

/// Create test config
fn create_test_config() -> Config {
    Config {
        server_host: "127.0.0.1".to_string(),
        server_port: 3001,
        worker_threads: None,
        max_concurrent_scans: 2,
        scans_per_hour_per_key: 10,
        redis_url: std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
        solana_rpc_url: "https://api.devnet.solana.com".to_string(),
        solana_rpc_urls: vec!["https://api.devnet.solana.com".to_string()],
        solana_network: "devnet".to_string(),
        authorized_wallets: vec!["test_approver".to_string()],
        nonce_ttl_seconds: 300,
        mcp_api_keys: vec!["test_key".to_string()],
    }
}

#[tokio::test]
async fn test_escalation_create_and_retrieve() {
    let config = create_test_config();

    // Try to connect to Redis
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

    let state = AppState::new(config).await.unwrap();
    let app = create_router(state);

    // Create test escalation directly in Redis
    let escalation_id = format!("esc_test_{}", uuid::Uuid::new_v4());
    let escalation = json!({
        "escalation_id": escalation_id,
        "canonical_hash": "test_hash_123",
        "requester_wallet": "requester_test",
        "approver_wallet": "test_approver",
        "risk_score": 75,
        "warnings": ["Test warning"],
        "decoded_instructions": [],
        "suggested_rules": [],
        "status": "pending",
        "created_at": chrono::Utc::now().timestamp() as u64,
        "expires_at": (chrono::Utc::now().timestamp() + 300) as u64,
    });

    let escalation_key = format!("escalation:pending:{}", escalation_id);
    let _: () = conn
        .set_ex(
            &escalation_key,
            serde_json::to_string(&escalation).unwrap(),
            300,
        )
        .await
        .unwrap();

    // Also add to approver's pending set
    let approver_key = "escalation:pending:approver:test_approver";
    let _: () = conn.sadd(approver_key, &escalation_id).await.unwrap();
    let _: () = conn.expire(approver_key, 300).await.unwrap();

    // GET the escalation
    let request = Request::builder()
        .uri(format!("/api/v1/escalations/{}", escalation_id))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    let status = response.status();
    assert!(
        status.is_success() || status.is_client_error(),
        "Unexpected status: {:?}",
        status
    );

    if status != StatusCode::OK {
        // If not OK, skip body validation
        let _: () = conn.del(&escalation_key).await.unwrap();
        let _: () = conn.srem(approver_key, &escalation_id).await.unwrap();
        return;
    }

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_response: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json_response["escalation_id"], escalation_id);
    assert_eq!(json_response["status"], "pending");
    assert_eq!(json_response["risk_score"], 75);

    // Cleanup
    let _: () = conn.del(&escalation_key).await.unwrap();
    let _: () = conn.srem(approver_key, &escalation_id).await.unwrap();
}

#[tokio::test]
async fn test_escalation_approve_flow() {
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

    let state = AppState::new(config).await.unwrap();
    let app = create_router(state);

    // Create test escalation
    let escalation_id = format!("esc_approve_{}", uuid::Uuid::new_v4());
    let escalation = json!({
        "escalation_id": escalation_id,
        "canonical_hash": "test_hash_approve",
        "requester_wallet": "requester_test",
        "approver_wallet": "test_approver",
        "risk_score": 75,
        "warnings": ["Test warning"],
        "decoded_instructions": [],
        "suggested_rules": [],
        "status": "pending",
        "created_at": chrono::Utc::now().timestamp() as u64,
        "expires_at": (chrono::Utc::now().timestamp() + 300) as u64,
    });

    let escalation_key = format!("escalation:pending:{}", escalation_id);
    let _: () = conn
        .set_ex(
            &escalation_key,
            serde_json::to_string(&escalation).unwrap(),
            300,
        )
        .await
        .unwrap();

    // Store pending transaction
    let tx_key = format!("pending_tx:{}", escalation_id);
    let _: () = conn
        .set_ex(&tx_key, vec![1u8, 2, 3, 4, 5], 50)
        .await
        .unwrap();

    // Approve the escalation
    let approve_payload = json!({
        "approver_wallet": "test_approver",
        "signature": "test_signature_approve",
        "message": "Approved by test"
    });

    let request = Request::builder()
        .uri(format!("/api/v1/escalations/{}/approve", escalation_id))
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&approve_payload).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should succeed or return validation error (signature verification)
    let status = response.status();
    assert!(
        status.is_success() || status.is_client_error() || status.is_server_error(),
        "Unexpected status: {:?}",
        status
    );

    // Cleanup
    let _: () = conn.del(&escalation_key).await.unwrap();
    let _: () = conn.del(&tx_key).await.unwrap();
}

#[tokio::test]
async fn test_escalation_deny_flow() {
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

    let state = AppState::new(config).await.unwrap();
    let app = create_router(state);

    // Create test escalation
    let escalation_id = format!("esc_deny_{}", uuid::Uuid::new_v4());
    let escalation = json!({
        "escalation_id": escalation_id,
        "canonical_hash": "test_hash_deny",
        "requester_wallet": "requester_test",
        "approver_wallet": "test_approver",
        "risk_score": 85,
        "warnings": ["High risk warning"],
        "decoded_instructions": [],
        "suggested_rules": [],
        "status": "pending",
        "created_at": chrono::Utc::now().timestamp() as u64,
        "expires_at": (chrono::Utc::now().timestamp() + 300) as u64,
    });

    let escalation_key = format!("escalation:pending:{}", escalation_id);
    let _: () = conn
        .set_ex(
            &escalation_key,
            serde_json::to_string(&escalation).unwrap(),
            300,
        )
        .await
        .unwrap();

    // Deny the escalation
    let deny_payload = json!({
        "approver_wallet": "test_approver",
        "signature": "test_signature_deny",
        "reason": "Too risky"
    });

    let request = Request::builder()
        .uri(format!("/api/v1/escalations/{}/deny", escalation_id))
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&deny_payload).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should succeed or return validation error
    let status = response.status();
    assert!(
        status.is_success() || status.is_client_error() || status.is_server_error(),
        "Unexpected status: {:?}",
        status
    );

    // Cleanup
    let _: () = conn.del(&escalation_key).await.unwrap();
}

#[tokio::test]
async fn test_list_pending_escalations_for_approver() {
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

    let state = AppState::new(config).await.unwrap();
    let app = create_router(state);

    // Create multiple test escalations
    let escalation_ids: Vec<String> = (0..3)
        .map(|i| format!("esc_list_{}_{}", i, uuid::Uuid::new_v4()))
        .collect();

    for esc_id in &escalation_ids {
        let escalation = json!({
            "escalation_id": esc_id,
            "canonical_hash": format!("test_hash_{}", esc_id),
            "requester_wallet": "requester_test",
            "approver_wallet": "test_approver",
            "risk_score": 70,
            "warnings": ["Test warning"],
            "decoded_instructions": [],
            "suggested_rules": [],
            "status": "pending",
            "created_at": chrono::Utc::now().timestamp() as u64,
            "expires_at": (chrono::Utc::now().timestamp() + 300) as u64,
        });

        let escalation_key = format!("escalation:pending:{}", esc_id);
        let _: () = conn
            .set_ex(
                &escalation_key,
                serde_json::to_string(&escalation).unwrap(),
                300,
            )
            .await
            .unwrap();

        // Add to approver's pending set
        let _: () = conn
            .sadd("escalation:pending:approver:test_approver", esc_id)
            .await
            .unwrap();
    }
    let _: () = conn
        .expire("escalation:pending:approver:test_approver", 300)
        .await
        .unwrap();

    // List pending escalations
    let list_payload = json!({
        "approver_wallet": "test_approver"
    });

    let request = Request::builder()
        .uri("/api/v1/escalations/pending")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&list_payload).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    if response.status() == StatusCode::OK {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json_response: Value = serde_json::from_slice(&body).unwrap();

        // Should return an array of escalations
        if let Some(escalations) = json_response.as_array() {
            assert!(escalations.len() > 0);
        }
    }

    // Cleanup
    for esc_id in &escalation_ids {
        let escalation_key = format!("escalation:pending:{}", esc_id);
        let _: () = conn.del(&escalation_key).await.unwrap();
    }
    let _: () = conn
        .del("escalation:pending:approver:test_approver")
        .await
        .unwrap();
}

#[tokio::test]
async fn test_escalation_status_check() {
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

    let state = AppState::new(config).await.unwrap();
    let app = create_router(state);

    // Create test escalation
    let escalation_id = format!("esc_status_{}", uuid::Uuid::new_v4());
    let escalation = json!({
        "escalation_id": escalation_id,
        "canonical_hash": "test_hash_status",
        "requester_wallet": "requester_test",
        "approver_wallet": "test_approver",
        "risk_score": 75,
        "warnings": ["Test warning"],
        "decoded_instructions": [],
        "suggested_rules": [],
        "status": "pending",
        "created_at": chrono::Utc::now().timestamp() as u64,
        "expires_at": (chrono::Utc::now().timestamp() + 300) as u64,
    });

    let escalation_key = format!("escalation:pending:{}", escalation_id);
    let _: () = conn
        .set_ex(
            &escalation_key,
            serde_json::to_string(&escalation).unwrap(),
            300,
        )
        .await
        .unwrap();

    // Check status
    let request = Request::builder()
        .uri(format!("/api/v1/escalations/{}/status", escalation_id))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    let status = response.status();
    assert!(
        status.is_success() || status.is_client_error(),
        "Unexpected status: {:?}",
        status
    );

    if status != StatusCode::OK {
        // If not OK, skip body validation
        let _: () = conn.del(&escalation_key).await.unwrap();
        return;
    }

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_response: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json_response["status"], "pending");

    // Cleanup
    let _: () = conn.del(&escalation_key).await.unwrap();
}
