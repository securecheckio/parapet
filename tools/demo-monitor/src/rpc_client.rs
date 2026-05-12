use crate::types::ParapetResult;
use anyhow::Result;
use serde_json::{json, Value};
use std::time::Instant;

/// Fetch real transaction and analyze it via Parapet RPC Proxy
pub async fn simulate_via_parapet(
    transaction_signature: &str,
    rpc_proxy_url: &str,
) -> Result<ParapetResult> {
    let start = Instant::now();
    let client = reqwest::Client::new();

    log::info!(
        "Step 1: Fetching transaction {} via RPC proxy",
        transaction_signature
    );

    // Step 1: Fetch real transaction (proxy passes through to Solana)
    let get_tx_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTransaction",
        "params": [
            transaction_signature,
            {
                "encoding": "base64",
                "maxSupportedTransactionVersion": 0
            }
        ]
    });

    let fetch_response = client
        .post(rpc_proxy_url)
        .json(&get_tx_request)
        .send()
        .await?;

    let tx_data: Value = fetch_response.json().await?;

    // Check if transaction exists
    if tx_data["result"].is_null() {
        return Err(anyhow::anyhow!(
            "Transaction not found on Solana (signature: {})",
            transaction_signature
        ));
    }

    // Extract transaction bytes
    let tx_bytes = tx_data["result"]["transaction"][0]
        .as_str()
        .ok_or_else(|| {
            anyhow::anyhow!("Failed to extract transaction bytes from Solana response")
        })?;

    log::info!("Step 2: Sending transaction bytes to Parapet RPC proxy for analysis via simulateTransaction");

    // Step 2: Send transaction bytes to RPC proxy for REAL analysis
    let simulate_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "simulateTransaction",
        "params": [
            tx_bytes,
            {
                "encoding": "base64",
                "commitment": "finalized"
            }
        ]
    });

    let analyze_response = client
        .post(rpc_proxy_url)
        .json(&simulate_request)
        .send()
        .await?;

    let analysis_time = start.elapsed().as_secs_f64() * 1000.0;

    let result: Value = analyze_response.json().await?;

    log::info!(
        "RPC analysis response: {}",
        serde_json::to_string_pretty(&result).unwrap_or_else(|_| format!("{:?}", result))
    );

    // Extract Parapet analysis from response body
    // Try both locations: result.value.parapet (simulation) and result.parapet (direct)
    let parapet_data = result
        .get("result")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.get("parapet"))
        .or_else(|| result.get("result").and_then(|r| r.get("parapet")))
        .cloned();

    log::info!("Parapet data found: {}", parapet_data.is_some());

    if let Some(parapet) = parapet_data {
        let decision = parapet
            .get("decision")
            .and_then(|d| d.as_str())
            .unwrap_or("safe");
        let action = if decision == "would_block" {
            "block"
        } else if decision == "alert" {
            "alert"
        } else {
            "pass"
        };

        let risk_score = parapet
            .get("riskScore")
            .and_then(|r| r.as_u64())
            .unwrap_or(0) as u32;

        let empty_warnings = vec![];
        let warnings = parapet
            .get("warnings")
            .and_then(|w| w.as_array())
            .unwrap_or(&empty_warnings);
        let rules_triggered: Vec<String> = warnings
            .iter()
            .filter_map(|w| {
                w.get("ruleId")
                    .and_then(|id| id.as_str())
                    .map(|s| s.to_string())
            })
            .collect();

        let message = warnings
            .iter()
            .filter_map(|w| w.get("message").and_then(|m| m.as_str()))
            .collect::<Vec<_>>()
            .join(". ");

        let final_message = if message.is_empty() {
            "Transaction analyzed".to_string()
        } else {
            message
        };

        // Extract analyzer_fields if present
        let analyzer_fields = parapet
            .get("analyzerFields")
            .and_then(|f| f.as_object())
            .cloned()
            .unwrap_or_default();

        log::info!(
            "Parapet analysis: decision={}, risk_score={}, rules={:?}",
            decision,
            risk_score,
            rules_triggered
        );

        Ok(ParapetResult {
            action: action.to_string(),
            risk_score,
            rules_triggered,
            message: final_message,
            analysis_time_ms: analysis_time,
            analyzer_fields,
        })
    } else {
        log::warn!("No Parapet analysis in response - rules may not have matched");
        Ok(ParapetResult {
            action: "pass".to_string(),
            risk_score: 0,
            rules_triggered: vec![],
            message: "No security rules triggered".to_string(),
            analysis_time_ms: analysis_time,
            analyzer_fields: serde_json::Map::new(),
        })
    }
}
