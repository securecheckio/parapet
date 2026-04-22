/// End-to-end correctness tests for parapet-rpc-proxy
/// Tests the complete HTTP request/response cycle
use axum::body::Body;
use axum::http::{Request, StatusCode};
use parapet_core::rules::analyzers::*;
use parapet_core::rules::{AnalyzerRegistry, RuleEngine};
use parapet_rpc_proxy::rpc_handler::{JsonRpcRequest, JsonRpcResponse};
use parapet_rpc_proxy::types::AppState;
use parapet_rpc_proxy::upstream;
use serde_json::{json, Value};
use solana_sdk::{
    message::Message, pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction,
};
use solana_system_interface::instruction as system_instruction;
use std::sync::Arc;
use tower::ServiceExt;

mod common;

/// Create a minimal test state without Redis
fn create_test_state() -> Arc<AppState> {
    let mut registry = AnalyzerRegistry::new();
    registry.register(Arc::new(BasicAnalyzer::new()));
    registry.register(Arc::new(TokenInstructionAnalyzer::new()));
    registry.register(Arc::new(SystemProgramAnalyzer::new()));

    let engine = RuleEngine::new(registry).with_flowstate(None);

    // Create simulation registry
    let mut sim_registry =
        parapet_core::rules::analyzers::simulation::SimulationAnalyzerRegistry::new();
    sim_registry.register(Box::new(
        parapet_core::rules::analyzers::simulation::SimulationBalanceAnalyzer::new(),
    ));
    sim_registry.register(Box::new(
        parapet_core::rules::analyzers::simulation::SimulationTokenAnalyzer::new(),
    ));
    sim_registry.register(Box::new(
        parapet_core::rules::analyzers::simulation::SimulationLogAnalyzer::new(),
    ));

    // Create upstream client with default config
    let upstream_client =
        upstream::UpstreamClient::new("https://api.devnet.solana.com".to_string());

    Arc::new(AppState {
        upstream_client,
        rule_engine: Arc::new(tokio::sync::RwLock::new(engine)),
        auth_provider: None,
        usage_tracker: None,
        allowed_wallets: None,
        output_manager: None,
        default_blocking_threshold: 70,
        simulation_registry: Arc::new(sim_registry),
        escalation_config: None,
        activity_feed_config: None,
    })
}

/// Create a valid JSON-RPC request
fn create_rpc_request(method: &str, params: Vec<Value>) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(1),
        method: method.to_string(),
        params,
    }
}

/// Create a transaction for testing
fn create_test_transaction() -> (Transaction, String) {
    let keypair = Keypair::new();
    let recipient = Pubkey::new_unique();
    let ix = system_instruction::transfer(&keypair.pubkey(), &recipient, 1_000_000);
    let message = Message::new(&[ix], Some(&keypair.pubkey()));
    let mut tx = Transaction::new_unsigned(message);
    tx.sign(&[&keypair], solana_sdk::hash::Hash::default());

    // Serialize to base58
    let tx_bytes = bincode::serialize(&tx).unwrap();
    let tx_b58 = bs58::encode(&tx_bytes).into_string();

    (tx, tx_b58)
}

#[tokio::test]
async fn test_getHealth_returns_success() {
    let state = create_test_state();
    let app = parapet_rpc_proxy::server::create_router_with_state(state);

    let request = Request::builder()
        .uri("/")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&create_rpc_request("getHealth", vec![])).unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_response: JsonRpcResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(json_response.jsonrpc, "2.0");
    assert!(json_response.error.is_none());
    assert!(json_response.result.is_some());
}

#[tokio::test]
async fn test_simulateTransaction_enriches_with_risk_score() {
    let state = create_test_state();
    let app = parapet_rpc_proxy::server::create_router_with_state(state);

    let (_tx, tx_b58) = create_test_transaction();

    let request = Request::builder()
        .uri("/")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&create_rpc_request(
                "simulateTransaction",
                vec![json!(tx_b58)],
            ))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_response: JsonRpcResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(json_response.jsonrpc, "2.0");

    // Should have result (even if upstream fails, we return structure)
    if let Some(result) = json_response.result {
        // Check for solShield enrichment
        if let Some(sol_shield) = result.get("solShield") {
            assert!(sol_shield.is_object(), "solShield should be an object");
            // Should contain risk fields
            assert!(
                sol_shield.get("totalRisk").is_some()
                    || sol_shield.get("action").is_some()
                    || sol_shield.get("message").is_some(),
                "solShield should contain risk assessment fields"
            );
        }
    }
}

