use anyhow::Result;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const SESSION_PREFIX: &str = "session:";
const SESSION_EXPIRY_SECONDS: u64 = 86400; // 24 hours
const SESSION_EXPIRY_SECONDS_I64: i64 = 86400; // For Redis i64 API

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub user_id: String,
    pub wallet_address: String,
}

#[derive(Clone)]
pub struct SessionStore {
    redis_url: String,
}

impl SessionStore {
    pub fn new(redis_url: String) -> Self {
        Self { redis_url }
    }

    async fn get_connection(&self) -> Result<redis::aio::MultiplexedConnection> {
        let client = redis::Client::open(self.redis_url.as_str())?;
        let conn = client.get_multiplexed_async_connection().await?;
        Ok(conn)
    }

    pub async fn create_session(&self, user_id: Uuid, wallet_address: &str) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();
        let session = Session {
            session_id: session_id.clone(),
            user_id: user_id.to_string(),
            wallet_address: wallet_address.to_string(),
        };

        let session_json = serde_json::to_string(&session)?;
        let key = format!("{}{}", SESSION_PREFIX, session_id);

        let mut conn = self.get_connection().await?;
        conn.set_ex::<_, _, ()>(&key, session_json, SESSION_EXPIRY_SECONDS).await?;

        log::info!("✅ Session created: {} for user {}", session_id, user_id);
        Ok(session_id)
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Option<Session>> {
        let key = format!("{}{}", SESSION_PREFIX, session_id);
        let mut conn = self.get_connection().await?;
        
        let session_json: Option<String> = conn.get(&key).await?;
        
        match session_json {
            Some(json) => {
                let session: Session = serde_json::from_str(&json)?;
                Ok(Some(session))
            }
            None => Ok(None),
        }
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        let key = format!("{}{}", SESSION_PREFIX, session_id);
        let mut conn = self.get_connection().await?;
        conn.del::<_, ()>(&key).await?;
        log::info!("🗑️  Session deleted: {}", session_id);
        Ok(())
    }

    pub async fn refresh_session(&self, session_id: &str) -> Result<bool> {
        let key = format!("{}{}", SESSION_PREFIX, session_id);
        let mut conn = self.get_connection().await?;
        let refreshed: bool = conn.expire::<_, bool>(&key, SESSION_EXPIRY_SECONDS_I64).await?;
        Ok(refreshed)
    }
}
