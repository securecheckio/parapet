use super::SimulationAnalyzer;
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Analyzes token balance changes from simulation results
pub struct SimulationTokenAnalyzer;

impl SimulationTokenAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Check if a token is likely an NFT (amount = 1, decimals = 0)
    fn is_likely_nft(amount: &str, decimals: u8) -> bool {
        decimals == 0 && amount == "1"
    }
}

#[async_trait::async_trait]
impl SimulationAnalyzer for SimulationTokenAnalyzer {
    fn name(&self) -> &str {
        "simulation_token"
    }

    fn fields(&self) -> Vec<String> {
        vec![
            "token_transfers_out".to_string(),
            "token_transfers_in".to_string(),
            "net_token_changes".to_string(),
            "nft_transfers".to_string(),
            "nft_transfer_count".to_string(),
            "token_mints_involved".to_string(),
            "tokens_fully_drained".to_string(),
        ]
    }

    async fn analyze(&self, simulation_result: &Value) -> Result<HashMap<String, Value>> {
        let mut fields = HashMap::new();

        // Extract token balance arrays
        let pre_token_balances = simulation_result
            .get("preTokenBalances")
            .and_then(|v| v.as_array());
        let post_token_balances = simulation_result
            .get("postTokenBalances")
            .and_then(|v| v.as_array());

        let mut transfers_out = 0u32;
        let mut transfers_in = 0u32;
        let mut nft_transfers = Vec::new();
        let mut token_mints = std::collections::HashSet::new();
        let mut tokens_drained = 0u32;

        if let (Some(pre), Some(post)) = (pre_token_balances, post_token_balances) {
            // Build map of account_index -> pre balance
            let mut pre_map: HashMap<u64, &Value> = HashMap::new();
            for pre_balance in pre {
                if let Some(account_idx) = pre_balance.get("accountIndex").and_then(|v| v.as_u64()) {
                    pre_map.insert(account_idx, pre_balance);
                }
            }

            // Build map of account_index -> post balance
            let mut post_map: HashMap<u64, &Value> = HashMap::new();
            for post_balance in post {
                if let Some(account_idx) = post_balance.get("accountIndex").and_then(|v| v.as_u64())
                {
                    post_map.insert(account_idx, post_balance);
                }
            }

            // Compare pre and post for each account
            let mut all_accounts: std::collections::HashSet<u64> = pre_map.keys().copied().collect();
            all_accounts.extend(post_map.keys());

            for account_idx in all_accounts {
                let pre_balance = pre_map.get(&account_idx);
                let post_balance = post_map.get(&account_idx);

                let pre_amount = pre_balance
                    .and_then(|b| b.get("uiTokenAmount"))
                    .and_then(|amt| amt.get("amount"))
                    .and_then(|a| a.as_str())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);

                let post_amount = post_balance
                    .and_then(|b| b.get("uiTokenAmount"))
                    .and_then(|amt| amt.get("amount"))
                    .and_then(|a| a.as_str())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);

                // Track mint addresses
                if let Some(mint) = pre_balance.and_then(|b| b.get("mint")).and_then(|m| m.as_str())
                {
                    token_mints.insert(mint.to_string());
                }
                if let Some(mint) = post_balance
                    .and_then(|b| b.get("mint"))
                    .and_then(|m| m.as_str())
                {
                    token_mints.insert(mint.to_string());
                }

                // Determine transfer direction
                if post_amount < pre_amount {
                    transfers_out += 1;

                    // Check if fully drained
                    if post_amount == 0 && pre_amount > 0 {
                        tokens_drained += 1;
                    }

                    // Check if NFT
                    if let Some(pre_bal) = pre_balance {
                        let decimals = pre_bal
                            .get("uiTokenAmount")
                            .and_then(|amt| amt.get("decimals"))
                            .and_then(|d| d.as_u64())
                            .unwrap_or(0) as u8;

                        let amount_str = pre_amount.to_string();
                        if Self::is_likely_nft(&amount_str, decimals) {
                            if let Some(mint) = pre_bal.get("mint").and_then(|m| m.as_str()) {
                                nft_transfers.push(mint.to_string());
                            }
                        }
                    }
                } else if post_amount > pre_amount {
                    transfers_in += 1;
                }
            }
        }

        // Calculate net token changes (for rules)
        let net_changes = transfers_in as i32 - transfers_out as i32;

        fields.insert("token_transfers_out".to_string(), json!(transfers_out));
        fields.insert("token_transfers_in".to_string(), json!(transfers_in));
        fields.insert("net_token_changes".to_string(), json!(net_changes));
        fields.insert("nft_transfers".to_string(), json!(nft_transfers));
        fields.insert("nft_transfer_count".to_string(), json!(nft_transfers.len()));
        fields.insert(
            "token_mints_involved".to_string(),
            json!(token_mints.len()),
        );
        fields.insert("tokens_fully_drained".to_string(), json!(tokens_drained));

        Ok(fields)
    }

    fn estimated_latency_ms(&self) -> u64 {
        1
    }
}

impl Default for SimulationTokenAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
