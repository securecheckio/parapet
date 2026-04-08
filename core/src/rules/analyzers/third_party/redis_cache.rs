use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use std::collections::HashMap;

#[cfg(feature = "redis")]
use redis;

/// Shared cache that uses Redis when available, falls back to in-memory
pub struct SharedCache {
    #[cfg(feature = "redis")]
    redis_client: Option<Arc<redis::Client>>,
    fallback_cache: Arc<Mutex<HashMap<String, (Vec<u8>, std::time::Instant)>>>,
}

impl SharedCache {
    /// Create a new shared cache
    /// 
    /// If redis_url is provided and connection succeeds, uses Redis.
    /// Otherwise falls back to in-memory cache.
    pub fn new(redis_url: Option<String>) -> Self {
        #[cfg(feature = "redis")]
        let redis_client = redis_url.and_then(|url| {
            match redis::Client::open(url.as_str()) {
                Ok(client) => {
                    log::info!("✅ SharedCache: Connected to Redis at {}", url);
                    Some(Arc::new(client))
                }
                Err(e) => {
                    log::warn!("⚠️  SharedCache: Failed to connect to Redis: {}. Using in-memory fallback.", e);
                    None
                }
            }
        });

        #[cfg(not(feature = "redis"))]
        let _redis_url = redis_url; // Suppress unused warning

        #[cfg(feature = "redis")]
        if redis_client.is_none() {
            log::info!("💾 SharedCache: Using in-memory cache (no Redis configured)");
        }

        #[cfg(not(feature = "redis"))]
        log::info!("💾 SharedCache: Using in-memory cache (Redis feature not enabled)");

        Self {
            #[cfg(feature = "redis")]
            redis_client,
            fallback_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get a value from cache
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        // Try Redis first
        #[cfg(feature = "redis")]
        if let Some(client) = &self.redis_client {
            match self.get_from_redis(client, key).await {
                Ok(Some(value)) => return Ok(Some(value)),
                Ok(None) => return Ok(None),
                Err(e) => {
                    log::warn!("Redis GET failed for key '{}': {}. Checking fallback cache.", key, e);
                }
            }
        }

        // Fallback to in-memory
        self.get_from_memory(key).await
    }

    /// Set a value in cache with TTL
    pub async fn set<T: Serialize>(&self, key: &str, value: &T, ttl: Duration) -> Result<()> {
        let serialized = serde_json::to_vec(value)?;

        // Try Redis first
        #[cfg(feature = "redis")]
        if let Some(client) = &self.redis_client {
            match self.set_in_redis(client, key, &serialized, ttl).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    log::warn!("Redis SET failed for key '{}': {}. Using fallback cache.", key, e);
                }
            }
        }

        // Fallback to in-memory
        self.set_in_memory(key, serialized, ttl).await
    }

    /// Check if cache is using Redis (true) or in-memory fallback (false)
    pub fn is_redis_enabled(&self) -> bool {
        #[cfg(feature = "redis")]
        return self.redis_client.is_some();
        
        #[cfg(not(feature = "redis"))]
        return false;
    }

    // Private helper methods

    #[cfg(feature = "redis")]
    async fn get_from_redis<T: DeserializeOwned>(
        &self,
        client: &redis::Client,
        key: &str,
    ) -> Result<Option<T>> {
        use redis::AsyncCommands;

        let mut conn = client.get_multiplexed_async_connection().await?;
        let bytes: Option<Vec<u8>> = conn.get(key).await?;

        match bytes {
            Some(b) => {
                let value: T = serde_json::from_slice(&b)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    #[cfg(feature = "redis")]
    async fn set_in_redis(
        &self,
        client: &redis::Client,
        key: &str,
        value: &[u8],
        ttl: Duration,
    ) -> Result<()> {
        use redis::AsyncCommands;

        let mut conn = client.get_multiplexed_async_connection().await?;
        let ttl_secs = ttl.as_secs() as usize;
        conn.set_ex(key, value, ttl_secs).await?;
        Ok(())
    }

    async fn get_from_memory<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let cache = self.fallback_cache.lock().await;
        
        if let Some((bytes, expiry)) = cache.get(key) {
            // Check if expired
            if std::time::Instant::now() < *expiry {
                let value: T = serde_json::from_slice(bytes)?;
                return Ok(Some(value));
            }
        }

        Ok(None)
    }

    async fn set_in_memory(&self, key: &str, value: Vec<u8>, ttl: Duration) -> Result<()> {
        let mut cache = self.fallback_cache.lock().await;
        let expiry = std::time::Instant::now() + ttl;
        cache.insert(key.to_string(), (value, expiry));
        
        // Simple cleanup: remove expired entries if cache is getting large
        if cache.len() > 1000 {
            cache.retain(|_, (_, exp)| std::time::Instant::now() < *exp);
        }

        Ok(())
    }

    /// Clear all entries from cache (useful for testing)
    pub async fn clear(&self) -> Result<()> {
        #[cfg(feature = "redis")]
        if let Some(client) = &self.redis_client {
            use redis::AsyncCommands;
            let mut conn = client.get_multiplexed_async_connection().await?;
            redis::cmd("FLUSHDB").query_async(&mut conn).await?;
        }

        let mut cache = self.fallback_cache.lock().await;
        cache.clear();

        Ok(())
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let memory_entries = self.fallback_cache.lock().await.len();
        
        CacheStats {
            is_redis: self.is_redis_enabled(),
            memory_entries,
        }
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub is_redis: bool,
    pub memory_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        value: String,
    }

    #[tokio::test]
    async fn test_in_memory_cache() {
        let cache = SharedCache::new(None);
        assert!(!cache.is_redis_enabled());

        let key = "test_key";
        let data = TestData {
            value: "test_value".to_string(),
        };

        // Set and get
        cache.set(key, &data, Duration::from_secs(60)).await.unwrap();
        let retrieved: Option<TestData> = cache.get(key).await.unwrap();
        assert_eq!(retrieved, Some(data));

        // Non-existent key
        let missing: Option<TestData> = cache.get("missing").await.unwrap();
        assert_eq!(missing, None);
    }

    #[tokio::test]
    async fn test_expiry() {
        let cache = SharedCache::new(None);

        let key = "expiring_key";
        let data = TestData {
            value: "expires_soon".to_string(),
        };

        // Set with 1 second TTL
        cache.set(key, &data, Duration::from_millis(100)).await.unwrap();
        
        // Should exist immediately
        let retrieved: Option<TestData> = cache.get(key).await.unwrap();
        assert_eq!(retrieved, Some(data.clone()));

        // Wait for expiry
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should be expired
        let expired: Option<TestData> = cache.get(key).await.unwrap();
        assert_eq!(expired, None);
    }

    #[tokio::test]
    async fn test_clear() {
        let cache = SharedCache::new(None);

        cache.set("key1", &TestData { value: "val1".to_string() }, Duration::from_secs(60)).await.unwrap();
        cache.set("key2", &TestData { value: "val2".to_string() }, Duration::from_secs(60)).await.unwrap();

        cache.clear().await.unwrap();

        let val1: Option<TestData> = cache.get("key1").await.unwrap();
        let val2: Option<TestData> = cache.get("key2").await.unwrap();
        
        assert_eq!(val1, None);
        assert_eq!(val2, None);
    }
}
