/// Cache tests for parapet-proxy
use parapet_proxy::cache::Cache;
use solana_sdk::pubkey::Pubkey;

#[tokio::test]
async fn test_in_memory_cache_creation() {
    let cache = Cache::new_in_memory();
    assert!(cache.is_ok());
}

#[tokio::test]
async fn test_cache_allowlist() {
    let cache = Cache::new_in_memory().unwrap();
    let program_id = Pubkey::new_unique();

    // Initially not in allowlist
    let is_allowed = cache.is_allowed(&program_id).await;
    assert!(is_allowed.is_ok());
    // In-memory cache starts empty, so program is not allowed
    let allowed = is_allowed.unwrap();
    assert!(!allowed);
}

#[tokio::test]
async fn test_cache_backend_types() {
    // Test in-memory backend
    let in_memory_cache = Cache::new_in_memory();
    assert!(in_memory_cache.is_ok());

    // Test Redis backend (will fail without Redis, which is expected)
    let redis_cache = Cache::new("redis://invalid:6379").await;
    assert!(redis_cache.is_err()); // Expected to fail with invalid URL
}

#[tokio::test]
async fn test_cache_structure() {
    let cache = Cache::new_in_memory().unwrap();
    // Just verify the cache can be created and used
    let test_program = Pubkey::new_unique();
    let _ = cache.is_allowed(&test_program).await;
}
