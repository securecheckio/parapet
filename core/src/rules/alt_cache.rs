/// Cache for Address Lookup Table data to avoid repeated RPC calls
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Clone)]
struct CachedAlt {
    data: Vec<u8>,
    cached_at: Instant,
}

pub struct AltCache {
    cache: Arc<RwLock<HashMap<String, CachedAlt>>>,
    ttl: Duration,
}

impl AltCache {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    /// Get cached ALT data if available and not expired
    pub async fn get(&self, pubkey: &str) -> Option<Vec<u8>> {
        let cache = self.cache.read().await;
        if let Some(cached) = cache.get(pubkey) {
            if cached.cached_at.elapsed() < self.ttl {
                return Some(cached.data.clone());
            }
        }
        None
    }

    /// Get multiple cached ALT data entries
    pub async fn get_multiple(&self, pubkeys: &[String]) -> Vec<Option<Vec<u8>>> {
        let cache = self.cache.read().await;
        let now = Instant::now();

        pubkeys
            .iter()
            .map(|pubkey| {
                cache.get(pubkey).and_then(|cached| {
                    if now.duration_since(cached.cached_at) < self.ttl {
                        Some(cached.data.clone())
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    /// Store ALT data in cache
    pub async fn set(&self, pubkey: String, data: Vec<u8>) {
        let mut cache = self.cache.write().await;
        cache.insert(
            pubkey,
            CachedAlt {
                data,
                cached_at: Instant::now(),
            },
        );
    }

    /// Store multiple ALT data entries
    pub async fn set_multiple(&self, entries: Vec<(String, Vec<u8>)>) {
        let mut cache = self.cache.write().await;
        let now = Instant::now();

        for (pubkey, data) in entries {
            cache.insert(
                pubkey,
                CachedAlt {
                    data,
                    cached_at: now,
                },
            );
        }
    }

    /// Clear expired entries
    pub async fn cleanup(&self) {
        let mut cache = self.cache.write().await;
        let now = Instant::now();

        cache.retain(|_, cached| now.duration_since(cached.cached_at) < self.ttl);
    }

    /// Get cache statistics
    pub async fn stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        let total = cache.len();
        let now = Instant::now();
        let valid = cache
            .values()
            .filter(|cached| now.duration_since(cached.cached_at) < self.ttl)
            .count();

        (total, valid)
    }
}

impl Default for AltCache {
    fn default() -> Self {
        // Default: 1 hour TTL (ALTs are relatively static)
        Self::new(3600)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_basic() {
        let cache = AltCache::new(1);

        cache.set("test".to_string(), vec![1, 2, 3]).await;
        assert_eq!(cache.get("test").await, Some(vec![1, 2, 3]));
    }

    #[tokio::test]
    async fn test_cache_expiry() {
        let cache = AltCache::new(1);

        cache.set("test".to_string(), vec![1, 2, 3]).await;
        tokio::time::sleep(Duration::from_secs(2)).await;
        assert_eq!(cache.get("test").await, None);
    }

    #[tokio::test]
    async fn test_cache_multiple() {
        let cache = AltCache::new(10);

        cache.set("key1".to_string(), vec![1]).await;
        cache.set("key2".to_string(), vec![2]).await;

        let keys = vec!["key1".to_string(), "key2".to_string(), "key3".to_string()];
        let results = cache.get_multiple(&keys).await;

        assert_eq!(results, vec![Some(vec![1]), Some(vec![2]), None]);
    }
}
