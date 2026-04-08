use anyhow::Result;
use redis::AsyncCommands;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

pub enum CacheBackend {
    Redis(redis::Client),
    InMemory(Arc<RwLock<InMemoryCache>>),
}

pub struct InMemoryCache {
    blocklist: HashSet<Pubkey>,
    allowlist: HashSet<Pubkey>,
}

pub struct Cache {
    pub backend: CacheBackend,
}

impl Cache {
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;

        // Test connection
        let mut conn = client.get_multiplexed_async_connection().await?;
        let _: String = redis::cmd("PING").query_async(&mut conn).await?;

        Ok(Self {
            backend: CacheBackend::Redis(client),
        })
    }

    pub fn new_in_memory() -> Result<Self> {
        Ok(Self {
            backend: CacheBackend::InMemory(Arc::new(RwLock::new(InMemoryCache {
                blocklist: HashSet::new(),
                allowlist: HashSet::new(),
            }))),
        })
    }

    #[allow(dead_code)]
    pub async fn is_allowed(&self, program_id: &Pubkey) -> Result<bool> {
        match &self.backend {
            CacheBackend::Redis(client) => {
                let mut conn = client.get_multiplexed_async_connection().await?;
                let key = format!("allowlist:{}", program_id);
                let exists: bool = conn.exists(&key).await?;
                Ok(exists)
            }
            CacheBackend::InMemory(cache) => {
                let cache = cache.read().await;
                Ok(cache.allowlist.contains(program_id))
            }
        }
    }

    /// Add program to blocklist (operational API)
    #[allow(dead_code)]
    pub async fn add_to_blocklist(&self, program_id: &Pubkey) -> Result<()> {
        match &self.backend {
            CacheBackend::Redis(client) => {
                let mut conn = client.get_multiplexed_async_connection().await?;
                let key = format!("blocklist:{}", program_id);
                conn.set::<_, _, ()>(&key, "1").await?;
                Ok(())
            }
            CacheBackend::InMemory(cache) => {
                let mut cache = cache.write().await;
                cache.blocklist.insert(*program_id);
                Ok(())
            }
        }
    }

    /// Remove program from blocklist (operational API)
    #[allow(dead_code)]
    pub async fn remove_from_blocklist(&self, program_id: &Pubkey) -> Result<()> {
        match &self.backend {
            CacheBackend::Redis(client) => {
                let mut conn = client.get_multiplexed_async_connection().await?;
                let key = format!("blocklist:{}", program_id);
                conn.del::<_, ()>(&key).await?;
                Ok(())
            }
            CacheBackend::InMemory(cache) => {
                let mut cache = cache.write().await;
                cache.blocklist.remove(program_id);
                Ok(())
            }
        }
    }
}
