use anyhow::{anyhow, Result};
use log::{debug, info, warn};
use solana_account_decoder::UiAccountEncoding;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use spl_token::solana_program::program_option::COption;
use spl_token::state::Account as TokenAccount;
use std::io::Write;
use std::str::FromStr;

use crate::detector::{Severity, ThreatAssessment, ThreatType};

/// Scans current on-chain state for active vulnerabilities
pub struct StateScanner;

impl StateScanner {
    /// Scan wallet's current state for active threats
    pub async fn scan_current_state(
        rpc: &RpcClient,
        wallet: &str,
        commitment: CommitmentConfig,
    ) -> Result<Vec<ThreatAssessment>> {
        let wallet_pubkey =
            Pubkey::from_str(wallet).map_err(|e| anyhow!("Invalid wallet address: {}", e))?;

        eprint!("🔍 Checking active delegations... ");
        std::io::stderr().flush().ok();

        // Fetch all token accounts owned by this wallet
        let token_accounts = Self::fetch_token_accounts(rpc, &wallet_pubkey, commitment)?;

        eprintln!("found {} token accounts", token_accounts.len());
        info!("Found {} token accounts", token_accounts.len());

        // Check each token account for active delegations
        let mut threats = Vec::new();
        for (pubkey, account_data) in token_accounts {
            if let Some(threat) = Self::check_delegation(&pubkey, &account_data) {
                threats.push(threat);
            }
        }

        info!("Found {} active threats in current state", threats.len());
        Ok(threats)
    }

    /// Fetch all token accounts owned by a wallet
    fn fetch_token_accounts(
        rpc: &RpcClient,
        owner: &Pubkey,
        commitment: CommitmentConfig,
    ) -> Result<Vec<(Pubkey, TokenAccount)>> {
        // SPL Token program ID
        let token_program = spl_token::id();

        // Filter for accounts owned by this wallet
        let config = RpcProgramAccountsConfig {
            filters: Some(vec![
                RpcFilterType::Memcmp(Memcmp::new(
                    32, // owner field offset in token account
                    MemcmpEncodedBytes::Bytes(owner.to_bytes().to_vec()),
                )),
                RpcFilterType::DataSize(165), // TokenAccount::LEN
            ]),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                commitment: Some(commitment),
                ..Default::default()
            },
            ..Default::default()
        };

        let accounts = rpc.get_program_accounts_with_config(&token_program, config)?;

        let mut token_accounts = Vec::new();
        for (pubkey, account) in accounts {
            // Unpack token account data using SPL Token's Pack trait
            match TokenAccount::unpack(&account.data) {
                Ok(token_account) => {
                    debug!(
                        "Token account {}: mint={}, amount={}, delegate={:?}",
                        pubkey, token_account.mint, token_account.amount, token_account.delegate
                    );
                    token_accounts.push((pubkey, token_account));
                }
                Err(e) => {
                    warn!("Failed to unpack token account {}: {}", pubkey, e);
                }
            }
        }

        Ok(token_accounts)
    }

    /// Check if a token account has an active delegation
    fn check_delegation(
        account_pubkey: &Pubkey,
        token_account: &TokenAccount,
    ) -> Option<ThreatAssessment> {
        // Check if there's an active delegate (COption is SPL token's Option type)
        if let COption::Some(delegate) = token_account.delegate {
            let delegated_amount = token_account.delegated_amount;

            if delegated_amount > 0 {
                // Check if it's an unlimited delegation (max u64)
                let is_unlimited = delegated_amount == u64::MAX;

                let severity = if is_unlimited {
                    Severity::Critical
                } else if delegated_amount > token_account.amount / 2 {
                    Severity::High
                } else {
                    Severity::Medium
                };

                let recommendation = if is_unlimited {
                    format!(
                        "URGENT: Revoke unlimited delegation to {} on token account {}. \
                        This allows the delegate to drain all tokens at any time.",
                        delegate, account_pubkey
                    )
                } else {
                    format!(
                        "Consider revoking delegation to {} on token account {}. \
                        Delegated amount: {}",
                        delegate, account_pubkey, delegated_amount
                    )
                };

                return Some(ThreatAssessment {
                    threat_type: ThreatType::ActiveUnlimitedDelegation {
                        token_account: account_pubkey.to_string(),
                        delegate: delegate.to_string(),
                        amount: delegated_amount,
                        granted_at: None, // We don't know when it was granted from state alone
                    },
                    severity,
                    recommendation,
                });
            }
        }

        None
    }
}
