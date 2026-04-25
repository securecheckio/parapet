//! Helius API Tools
//!
//! Tools for querying Helius Wallet History API and RPC methods

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HeliusTransaction {
    pub description: String,
    pub r#type: String,
    pub source: String,
    pub fee: u64,
    pub fee_payer: String,
    pub signature: String,
    pub slot: u64,
    pub timestamp: i64,
    pub native_transfers: Option<Vec<NativeTransfer>>,
    pub token_transfers: Option<Vec<TokenTransfer>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeTransfer {
    pub from_user_account: String,
    pub to_user_account: String,
    pub amount: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenTransfer {
    pub from_user_account: Option<String>,
    pub to_user_account: Option<String>,
    pub token_amount: f64,
    pub mint: String,
}

#[derive(Debug, Deserialize)]
struct TokenAccount {
    pub pubkey: String,
    pub account: TokenAccountData,
}

#[derive(Debug, Deserialize)]
struct TokenAccountData {
    pub data: TokenAccountParsed,
}

#[derive(Debug, Deserialize)]
struct TokenAccountParsed {
    pub parsed: TokenAccountInfo,
}

#[derive(Debug, Deserialize)]
struct TokenAccountInfo {
    pub info: TokenInfo,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenInfo {
    pub is_native: bool,
    pub mint: String,
    pub owner: String,
    pub state: String,
    pub token_amount: TokenAmount,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenAmount {
    pub amount: String,
    pub decimals: u8,
    pub ui_amount_string: String,
}

/// Get wallet transaction history from Helius API
pub async fn get_wallet_history(
    wallet_address: &str,
    transaction_type: Option<&str>,
    limit: Option<usize>,
    before_cursor: Option<&str>,
) -> Result<Value> {
    let api_key = std::env::var("HELIUS_API_KEY")
        .map_err(|_| anyhow::anyhow!("HELIUS_API_KEY environment variable not set"))?;

    let limit = limit.unwrap_or(100).min(100); // Max 100 per request
    let mut url = format!(
        "https://api.helius.xyz/v0/addresses/{}/transactions?api-key={}",
        wallet_address, api_key
    );

    // Add optional parameters
    if let Some(tx_type) = transaction_type {
        url.push_str(&format!("&type={}", tx_type));
    }

    if let Some(cursor) = before_cursor {
        url.push_str(&format!("&before={}", cursor));
    }

    url.push_str(&format!("&limit={}", limit));

    log::info!("Fetching wallet history from Helius: {}", wallet_address);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch wallet history: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "Helius API error ({}): {}",
            status,
            error_text
        ));
    }

    let response_data: Vec<HeliusTransaction> = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse Helius response: {}", e))?;

    // Format output
    let mut output = String::new();
    output.push_str(&format!(
        "# Wallet Transaction History: {}\n\n",
        wallet_address
    ));
    output.push_str(&format!(
        "**Transactions Found:** {}\n",
        response_data.len()
    ));

    if let Some(tx_type) = transaction_type {
        output.push_str(&format!("**Filtered by Type:** {}\n", tx_type));
    }
    output.push_str("\n");

    if response_data.is_empty() {
        output.push_str("No transactions found.\n");
    } else {
        output.push_str("## Transactions\n\n");

        for (i, tx) in response_data.iter().enumerate() {
            output.push_str(&format!("### {}. {}\n", i + 1, tx.r#type));
            output.push_str(&format!("- **Description:** {}\n", tx.description));
            output.push_str(&format!("- **Signature:** `{}`\n", tx.signature));
            output.push_str(&format!("- **Slot:** {}\n", tx.slot));
            output.push_str(&format!(
                "- **Timestamp:** {} ({})\n",
                tx.timestamp,
                chrono::DateTime::from_timestamp(tx.timestamp, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| "Invalid timestamp".to_string())
            ));
            output.push_str(&format!("- **Fee:** {} lamports\n", tx.fee));
            output.push_str(&format!("- **Fee Payer:** `{}`\n", tx.fee_payer));
            output.push_str(&format!("- **Source:** {}\n", tx.source));

            // Show native transfers if present
            if let Some(ref transfers) = tx.native_transfers {
                if !transfers.is_empty() {
                    output.push_str("\n**Native Transfers:**\n");
                    for transfer in transfers {
                        output.push_str(&format!(
                            "  - {} SOL: `{}` → `{}`\n",
                            transfer.amount as f64 / 1_000_000_000.0,
                            transfer.from_user_account,
                            transfer.to_user_account
                        ));
                    }
                }
            }

            // Show token transfers if present
            if let Some(ref transfers) = tx.token_transfers {
                if !transfers.is_empty() {
                    output.push_str("\n**Token Transfers:**\n");
                    for transfer in transfers {
                        let from = transfer.from_user_account.as_deref().unwrap_or("Unknown");
                        let to = transfer.to_user_account.as_deref().unwrap_or("Unknown");
                        output.push_str(&format!(
                            "  - {} tokens (mint: `{}`): `{}` → `{}`\n",
                            transfer.token_amount, transfer.mint, from, to
                        ));
                    }
                }
            }

            output.push_str("\n");
        }

        // Pagination info
        output.push_str("## Pagination\n");
        if response_data.len() >= limit {
            let last_sig = &response_data.last().unwrap().signature;
            output.push_str(&format!(
                "More transactions may be available. Use `before_cursor: \"{}\"` to fetch next page.\n",
                last_sig
            ));
        } else {
            output.push_str("All available transactions fetched.\n");
        }
    }

    Ok(json!({
        "content": [{
            "type": "text",
            "text": output
        }]
    }))
}

/// Get token accounts by delegate using Helius RPC
pub async fn get_token_accounts_by_delegate(
    delegate_address: &str,
    program_id: Option<&str>,
) -> Result<Value> {
    let api_key = std::env::var("HELIUS_API_KEY")
        .map_err(|_| anyhow::anyhow!("HELIUS_API_KEY environment variable not set"))?;

    let rpc_url = format!("https://mainnet.helius-rpc.com/?api-key={}", api_key);
    let program_id = program_id.unwrap_or("TokenkegQfeZyiNwAJbNbGKPFXCwuBvf9Ss623VQ5DA");

    log::info!(
        "Fetching token accounts by delegate: {} (program: {})",
        delegate_address,
        program_id
    );

    let client = reqwest::Client::new();
    let request_body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTokenAccountsByDelegate",
        "params": [
            delegate_address,
            {
                "programId": program_id
            },
            {
                "encoding": "jsonParsed"
            }
        ]
    });

    let response = client
        .post(&rpc_url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to query RPC: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("RPC error ({}): {}", status, error_text));
    }

    let response_data: Value = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse RPC response: {}", e))?;

    // Check for RPC error
    if let Some(error) = response_data.get("error") {
        return Err(anyhow::anyhow!(
            "RPC returned error: {}",
            serde_json::to_string_pretty(error)?
        ));
    }

    // Parse token accounts
    let token_accounts: Vec<TokenAccount> = serde_json::from_value(
        response_data
            .get("result")
            .and_then(|r| r.get("value"))
            .cloned()
            .unwrap_or(json!([])),
    )
    .unwrap_or_default();

    // Format output
    let mut output = String::new();
    output.push_str(&format!(
        "# Token Accounts Delegated to: {}\n\n",
        delegate_address
    ));
    output.push_str(&format!(
        "**Total Delegated Accounts:** {}\n",
        token_accounts.len()
    ));
    output.push_str(&format!("**Program ID:** `{}`\n\n", program_id));

    if token_accounts.is_empty() {
        output.push_str("No delegated token accounts found.\n");
        output.push_str("\nThis means no wallet has approved this address as a delegate.\n");
    } else {
        output.push_str("## Delegated Token Accounts\n\n");
        output.push_str(
            "⚠️ **WARNING:** These accounts have granted spending authority to the delegate.\n\n",
        );

        for (i, account) in token_accounts.iter().enumerate() {
            let info = &account.account.data.parsed.info;
            output.push_str(&format!(
                "### {}. Token Account: `{}`\n",
                i + 1,
                account.pubkey
            ));
            output.push_str(&format!("- **Owner:** `{}`\n", info.owner));
            output.push_str(&format!("- **Mint:** `{}`\n", info.mint));
            output.push_str(&format!("- **State:** {}\n", info.state));
            output.push_str(&format!(
                "- **Balance:** {} ({})\n",
                info.token_amount.ui_amount_string, info.token_amount.amount
            ));
            output.push_str(&format!("- **Decimals:** {}\n", info.token_amount.decimals));

            if info.is_native {
                output.push_str("- **Type:** Native SOL\n");
            }

            output.push_str("\n");
        }

        output.push_str("## Security Recommendation\n");
        output.push_str("If you don't recognize this delegate or no longer need the delegation:\n");
        output.push_str("1. Revoke the delegation immediately using your wallet\n");
        output.push_str("2. Consider transferring tokens to a new account\n");
        output.push_str("3. Investigate how this delegation was created\n");
    }

    Ok(json!({
        "content": [{
            "type": "text",
            "text": output
        }]
    }))
}
