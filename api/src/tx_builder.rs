// Transaction builder utilities for defense automation
// Generates unsigned transactions that users can sign to remediate threats

use anyhow::{anyhow, Result};
use base64::Engine;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{message::Message, pubkey::Pubkey, transaction::Transaction};
use spl_token_interface::instruction as token_instruction;
use std::str::FromStr;

/// Generate an unsigned transaction to revoke a token approval
///
/// # Arguments
/// * `wallet` - The wallet address (owner of the token account)
/// * `token_account` - The token account with the delegation
/// * `rpc_client` - RPC client to get recent blockhash
///
/// # Returns
/// Base64-encoded unsigned transaction that user can sign
pub async fn build_revoke_approval_tx(
    wallet: &str,
    token_account: &str,
    rpc_client: &RpcClient,
) -> Result<String> {
    let owner = Pubkey::from_str(wallet).map_err(|e| anyhow!("Invalid wallet address: {}", e))?;

    let token_account_pubkey = Pubkey::from_str(token_account)
        .map_err(|e| anyhow!("Invalid token account address: {}", e))?;

    // Create revoke instruction
    let revoke_ix = token_instruction::revoke(
        &spl_token_interface::id(),
        &token_account_pubkey,
        &owner,
        &[],
    )?;

    // Get recent blockhash
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .map_err(|e| anyhow!("Failed to get recent blockhash: {}", e))?;

    // Create transaction
    let message = Message::new(&[revoke_ix], Some(&owner));
    let mut tx = Transaction::new_unsigned(message);
    tx.message.recent_blockhash = recent_blockhash;

    // Serialize to base64
    let serialized = bincode::serialize(&tx)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&serialized))
}

/// Generate unsigned transactions to revoke multiple approvals
///
/// SPL Token revoke instructions are small, so we can batch multiple in one transaction.
/// If there are too many to fit in one transaction, we return multiple transactions.
///
/// # Arguments
/// * `wallet` - The wallet address (owner of the token accounts)
/// * `token_accounts` - List of token accounts with delegations
/// * `rpc_client` - RPC client to get recent blockhash
///
/// # Returns
/// Vector of base64-encoded unsigned transactions
pub async fn build_batch_revoke_tx(
    wallet: &str,
    token_accounts: &[String],
    rpc_client: &RpcClient,
) -> Result<Vec<String>> {
    if token_accounts.is_empty() {
        return Err(anyhow!("No token accounts provided"));
    }

    let owner = Pubkey::from_str(wallet).map_err(|e| anyhow!("Invalid wallet address: {}", e))?;

    let mut transactions = Vec::new();

    // Batch up to 10 revoke instructions per transaction (conservative limit)
    // Solana transactions have a size limit, and we want to leave room for signatures
    const MAX_REVOKES_PER_TX: usize = 10;

    for chunk in token_accounts.chunks(MAX_REVOKES_PER_TX) {
        let mut instructions = Vec::new();

        for token_account_str in chunk {
            let token_account_pubkey = Pubkey::from_str(token_account_str).map_err(|e| {
                anyhow!("Invalid token account address {}: {}", token_account_str, e)
            })?;

            let revoke_ix = token_instruction::revoke(
                &spl_token_interface::id(),
                &token_account_pubkey,
                &owner,
                &[],
            )?;

            instructions.push(revoke_ix);
        }

        // Get recent blockhash
        let recent_blockhash = rpc_client
            .get_latest_blockhash()
            .map_err(|e| anyhow!("Failed to get recent blockhash: {}", e))?;

        // Create transaction
        let message = Message::new(&instructions, Some(&owner));
        let mut tx = Transaction::new_unsigned(message);
        tx.message.recent_blockhash = recent_blockhash;

        // Serialize to base64
        let serialized = bincode::serialize(&tx)?;
        transactions.push(base64::engine::general_purpose::STANDARD.encode(&serialized));
    }

    Ok(transactions)
}

/// Build a summary of what a revoke transaction will do
///
/// This is useful for displaying to users before they sign
pub fn describe_revoke_transaction(
    wallet: &str,
    token_account: &str,
    token_symbol: Option<&str>,
) -> String {
    let symbol = token_symbol.unwrap_or("tokens");
    format!(
        "Transaction will REVOKE approval on token account:\n\
         - Token Account: {}\n\
         - Token: {}\n\
         - Owner: {}\n\
         \n\
         After signing, the delegate will NO LONGER be able to transfer {} from this account.\n\
         This is a security action to protect your wallet.",
        token_account, symbol, wallet, symbol
    )
}

/// Build a summary for batch revoke transaction
pub fn describe_batch_revoke(
    wallet: &str,
    token_accounts: &[String],
    num_transactions: usize,
) -> String {
    if num_transactions == 1 {
        format!(
            "Transaction will REVOKE approvals on {} token accounts:\n\
             - Owner: {}\n\
             \n\
             After signing, delegates will NO LONGER be able to transfer from these accounts.\n\
             This is a security action to protect your wallet.",
            token_accounts.len(),
            wallet
        )
    } else {
        format!(
            "Will create {} transactions to REVOKE approvals on {} token accounts:\n\
             - Owner: {}\n\
             \n\
             After signing all transactions, delegates will NO LONGER be able to transfer from these accounts.\n\
             This is a security action to protect your wallet.",
            num_transactions,
            token_accounts.len(),
            wallet
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_describe_revoke_transaction() {
        let description = describe_revoke_transaction(
            "So11111111111111111111111111111111111111112",
            "TokenAccount1111111111111111111111111111",
            Some("USDC"),
        );

        assert!(description.contains("REVOKE"));
        assert!(description.contains("USDC"));
        assert!(description.contains("TokenAccount1111111111111111111111111111"));
    }

    #[test]
    fn test_describe_batch_revoke() {
        let token_accounts = vec![
            "TokenAccount1".to_string(),
            "TokenAccount2".to_string(),
            "TokenAccount3".to_string(),
        ];

        let description = describe_batch_revoke(
            "So11111111111111111111111111111111111111112",
            &token_accounts,
            1,
        );

        assert!(description.contains("3 token accounts"));
        assert!(description.contains("REVOKE"));
    }
}
