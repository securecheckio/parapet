#[cfg(test)]
mod tests {
    use super::super::providers::*;
    use super::super::*;
    use axum::http::HeaderMap;

    #[tokio::test]
    async fn test_no_auth_always_succeeds() {
        let auth = NoAuth;
        let headers = HeaderMap::new();

        let result = auth.authenticate(&headers, "getHealth").await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.context.identity, "anonymous");
        assert_eq!(result.context.wallets.len(), 0);
    }

    #[tokio::test]
    async fn test_api_key_auth_simple_format() {
        let auth = ApiKeyAuth::from_str("test_key:user123|prod_key:user456").unwrap();
        assert_eq!(auth.key_count(), 2);

        let mut headers = HeaderMap::new();
        headers.insert("Authorization", "Bearer test_key".parse().unwrap());

        let result = auth.authenticate(&headers, "getHealth").await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.context.identity, "user123");
        assert_eq!(result.context.wallets.len(), 0); // No wallets in simple format
    }

    #[tokio::test]
    async fn test_api_key_auth_with_wallets() {
        let auth = ApiKeyAuth::from_str("key1:user1:wallet1,wallet2|key2:user2:wallet3").unwrap();
        assert_eq!(auth.key_count(), 2);

        let mut headers = HeaderMap::new();
        headers.insert("Authorization", "Bearer key1".parse().unwrap());

        let result = auth.authenticate(&headers, "sendTransaction").await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.context.identity, "user1");
        assert_eq!(result.context.wallets.len(), 2);
        assert!(result.context.owns_wallet("wallet1"));
        assert!(result.context.owns_wallet("wallet2"));
        assert!(!result.context.owns_wallet("wallet3"));
    }

    #[tokio::test]
    async fn test_api_key_auth_invalid_key() {
        let auth = ApiKeyAuth::from_str("valid_key:user1").unwrap();

        let mut headers = HeaderMap::new();
        headers.insert("Authorization", "Bearer invalid_key".parse().unwrap());

        let result = auth.authenticate(&headers, "getHealth").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid API key"));
    }

    #[tokio::test]
    async fn test_api_key_auth_missing_header() {
        let auth = ApiKeyAuth::from_str("key:user").unwrap();
        let headers = HeaderMap::new();

        let result = auth.authenticate(&headers, "getHealth").await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing API key"));
    }

    #[tokio::test]
    async fn test_wallet_allowlist() {
        std::env::remove_var("ALLOWED_WALLETS");
        std::env::set_var("ALLOWED_WALLETS", "wallet1,wallet2,wallet3");

        let auth = WalletAllowlist::from_env().unwrap();
        assert_eq!(auth.wallet_count(), 3);

        let mut headers = HeaderMap::new();
        headers.insert("X-Wallet-Address", "wallet1".parse().unwrap());

        let result = auth.authenticate(&headers, "sendTransaction").await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.context.identity, "wallet1");
        assert!(result.context.owns_wallet("wallet1"));

        std::env::remove_var("ALLOWED_WALLETS");
    }

    #[tokio::test]
    async fn test_wallet_allowlist_not_allowed() {
        std::env::set_var("ALLOWED_WALLETS", "wallet1,wallet2");

        let auth = WalletAllowlist::from_env().unwrap();

        let mut headers = HeaderMap::new();
        headers.insert("X-Wallet-Address", "wallet3".parse().unwrap());

        let result = auth.authenticate(&headers, "sendTransaction").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not on allowlist"));

        std::env::remove_var("ALLOWED_WALLETS");
    }

    #[test]
    fn test_auth_context_owns_wallet() {
        let ctx = AuthContext {
            identity: "user1".to_string(),
            wallets: vec!["wallet1".to_string(), "wallet2".to_string()],
            scopes: vec![],
            tier: None,
            metadata: HashMap::new(),
        };

        assert!(ctx.owns_wallet("wallet1"));
        assert!(ctx.owns_wallet("wallet2"));
        assert!(!ctx.owns_wallet("wallet3"));
    }

    #[test]
    fn test_auth_context_has_scope() {
        let ctx = AuthContext {
            identity: "user1".to_string(),
            wallets: vec![],
            scopes: vec!["rpc:read".to_string(), "admin".to_string()],
            tier: None,
            metadata: HashMap::new(),
        };

        assert!(ctx.has_scope("rpc:read"));
        assert!(ctx.has_scope("admin"));
        assert!(!ctx.has_scope("rpc:write"));
    }

    #[test]
    fn test_auth_context_anonymous() {
        let ctx = AuthContext::anonymous();

        assert_eq!(ctx.identity, "anonymous");
        assert_eq!(ctx.wallets.len(), 0);
        assert_eq!(ctx.scopes.len(), 0);
        assert!(ctx.tier.is_none());
    }
}
