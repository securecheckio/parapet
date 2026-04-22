use anyhow::Result;
use async_trait::async_trait;
use axum::http::HeaderMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod providers;

#[cfg(test)]
mod tests;

/// Authentication context for an authenticated request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    /// Unique identifier (user ID, wallet address, etc.)
    pub identity: String,

    /// Optional: wallet addresses this identity controls
    pub wallets: Vec<String>,

    /// Optional: permissions/scopes
    pub scopes: Vec<String>,

    /// Optional: tier/plan (e.g., "free", "pro", "enterprise")
    pub tier: Option<String>,

    /// Arbitrary metadata for custom use
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl AuthContext {
    /// Create anonymous context (no auth)
    pub fn anonymous() -> Self {
        Self {
            identity: "anonymous".to_string(),
            wallets: vec![],
            scopes: vec![],
            tier: None,
            metadata: HashMap::new(),
        }
    }

    /// Check if identity owns a wallet
    pub fn owns_wallet(&self, wallet: &str) -> bool {
        self.wallets.iter().any(|w| w == wallet)
    }

    /// Check if has specific scope/permission
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope)
    }
}

/// Authentication result with optional rate limit info
#[derive(Debug, Clone)]
pub struct AuthResult {
    pub context: AuthContext,

    /// Optional: remaining quota/requests
    pub quota_remaining: Option<u64>,

    /// Optional: quota resets at (unix timestamp)
    pub quota_reset: Option<i64>,
}

impl AuthResult {
    pub fn success(context: AuthContext) -> Self {
        Self {
            context,
            quota_remaining: None,
            quota_reset: None,
        }
    }

    /// Add quota information to auth result (for rate-limited APIs)
    #[allow(dead_code)]
    pub fn with_quota(mut self, remaining: u64, reset: i64) -> Self {
        self.quota_remaining = Some(remaining);
        self.quota_reset = Some(reset);
        self
    }
}

/// Auth provider trait - implement this for custom auth
#[async_trait]
pub trait AuthProvider: Send + Sync {
    /// Authenticate a request
    ///
    /// Returns Ok(AuthResult) if authenticated, Err if rejected
    async fn authenticate(
        &self,
        headers: &HeaderMap,
        method: &str, // RPC method (e.g., "sendTransaction")
    ) -> Result<AuthResult>;

    /// Optional: called after successful request (for usage tracking, billing)
    async fn on_success(
        &self,
        _context: &AuthContext,
        _method: &str,
        _response_status: u16,
    ) -> Result<()> {
        Ok(())
    }

    /// Optional: called after failed request (for logging, alerts)
    async fn on_failure(
        &self,
        context: Option<&AuthContext>,
        method: &str,
        error: &str,
    ) -> Result<()> {
        let _ = (context, method, error);
        Ok(())
    }

    /// Provider name (for logging/metrics)
    fn name(&self) -> &str {
        "unknown"
    }
}
