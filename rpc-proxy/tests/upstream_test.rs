use parapet_rpc_proxy::rpc_handler::JsonRpcRequest;
/// Upstream client tests for parapet-rpc-proxy
use parapet_rpc_proxy::upstream::{CircuitState, UpstreamClient, UpstreamHttpSettings};
use serde_json::json;

const DEVNET_RPC: &str = "https://api.devnet.solana.com";

#[tokio::test]
async fn test_upstream_client_creation() {
    let url = DEVNET_RPC.to_string();
    let client = UpstreamClient::new(url.clone());
    assert_eq!(client.upstream_url, url);
    assert!(
        client.circuit_call_permitted().await,
        "new client should permit calls (circuit closed)"
    );
    assert_eq!(
        client.get_circuit_state().await,
        CircuitState::Closed,
        "circuit should start closed"
    );
}

#[tokio::test]
async fn test_upstream_client_with_config() {
    let config = UpstreamHttpSettings {
        max_concurrent: 5,
        delay_ms: 50,
        timeout_secs: 10,
        max_retries: 2,
        retry_base_delay_ms: 100,
        circuit_breaker_threshold: 3,
        circuit_breaker_timeout_secs: 30,
    };

    let url = DEVNET_RPC.to_string();
    let client = UpstreamClient::new_with_config(url.clone(), config);
    assert_eq!(client.upstream_url, url);
    assert!(
        client.circuit_call_permitted().await,
        "configured client should permit calls (circuit closed)"
    );
    assert_eq!(client.get_circuit_state().await, CircuitState::Closed);
}

#[tokio::test]
async fn test_upstream_forward_invalid_url() {
    let client =
        UpstreamClient::new("http://invalid-url-that-does-not-exist-12345.com".to_string());

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(1),
        method: "getHealth".to_string(),
        params: vec![],
    };

    let result = client.forward(&request).await;

    // Should fail with network error
    assert!(result.is_err());
}

#[tokio::test]
async fn test_upstream_config_defaults() {
    let config = UpstreamHttpSettings {
        max_concurrent: 10,
        delay_ms: 100,
        timeout_secs: 30,
        max_retries: 3,
        retry_base_delay_ms: 100,
        circuit_breaker_threshold: 5,
        circuit_breaker_timeout_secs: 60,
    };

    assert_eq!(config.max_concurrent, 10);
    assert_eq!(config.delay_ms, 100);
    assert_eq!(config.timeout_secs, 30);
    assert_eq!(config.max_retries, 3);
}
