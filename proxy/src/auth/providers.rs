use super::*;
use anyhow::anyhow;

/// No authentication (default, backwards compatible)
pub struct NoAuth;

#[async_trait]
impl AuthProvider for NoAuth {
    async fn authenticate(&self, _: &HeaderMap, _: &str) -> Result<AuthResult> {
        Ok(AuthResult::success(AuthContext::anonymous()))
    }

    fn name(&self) -> &str {
        "none"
    }
}

/// Simple API key authentication from environment or HashMap
///
/// Format: API_KEYS=key1:user1,key2:user2
pub struct ApiKeyAuth {
    keys: HashMap<String, UserInfo>,
}

#[derive(Debug, Clone)]
struct UserInfo {
    user_id: String,
    wallets: Vec<String>,
    tier: String,
}

impl ApiKeyAuth {
    /// Create ApiKeyAuth from a config string (for testing and direct usage)
    pub fn from_str(keys_str: &str) -> Result<Self> {
        let mut keys = HashMap::new();

        // Determine separator: if string contains '|', use | (new format with wallets)
        // Otherwise use , (old format for backwards compatibility)
        let separator = if keys_str.contains('|') { '|' } else { ',' };

        for pair in keys_str.split(separator) {
            let pair = pair.trim();
            if pair.is_empty() {
                continue;
            }

            let mut parts = pair.split(':');
            let key = parts.next().map(|s| s.trim().to_string());
            let user = parts.next().map(|s| s.trim().to_string());
            let wallets_str = parts.next().map(|s| s.trim());

            if let (Some(key), Some(user_id)) = (key, user) {
                if !key.is_empty() && !user_id.is_empty() {
                    // Parse wallets if provided
                    let wallets = if let Some(w_str) = wallets_str {
                        w_str
                            .split(',')
                            .map(|w| w.trim().to_string())
                            .filter(|w| !w.is_empty())
                            .collect()
                    } else {
                        vec![]
                    };

                    keys.insert(
                        key,
                        UserInfo {
                            user_id,
                            wallets,
                            tier: "basic".to_string(),
                        },
                    );
                }
            }
        }

        Ok(Self { keys })
    }

    /// Load from environment variable API_KEYS
    ///
    /// Format: key:userid[:wallet1,wallet2,...]
    /// Separators:
    /// - Use | to separate multiple API keys when wallets are included
    /// - Use , for backwards compatibility when no wallets
    /// Examples:
    /// - Simple: "key1:user1,key2:user2" (backwards compatible)
    /// - With wallets: "key1:user1:wallet1,wallet2|key2:user2:wallet3"
    pub fn from_env() -> Result<Self> {
        let keys_str = std::env::var("API_KEYS").unwrap_or_default();
        Self::from_str(&keys_str)
    }

    /// Number of configured keys
    pub fn key_count(&self) -> usize {
        self.keys.len()
    }
}

#[async_trait]
impl AuthProvider for ApiKeyAuth {
    async fn authenticate(&self, headers: &HeaderMap, _method: &str) -> Result<AuthResult> {
        // Try Authorization: Bearer {key} first (recommended)
        let key = if let Some(auth_header) = headers.get("Authorization") {
            auth_header
                .to_str()
                .ok()
                .and_then(|h| h.strip_prefix("Bearer ").or(Some(h)))
        } else {
            // Fall back to X-API-Key header (alternative)
            headers.get("X-API-Key").and_then(|h| h.to_str().ok())
        };

        let key = key.ok_or_else(|| {
            anyhow!("Missing API key (use Authorization: Bearer header or X-API-Key header)")
        })?;

        // Lookup user
        let user_info = self
            .keys
            .get(key)
            .ok_or_else(|| anyhow!("Invalid API key"))?;

        Ok(AuthResult::success(AuthContext {
            identity: user_info.user_id.clone(),
            wallets: user_info.wallets.clone(),
            scopes: vec!["rpc:*".to_string()],
            tier: Some(user_info.tier.clone()),
            metadata: HashMap::new(),
        }))
    }

    fn name(&self) -> &str {
        "api_key"
    }
}

/// Wallet allowlist authentication
///
/// Format: ALLOWED_WALLETS=wallet1,wallet2,wallet3
pub struct WalletAllowlist {
    allowed: Vec<String>,
}

impl WalletAllowlist {
    /// Load from environment variable ALLOWED_WALLETS
    pub fn from_env() -> Result<Self> {
        let wallets_str = std::env::var("ALLOWED_WALLETS").unwrap_or_default();

        let allowed = wallets_str
            .split(',')
            .map(|w| w.trim().to_string())
            .filter(|w| !w.is_empty())
            .collect();

        Ok(Self { allowed })
    }

    /// Number of allowed wallets
    pub fn wallet_count(&self) -> usize {
        self.allowed.len()
    }
}

#[async_trait]
impl AuthProvider for WalletAllowlist {
    async fn authenticate(&self, headers: &HeaderMap, _method: &str) -> Result<AuthResult> {
        // Extract wallet from X-Wallet-Address header
        let wallet = headers
            .get("X-Wallet-Address")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| anyhow!("Missing X-Wallet-Address header"))?;

        if !self.allowed.iter().any(|w| w == wallet) {
            return Err(anyhow!("Wallet not on allowlist"));
        }

        Ok(AuthResult::success(AuthContext {
            identity: wallet.to_string(),
            wallets: vec![wallet.to_string()],
            scopes: vec!["rpc:*".to_string()],
            tier: Some("allowlist".to_string()),
            metadata: HashMap::new(),
        }))
    }

    fn name(&self) -> &str {
        "wallet_allowlist"
    }
}
