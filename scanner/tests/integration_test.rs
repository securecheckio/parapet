use parapet_scanner::{ScanConfig, WalletScanner};

#[tokio::test]
#[ignore] // Ignore by default as it requires RPC connection
async fn test_basic_scanner_creation() {
    let scanner = WalletScanner::new("https://api.mainnet-beta.solana.com".to_string());
    assert!(scanner.is_ok());
}

#[tokio::test]
#[ignore] // Ignore by default as it requires RPC connection
async fn test_scan_config_default() {
    let config = ScanConfig::default();
    assert_eq!(config.max_transactions, Some(100));
    assert_eq!(config.time_window_days, Some(30));
    assert!(config.check_active_threats);
    assert!(config.check_historical);
}

// Note: Real wallet scanning tests would require:
// 1. A test wallet with known transaction history
// 2. RPC endpoint access
// 3. Longer test timeouts
// These should be added as integration tests in a separate test suite
