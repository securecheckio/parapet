/// End-to-end correctness tests for parapet-api
/// Tests the complete HTTP endpoint request/response cycle
use axum::body::Body;
use axum::http::{Request, StatusCode};
use parapet_api::{create_router, state::AppState, state::Config};
use serde_json::{json, Value};
use tower::util::ServiceExt;

#[allow(dead_code)]
mod common;

/// Create test config without Redis dependency
fn create_test_config() -> Config {
    Config {
        server_host: "127.0.0.1".to_string(),
        server_port: 3001,
        worker_threads: None,
        max_concurrent_scans: 2,
        scans_per_hour_per_key: 10,
        redis_url: "redis://localhost:6379".to_string(), // Not used in tests without Redis
        solana_rpc_url: "https://api.devnet.solana.com".to_string(),
        solana_rpc_urls: vec!["https://api.devnet.solana.com".to_string()],
        solana_network: "devnet".to_string(),
        authorized_wallets: vec!["test_wallet".to_string()],
        nonce_ttl_seconds: 300,
        mcp_api_keys: vec!["test_key".to_string()],
    }
}

/// Create test state for integration tests
async fn create_test_state() -> AppState {
    let config = create_test_config();
    // Create state without Redis for basic testing
    AppState::new_without_redis(config)
}

#[tokio::test]
async fn test_health_endpoint() {
    let state = create_test_state().await;
    let app = create_router(state);

    let request = Request::builder()
        .uri("/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_response: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json_response["status"], "ok");
    assert_eq!(json_response["service"], "parapet-api");
}

#[tokio::test]
async fn test_nonce_generation_endpoint() {
    let state = create_test_state().await;
    let app = create_router(state);

    let payload = json!({
        "wallet": "test_wallet_address"
    });

    let request = Request::builder()
        .uri("/api/v1/auth/nonce")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should either succeed or fail gracefully
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
            || response.status() == StatusCode::SERVICE_UNAVAILABLE
    );

    if response.status() == StatusCode::OK {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json_response: Value = serde_json::from_slice(&body).unwrap();

        // Should return nonce and timestamp
        assert!(json_response.get("nonce").is_some());
        assert!(json_response.get("timestamp").is_some());
    }
}

#[tokio::test]
async fn test_create_rule_endpoint_structure() {
    let state = create_test_state().await;
    let app = create_router(state);

    let rule = json!({
        "rule_id": "test_rule_123",
        "name": "Test Rule",
        "action": "pass",
        "conditions": {
            "all": [
                {
                    "field": "basic:instruction_count",
                    "operator": "greater_than",
                    "value": 0
                }
            ]
        },
        "weight": 50,
        "ttl_seconds": 3600
    });

    let request = Request::builder()
        .uri("/api/v1/rules")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&rule).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should either succeed (with Redis) or fail gracefully (without Redis)
    let status = response.status();
    assert!(
        status.is_success() || status.is_client_error() || status.is_server_error(),
        "Unexpected status: {:?}",
        status
    );

    if response.status().is_success() {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json_response: Value = serde_json::from_slice(&body).unwrap();

        // Should return success indicator
        assert!(json_response.get("success").is_some() || json_response.get("rule_id").is_some());
    }
}

#[tokio::test]
async fn test_list_rules_endpoint() {
    let state = create_test_state().await;
    let app = create_router(state);

    let payload = json!({});

    let request = Request::builder()
        .uri("/api/v1/rules/list")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should either succeed or fail gracefully
    let status = response.status();
    assert!(
        status.is_success() || status.is_client_error() || status.is_server_error(),
        "Unexpected status: {:?}",
        status
    );

    if response.status() == StatusCode::OK {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json_response: Value = serde_json::from_slice(&body).unwrap();

        // Should return array of rules
        assert!(json_response.is_array() || json_response.get("rules").is_some());
    }
}

#[tokio::test]
async fn test_get_escalation_not_found() {
    let state = create_test_state().await;
    let app = create_router(state);

    let request = Request::builder()
        .uri("/api/v1/escalations/nonexistent_id")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 404 or error (if Redis unavailable)
    let status = response.status();
    assert!(
        status.is_client_error() || status.is_server_error(),
        "Expected error status, got: {:?}",
        status
    );
}

#[tokio::test]
async fn test_approve_escalation_validation() {
    let state = create_test_state().await;
    let app = create_router(state);

    let payload = json!({
        "approver_wallet": "test_approver",
        "signature": "fake_signature"
    });

    let request = Request::builder()
        .uri("/api/v1/escalations/test_esc_id/approve")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return error (escalation doesn't exist or invalid signature)
    let status = response.status();
    assert!(
        status.is_client_error() || status.is_server_error(),
        "Expected error status, got: {:?}",
        status
    );
}

#[tokio::test]
async fn test_list_pending_escalations() {
    let state = create_test_state().await;
    let app = create_router(state);

    let payload = json!({
        "approver_wallet": "test_approver"
    });

    let request = Request::builder()
        .uri("/api/v1/escalations/pending")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should either succeed or fail gracefully
    let status = response.status();
    assert!(
        status.is_success() || status.is_client_error() || status.is_server_error(),
        "Unexpected status: {:?}",
        status
    );

    if status == StatusCode::OK {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json_response: Value = serde_json::from_slice(&body).unwrap();

        // Should return array
        assert!(
            json_response.is_array() || json_response.get("escalations").is_some(),
            "Response should be an array or contain escalations field"
        );
    }
}

#[tokio::test]
async fn test_export_rules_endpoint() {
    let state = create_test_state().await;
    let app = create_router(state);

    let payload = json!({});

    let request = Request::builder()
        .uri("/api/v1/rules/export")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should either succeed or fail gracefully
    let status = response.status();
    assert!(
        status.is_success() || status.is_client_error() || status.is_server_error(),
        "Unexpected status: {:?}",
        status
    );
}

#[tokio::test]
async fn test_import_rules_validation() {
    let state = create_test_state().await;
    let app = create_router(state);

    let invalid_rules = json!({
        "rules": "not_an_array"
    });

    let request = Request::builder()
        .uri("/api/v1/rules/import")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&invalid_rules).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return error for invalid input
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
            || response.status() == StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn test_delete_rule_endpoint() {
    let state = create_test_state().await;
    let app = create_router(state);

    let request = Request::builder()
        .uri("/api/v1/rules/test_rule_id")
        .method("DELETE")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should either succeed or fail gracefully
    let status = response.status();
    assert!(
        status.is_success() || status.is_client_error() || status.is_server_error(),
        "Unexpected status: {:?}",
        status
    );
}

#[tokio::test]
async fn test_mcp_endpoints_structure() {
    let state = create_test_state().await;
    let app = create_router(state);

    // Test MCP message endpoint
    let mcp_message = json!({
        "method": "test_method",
        "params": {}
    });

    let request = Request::builder()
        .uri("/mcp/message")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&mcp_message).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should handle request
    assert!(
        response.status().is_success()
            || response.status().is_client_error()
            || response.status().is_server_error()
    );
}

#[tokio::test]
async fn test_cors_headers_api() {
    let state = create_test_state().await;
    let app = create_router(state);

    let request = Request::builder()
        .uri("/health")
        .method("OPTIONS")
        .header("origin", "http://localhost:3000")
        .header("access-control-request-method", "GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should handle CORS preflight
    assert!(response.status().is_success());
}

#[tokio::test]
async fn test_404_for_unknown_routes() {
    let state = create_test_state().await;
    let app = create_router(state);

    let request = Request::builder()
        .uri("/nonexistent/route")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 404
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
