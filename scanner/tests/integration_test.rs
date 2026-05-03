use parapet_scanner::{ScanConfig, WalletScanner};
use serde_json::json;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

/// Echo JSON-RPC `id` and return an empty `getProgramAccounts` result (active-state scan path).
#[derive(Clone, Copy)]
struct EmptyProgramAccountsRpc;

impl Respond for EmptyProgramAccountsRpc {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let id = serde_json::from_slice::<serde_json::Value>(&request.body)
            .ok()
            .and_then(|v| v.get("id").cloned())
            .unwrap_or(json!(0));
        let body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": []
        });
        ResponseTemplate::new(200).set_body_json(&body)
    }
}

#[tokio::test]
async fn test_basic_scanner_creation() {
    let scanner = WalletScanner::new("https://api.mainnet-beta.solana.com".to_string());
    assert!(scanner.is_ok());
}

#[tokio::test]
async fn test_scan_config_default() {
    let config = ScanConfig::default();
    assert_eq!(config.max_transactions, Some(100));
    assert_eq!(config.time_window_days, Some(30));
    assert!(config.check_active_threats);
    assert!(config.check_historical);
}

/// Active-state scan calls `getProgramAccounts`; mock RPC so tests do not hit the public internet.
/// `RpcClient` uses blocking HTTP; run on the multi-threaded runtime (see Solana rpc_client).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_wallet_scan_active_state_with_mock_rpc() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(EmptyProgramAccountsRpc)
        .mount(&server)
        .await;

    let url = server.uri();
    let scanner = WalletScanner::new(url).expect("scanner with mock URL");

    let mut config = ScanConfig::default();
    config.check_historical = false;

    let report = scanner
        .scan("11111111111111111111111111111112", config)
        .await
        .expect("scan with mocked empty token accounts");

    assert_eq!(report.wallet, "11111111111111111111111111111112");
    assert_eq!(report.stats.transactions_analyzed, 0);
}
