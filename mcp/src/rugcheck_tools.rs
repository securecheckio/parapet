//! Rugcheck tool implementations for MCP
//!
//! Uses the enrichment service to provide token security analysis

use anyhow::Result;
use serde_json::{json, Value};

#[cfg(feature = "reqwest")]
use parapet_core::enrichment::RugcheckClient;

pub async fn check_token_security(mint_address: &str) -> Result<Value> {
    check_token_security_with_client(mint_address, None).await
}

pub async fn check_insider_risk(mint_address: &str) -> Result<Value> {
    check_insider_risk_with_client(mint_address, None).await
}

pub async fn check_liquidity_lock(mint_address: &str) -> Result<Value> {
    check_liquidity_lock_with_client(mint_address, None).await
}

// Internal functions that accept optional client for testing
#[cfg(feature = "reqwest")]
async fn check_token_security_with_client(
    mint_address: &str,
    client: Option<RugcheckClient>,
) -> Result<Value> {
    let client = client.unwrap_or_else(|| RugcheckClient::new());
    let data = client.get_token_data(mint_address).await?;

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&data)?
        }]
    }))
}

#[cfg(not(feature = "reqwest"))]
async fn check_token_security_with_client(
    _mint_address: &str,
    _client: Option<()>,
) -> Result<Value> {
    Err(anyhow::anyhow!(
        "Rugcheck features not enabled. Rebuild with --features reqwest"
    ))
}

#[cfg(feature = "reqwest")]
async fn check_insider_risk_with_client(
    mint_address: &str,
    client: Option<RugcheckClient>,
) -> Result<Value> {
    let client = client.unwrap_or_else(|| RugcheckClient::new());
    let data = client.get_insider_analysis(mint_address).await?;

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&data)?
        }]
    }))
}

#[cfg(not(feature = "reqwest"))]
async fn check_insider_risk_with_client(_mint_address: &str, _client: Option<()>) -> Result<Value> {
    Err(anyhow::anyhow!(
        "Rugcheck features not enabled. Rebuild with --features reqwest"
    ))
}

#[cfg(feature = "reqwest")]
async fn check_liquidity_lock_with_client(
    mint_address: &str,
    client: Option<RugcheckClient>,
) -> Result<Value> {
    let client = client.unwrap_or_else(|| RugcheckClient::new());
    let data = client.get_vault_analysis(mint_address).await?;

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&data)?
        }]
    }))
}

#[cfg(not(feature = "reqwest"))]
async fn check_liquidity_lock_with_client(
    _mint_address: &str,
    _client: Option<()>,
) -> Result<Value> {
    Err(anyhow::anyhow!(
        "Rugcheck features not enabled. Rebuild with --features reqwest"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_token_security() {
        #[cfg(feature = "reqwest")]
        {
            let mut server = mockito::Server::new_async().await;
            let mock_url = server.url();

            let mock_response = serde_json::json!({
                "risks": [
                    {
                        "name": "Test Risk",
                        "description": "A test risk",
                        "level": "warning",
                        "score": 10
                    }
                ],
                "market_cap": 1000000.0,
                "top_holders_percentage": 50.0,
                "liquidity": 500000.0,
                "token_age_days": 30
            });

            let _mock = server
                .mock("GET", "/v1/tokens/test123/report")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(mock_response.to_string())
                .create_async()
                .await;

            let client = RugcheckClient::new_with_base_url(mock_url);
            let result = check_token_security_with_client("test123", Some(client)).await;

            assert!(result.is_ok());
            let value = result.unwrap();
            assert!(value.get("content").is_some());
        }

        #[cfg(not(feature = "reqwest"))]
        {
            let result = check_token_security("any_address").await;
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.to_string().contains("not enabled"));
        }
    }

    #[tokio::test]
    async fn test_check_insider_risk() {
        #[cfg(feature = "reqwest")]
        {
            let mut server = mockito::Server::new_async().await;
            let mock_url = server.url();

            let mock_response = serde_json::json!({
                "trade_networks": 2,
                "transfer_networks": 1,
                "total_insiders": 10,
                "insider_concentration": 45.5
            });

            let _mock = server
                .mock("GET", "/v1/tokens/test123/insiders/networks")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(mock_response.to_string())
                .create_async()
                .await;

            let client = RugcheckClient::new_with_base_url(mock_url);
            let result = check_insider_risk_with_client("test123", Some(client)).await;

            assert!(result.is_ok());
            let value = result.unwrap();
            assert!(value.get("content").is_some());
        }

        #[cfg(not(feature = "reqwest"))]
        {
            let result = check_insider_risk("any_address").await;
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    async fn test_check_liquidity_lock() {
        #[cfg(feature = "reqwest")]
        {
            let mut server = mockito::Server::new_async().await;
            let mock_url = server.url();

            let mock_response = serde_json::json!({
                "lockers": [
                    {
                        "locker_type": "streamflow",
                        "locked_amount": 100000.0,
                        "unlock_date": "2025-12-31",
                        "percentage_of_supply": 80.0
                    }
                ]
            });

            let _mock = server
                .mock("GET", "/v1/tokens/test123/lockers")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(mock_response.to_string())
                .create_async()
                .await;

            let client = RugcheckClient::new_with_base_url(mock_url);
            let result = check_liquidity_lock_with_client("test123", Some(client)).await;

            assert!(result.is_ok());
            let value = result.unwrap();
            assert!(value.get("content").is_some());
        }

        #[cfg(not(feature = "reqwest"))]
        {
            let result = check_liquidity_lock("any_address").await;
            assert!(result.is_err());
        }
    }
}
