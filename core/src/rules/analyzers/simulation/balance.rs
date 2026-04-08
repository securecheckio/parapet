use super::SimulationAnalyzer;
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Analyzes SOL balance changes from simulation results
pub struct SimulationBalanceAnalyzer;

impl SimulationBalanceAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Convert lamports to SOL
    fn lamports_to_sol(lamports: u64) -> f64 {
        lamports as f64 / 1_000_000_000.0
    }
}

#[async_trait::async_trait]
impl SimulationAnalyzer for SimulationBalanceAnalyzer {
    fn name(&self) -> &str {
        "simulation_balance"
    }

    fn fields(&self) -> Vec<String> {
        vec![
            "sol_balance_change".to_string(),
            "sol_balance_change_percent".to_string(),
            "total_sol_outflow".to_string(),
            "accounts_losing_balance".to_string(),
            "largest_balance_decrease".to_string(),
            "fee_payer_balance_change".to_string(),
        ]
    }

    async fn analyze(&self, simulation_result: &Value) -> Result<HashMap<String, Value>> {
        let mut fields = HashMap::new();

        // Extract balance arrays
        let pre_balances = simulation_result
            .get("preBalances")
            .and_then(|v| v.as_array());
        let post_balances = simulation_result
            .get("postBalances")
            .and_then(|v| v.as_array());

        if let (Some(pre), Some(post)) = (pre_balances, post_balances) {
            if pre.len() != post.len() {
                log::warn!(
                    "Balance array length mismatch: pre={}, post={}",
                    pre.len(),
                    post.len()
                );
                return Ok(fields);
            }

            // Calculate changes for each account
            let mut total_outflow = 0.0;
            let mut accounts_losing = 0u32;
            let mut largest_decrease = 0.0;
            let mut fee_payer_change = 0.0;

            for (idx, (pre_val, post_val)) in pre.iter().zip(post.iter()).enumerate() {
                let pre_lamports = pre_val.as_u64().unwrap_or(0);
                let post_lamports = post_val.as_u64().unwrap_or(0);

                let change_lamports = post_lamports as i64 - pre_lamports as i64;
                let change_sol = Self::lamports_to_sol(change_lamports.unsigned_abs());

                // Fee payer is always account index 0
                if idx == 0 {
                    fee_payer_change = if change_lamports < 0 {
                        -change_sol
                    } else {
                        change_sol
                    };
                }

                // Track losses
                if change_lamports < 0 {
                    total_outflow += change_sol;
                    accounts_losing += 1;
                    if change_sol > largest_decrease {
                        largest_decrease = change_sol;
                    }
                }
            }

            // Calculate percentage change for fee payer
            let fee_payer_percent = if let Some(pre_val) = pre.first() {
                let pre_lamports = pre_val.as_u64().unwrap_or(0);
                if pre_lamports > 0 {
                    (fee_payer_change / Self::lamports_to_sol(pre_lamports)) * 100.0
                } else {
                    0.0
                }
            } else {
                0.0
            };

            fields.insert("sol_balance_change".to_string(), json!(fee_payer_change));
            fields.insert(
                "sol_balance_change_percent".to_string(),
                json!(fee_payer_percent),
            );
            fields.insert("total_sol_outflow".to_string(), json!(total_outflow));
            fields.insert(
                "accounts_losing_balance".to_string(),
                json!(accounts_losing),
            );
            fields.insert(
                "largest_balance_decrease".to_string(),
                json!(largest_decrease),
            );
            fields.insert(
                "fee_payer_balance_change".to_string(),
                json!(fee_payer_change),
            );
        } else {
            // No balance data available - set defaults
            fields.insert("sol_balance_change".to_string(), json!(0.0));
            fields.insert("sol_balance_change_percent".to_string(), json!(0.0));
            fields.insert("total_sol_outflow".to_string(), json!(0.0));
            fields.insert("accounts_losing_balance".to_string(), json!(0));
            fields.insert("largest_balance_decrease".to_string(), json!(0.0));
            fields.insert("fee_payer_balance_change".to_string(), json!(0.0));
        }

        Ok(fields)
    }

    fn estimated_latency_ms(&self) -> u64 {
        1
    }
}

impl Default for SimulationBalanceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