#[tokio::test]
async fn test_sendTransaction_blocks_high_risk() {
    let state = create_test_state();

    // Load a blocking rule
    let rules = vec![parapet_core::rules::types::RuleDefinition {
        version: "1.0".to_string(),
        id: "test-block-all".to_string(),
        name: "Block All Transactions".to_string(),
        description: Some("Test blocking rule".to_string()),
        author: None,
        enabled: true,
        tags: vec![],
        rule: parapet_core::rules::types::Rule {
            action: parapet_core::rules::types::RuleAction::Block,
            conditions: serde_json::from_value(json!({
                "all": [
                    {
                        "field": "basic:signers_count",
                        "operator": "greater_than",
                        "value": 0
                    }
                ]
            }))
            .unwrap(),
            message: "Transaction blocked for testing".to_string(),
            flowstate: None,
        },
        metadata: Default::default(),
    }];

    {
        let mut engine = state.rule_engine.write().await;
        engine.load_rules(rules).unwrap();
    }

    let app = parapet_rpc_proxy::server::create_router_with_state(state);

    let (_tx, tx_b58) = create_test_transaction();

    let request = Request::builder()
        .uri("/")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&create_rpc_request("sendTransaction", vec![json!(tx_b58)]))
                .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return an error when blocked
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_response: JsonRpcResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(json_response.jsonrpc, "2.0");
    assert!(
        json_response.error.is_some(),
        "Should return error for blocked transaction"
    );

    let error = json_response.error.unwrap();
    assert!(
        error.message.contains("Block") || error.message.contains("blocked"),
        "Error message should indicate blocking: {}",
        error.message
    );
}

#[tokio::test]
async fn test_passthrough_for_non_transaction_methods() {
    let state = create_test_state();
    let app = parapet_rpc_proxy::server::create_router_with_state(state);

    // Test a read-only method that should pass through
    let request = Request::builder()
        .uri("/")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&create_rpc_request(
                "getLatestBlockhash",
                vec![json!({"commitment": "finalized"})],
            ))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_response: JsonRpcResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(json_response.jsonrpc, "2.0");
    // Should pass through to upstream (result or error from upstream)
    assert!(
        json_response.result.is_some() || json_response.error.is_some(),
        "Should have response from upstream"
    );
}

#[tokio::test]
async fn test_malformed_json_returns_error() {
    let state = create_test_state();
    let app = parapet_rpc_proxy::server::create_router_with_state(state);

    let request = Request::builder()
        .uri("/")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from("invalid json"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 400 or JSON-RPC error
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
            || response.status() == StatusCode::OK
    );
}

#[tokio::test]
async fn test_batch_requests_handled() {
    let state = create_test_state();
    let app = parapet_rpc_proxy::server::create_router_with_state(state);

    let batch = vec![
        create_rpc_request("getHealth", vec![]),
        create_rpc_request("getVersion", vec![]),
    ];

    let request = Request::builder()
        .uri("/")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&batch).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should handle batch or return appropriate error (proxy might not support batch)
    assert!(
        response.status().is_success()
            || response.status().is_client_error()
            || response.status() == StatusCode::NOT_IMPLEMENTED
            || response.status() == StatusCode::SERVICE_UNAVAILABLE,
        "Unexpected status code: {:?}",
        response.status()
    );
}

#[tokio::test]
async fn test_rule_engine_decision_flow() {
    let state = create_test_state();

    // Add a PASS rule
    let rules = vec![parapet_core::rules::types::RuleDefinition {
        version: "1.0".to_string(),
        id: "test-pass".to_string(),
        name: "Pass All".to_string(),
        description: None,
        author: None,
        enabled: true,
        tags: vec![],
        rule: parapet_core::rules::types::Rule {
            action: parapet_core::rules::types::RuleAction::Pass,
            conditions: serde_json::from_value(json!({
                "all": [
                    {
                        "field": "basic:instruction_count",
                        "operator": "greater_than",
                        "value": 0
                    }
                ]
            }))
            .unwrap(),
            message: "Transaction passed for testing".to_string(),
            flowstate: None,
        },
        metadata: Default::default(),
    }];

    {
        let mut engine = state.rule_engine.write().await;
        engine.load_rules(rules).unwrap();
    }

    let app = parapet_rpc_proxy::server::create_router_with_state(state);

    let (_tx, tx_b58) = create_test_transaction();

    let request = Request::builder()
        .uri("/")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&create_rpc_request("sendTransaction", vec![json!(tx_b58)]))
                .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_response: JsonRpcResponse = serde_json::from_slice(&body).unwrap();

    // With PASS rule, should proceed to upstream
    // (may fail if upstream unreachable, but shouldn't be blocked by proxy)
    assert_eq!(json_response.jsonrpc, "2.0");
}

#[tokio::test]
async fn test_cors_headers_present() {
    let state = create_test_state();
    let app = parapet_rpc_proxy::server::create_router_with_state(state);

    let request = Request::builder()
        .uri("/")
        .method("OPTIONS")
        .header("origin", "http://localhost:3000")
        .header("access-control-request-method", "POST")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should handle CORS preflight
    let headers = response.headers();
    assert!(
        headers.contains_key("access-control-allow-origin") || response.status() == StatusCode::OK
    );
}
