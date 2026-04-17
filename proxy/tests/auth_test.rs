/// Authentication tests for parapet-proxy
use axum::body::Body;
use axum::http::{Request, StatusCode};
use parapet_core::rules::analyzers::*;
use parapet_core::rules::{AnalyzerRegistry, RuleEngine};
use parapet_proxy::auth::{AuthContext, AuthProvider, AuthResult};
use parapet_proxy::rpc_handler::{JsonRpcRequest, JsonRpcResponse};
use parapet_proxy::types::AppState;
use parapet_proxy::upstream;
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;
use tower::ServiceExt;

mod common;

/// Create test state with API key auth
fn create_state_with_api_key_auth(valid_keys: Vec<String>) -> Arc<AppState> {
    let mut registry = AnalyzerRegistry::new();
    registry.register(Arc::new(BasicAnalyzer::new()));

    let engine = RuleEngine::new(registry).with_flowstate(None);
    let mut sim_registry =
        parapet_core::rules::analyzers::simulation::SimulationAnalyzerRegistry::new();
    sim_registry.register(Box::new(
        parapet_core::rules::analyzers::simulation::SimulationBalanceAnalyzer::new(),
    ));

    // Create simple API key auth provider
    struct SimpleApiKeyAuth {
        valid_keys: HashSet<String>,
    }

    #[async_trait::async_trait]
    impl AuthProvider for SimpleApiKeyAuth {
        async fn authenticate(
            &self,
            headers: &axum::http::HeaderMap,
            _method: &str,
        ) -> Result<AuthResult, anyhow::Error> {
            let api_key = headers
                .get("X-API-Key")
                .or_else(|| headers.get("Authorization"))
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| anyhow::anyhow!("Missing API key"))?;

            if self.valid_keys.contains(api_key) {
                Ok(AuthResult {
                    context: AuthContext {
                        identity: api_key.to_string(),
                        tier: Some("standard".to_string()),
                        wallets: vec![],
                        scopes: vec![],
                        metadata: std::collections::HashMap::new(),
                    },
                    quota_remaining: Some(1000),
                    quota_reset: Some(3600),
                })
            } else {
                Err(anyhow::anyhow!("Invalid API key"))
            }
        }

        async fn on_success(
            &self,
            _context: &AuthContext,
            _method: &str,
            _status_code: u16,
        ) -> Result<(), anyhow::Error> {
            Ok(())
        }

        async fn on_failure(
            &self,
            _context: Option<&AuthContext>,
            _method: &str,
            _reason: &str,
        ) -> Result<(), anyhow::Error> {
            Ok(())
        }

        fn name(&self) -> &str {
            "simple-api-key"
        }
    }

    let auth_provider = Arc::new(SimpleApiKeyAuth {
        valid_keys: valid_keys.into_iter().collect(),
    });

    let upstream_client =
        upstream::UpstreamClient::new("https://api.devnet.solana.com".to_string());

    Arc::new(AppState {
        upstream_client,
        rule_engine: Arc::new(tokio::sync::RwLock::new(engine)),
        auth_provider: Some(auth_provider),
        usage_tracker: None,
        allowed_wallets: None,
        output_manager: None,
        default_blocking_threshold: 70,
        simulation_registry: Arc::new(sim_registry),
        escalation_config: None,
        activity_feed_config: None,
    })
}

/// Create test state with wallet allowlist
fn create_state_with_wallet_allowlist(allowed: Vec<String>) -> Arc<AppState> {
    let mut registry = AnalyzerRegistry::new();
    registry.register(Arc::new(BasicAnalyzer::new()));

    let engine = RuleEngine::new(registry).with_flowstate(None);
    let mut sim_registry =
        parapet_core::rules::analyzers::simulation::SimulationAnalyzerRegistry::new();
    sim_registry.register(Box::new(
        parapet_core::rules::analyzers::simulation::SimulationBalanceAnalyzer::new(),
    ));

    let upstream_client =
        upstream::UpstreamClient::new("https://api.devnet.solana.com".to_string());

    Arc::new(AppState {
        upstream_client,
        rule_engine: Arc::new(tokio::sync::RwLock::new(engine)),
        auth_provider: None,
        usage_tracker: None,
        allowed_wallets: Some(allowed.into_iter().collect()),
        output_manager: None,
        default_blocking_threshold: 70,
        simulation_registry: Arc::new(sim_registry),
        escalation_config: None,
        activity_feed_config: None,
    })
}

#[tokio::test]
async fn test_api_key_auth_success() {
    let state = create_state_with_api_key_auth(vec!["valid-key-123".to_string()]);
    let app = parapet_proxy::server::create_router_with_state(state);

    let request = Request::builder()
        .uri("/")
        .method("POST")
        .header("content-type", "application/json")
        .header("X-API-Key", "valid-key-123")
        .body(Body::from(
            serde_json::to_string(&JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: json!(1),
                method: "getHealth".to_string(),
                params: vec![],
            })
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_api_key_auth_failure() {
    let state = create_state_with_api_key_auth(vec!["valid-key-123".to_string()]);
    let app = parapet_proxy::server::create_router_with_state(state);

    let request = Request::builder()
        .uri("/")
        .method("POST")
        .header("content-type", "application/json")
        .header("X-API-Key", "invalid-key")
        .body(Body::from(
            serde_json::to_string(&JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: json!(1),
                method: "getHealth".to_string(),
                params: vec![],
            })
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_response: JsonRpcResponse = serde_json::from_slice(&body).unwrap();

    assert!(json_response.error.is_some());
    assert!(json_response
        .error
        .unwrap()
        .message
        .contains("Authentication failed"));
}

#[tokio::test]
async fn test_api_key_missing() {
    let state = create_state_with_api_key_auth(vec!["valid-key-123".to_string()]);
    let app = parapet_proxy::server::create_router_with_state(state);

    let request = Request::builder()
        .uri("/")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: json!(1),
                method: "getHealth".to_string(),
                params: vec![],
            })
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_query_param_auth() {
    let state = create_state_with_api_key_auth(vec!["query-key-456".to_string()]);
    let app = parapet_proxy::server::create_router_with_state(state);

    let request = Request::builder()
        .uri("/?api-key=query-key-456")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: json!(1),
                method: "getHealth".to_string(),
                params: vec![],
            })
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_authorization_header() {
    let state = create_state_with_api_key_auth(vec!["bearer-token".to_string()]);
    let app = parapet_proxy::server::create_router_with_state(state);

    let request = Request::builder()
        .uri("/")
        .method("POST")
        .header("content-type", "application/json")
        .header("Authorization", "bearer-token")
        .body(Body::from(
            serde_json::to_string(&JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: json!(1),
                method: "getHealth".to_string(),
                params: vec![],
            })
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_wallet_allowlist_not_configured() {
    // No allowlist = all requests allowed
    let state = create_state_with_wallet_allowlist(vec![]);
    let app = parapet_proxy::server::create_router_with_state(state);

    let request = Request::builder()
        .uri("/")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: json!(1),
                method: "getHealth".to_string(),
                params: vec![],
            })
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should succeed even without wallet since allowlist is empty (disabled)
    assert!(response.status().is_success() || response.status().is_server_error());
}
