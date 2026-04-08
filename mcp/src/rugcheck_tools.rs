//! Rugcheck tool implementations for MCP
//! 
//! Uses the enrichment service to provide token security analysis

use anyhow::Result;
use serde_json::{json, Value};

#[cfg(feature = "reqwest")]
use parapet_core::enrichment::{EnrichmentService, RugcheckClient};

pub async fn check_token_security(mint_address: &str) -> Result<Value> {
    #[cfg(feature = "reqwest")]
    {
        let client = RugcheckClient::new();
        let data = client.get_token_data(mint_address).await?;
        
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&data)?
            }]
        }))
    }
    
    #[cfg(not(feature = "reqwest"))]
    {
        Err(anyhow::anyhow!("Rugcheck features not enabled. Rebuild with --features reqwest"))
    }
}

pub async fn check_insider_risk(mint_address: &str) -> Result<Value> {
    #[cfg(feature = "reqwest")]
    {
        let client = RugcheckClient::new();
        let data = client.get_insider_analysis(mint_address).await?;
        
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&data)?
            }]
        }))
    }
    
    #[cfg(not(feature = "reqwest"))]
    {
        Err(anyhow::anyhow!("Rugcheck features not enabled. Rebuild with --features reqwest"))
    }
}

pub async fn check_liquidity_lock(mint_address: &str) -> Result<Value> {
    #[cfg(feature = "reqwest")]
    {
        let client = RugcheckClient::new();
        let data = client.get_vault_analysis(mint_address).await?;
        
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&data)?
            }]
        }))
    }
    
    #[cfg(not(feature = "reqwest"))]
    {
        Err(anyhow::anyhow!("Rugcheck features not enabled. Rebuild with --features reqwest"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_check_token_security() {
        // Test with SOL
        let result = check_token_security("So11111111111111111111111111111111111111112").await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_check_insider_risk() {
        let result = check_insider_risk("So11111111111111111111111111111111111111112").await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_check_liquidity_lock() {
        let result = check_liquidity_lock("So11111111111111111111111111111111111111112").await;
        assert!(result.is_ok());
    }
}
