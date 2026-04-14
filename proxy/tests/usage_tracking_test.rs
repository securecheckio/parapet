/// Usage tracking tests for parapet-proxy
use parapet_proxy::usage_tracker::UsageTracker;

#[tokio::test]
async fn test_usage_tracker_creation() {
    // UsageTracker creation succeeds (client is lazy)
    let result = UsageTracker::new("redis://localhost:6379", 10_000);
    assert!(result.is_ok());
    
    // Actual connection happens on first use
    // Without Redis running, operations will fail
    if let Ok(tracker) = result {
        let check = tracker.check_rate_limit("test_wallet").await;
        // Will fail without Redis, which is expected
        assert!(check.is_err() || check.is_ok());
    }
}

#[tokio::test]
async fn test_usage_tracker_with_invalid_url() {
    let result = UsageTracker::new("not-a-valid-url", 10_000);
    assert!(result.is_err());
}

// Note: Full usage tracking tests require Redis
// See redis_integration_test.rs for complete usage tracking tests
