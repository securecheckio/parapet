/// Address Lookup Table resolution for v0 transactions
use anyhow::{anyhow, Result};
use solana_address_lookup_table_interface::state::AddressLookupTable;
use solana_sdk::{
    message::{compiled_instruction::CompiledInstruction, v0, VersionedMessage},
    pubkey::Pubkey,
    transaction::{Transaction, VersionedTransaction},
};
use std::sync::Arc;

/// Callback type for fetching a single account
pub type AccountFetcher = Box<
    dyn Fn(
            &str,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<Vec<u8>>>> + Send>>
        + Send
        + Sync,
>;

/// Callback type for fetching multiple accounts at once (more efficient)
pub type BatchAccountFetcher = Box<
    dyn Fn(
            &[String],
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<Vec<Option<Vec<u8>>>>> + Send>,
        > + Send
        + Sync,
>;

/// ALT resolution context with caching and batch fetching
pub struct AltResolver {
    cache: Arc<super::alt_cache::AltCache>,
    batch_fetcher: Arc<BatchAccountFetcher>,
}

impl AltResolver {
    pub fn new(
        cache: Arc<super::alt_cache::AltCache>,
        batch_fetcher: Arc<BatchAccountFetcher>,
    ) -> Self {
        Self {
            cache,
            batch_fetcher,
        }
    }

    /// Resolve Address Lookup Tables in a v0 transaction and convert to a legacy transaction
    pub async fn resolve_v0_transaction(&self, tx: &VersionedTransaction) -> Result<Transaction> {
        // Extract the v0 message
        let v0_message = match &tx.message {
            VersionedMessage::V0(msg) => msg,
            VersionedMessage::Legacy(_) => {
                return Err(anyhow!("Transaction is already legacy format"));
            }
            VersionedMessage::V1(_) => {
                return Err(anyhow!(
                    "v1 messages do not use address lookup tables; nothing to resolve"
                ));
            }
        };

        // Resolve ALT addresses with caching
        let resolved_addresses = self.resolve_alt_addresses(v0_message).await?;

        // Build the full account keys list
        let mut account_keys = v0_message.account_keys.clone();

        // Add resolved addresses in order: writable first, then readonly
        account_keys.extend(resolved_addresses.writable);
        account_keys.extend(resolved_addresses.readonly);

        // Create a legacy message
        let legacy_message = solana_sdk::message::Message {
            header: solana_sdk::message::MessageHeader {
                num_required_signatures: v0_message.header.num_required_signatures,
                num_readonly_signed_accounts: v0_message.header.num_readonly_signed_accounts,
                num_readonly_unsigned_accounts: v0_message.header.num_readonly_unsigned_accounts,
            },
            account_keys,
            recent_blockhash: v0_message.recent_blockhash,
            instructions: v0_message
                .instructions
                .iter()
                .map(|ix| CompiledInstruction {
                    program_id_index: ix.program_id_index,
                    accounts: ix.accounts.clone(),
                    data: ix.data.clone(),
                })
                .collect(),
        };

        // Create legacy transaction with the same signatures
        Ok(Transaction {
            signatures: tx.signatures.clone(),
            message: legacy_message,
        })
    }

    /// Fetch and resolve all ALT addresses referenced in the message (with caching and batching)
    async fn resolve_alt_addresses(&self, message: &v0::Message) -> Result<ResolvedAddresses> {
        if message.address_table_lookups.is_empty() {
            return Ok(ResolvedAddresses {
                writable: Vec::new(),
                readonly: Vec::new(),
            });
        }

        // Collect all ALT pubkeys
        let alt_pubkeys: Vec<String> = message
            .address_table_lookups
            .iter()
            .map(|lookup| lookup.account_key.to_string())
            .collect();

        log::debug!("🔍 Resolving {} ALTs", alt_pubkeys.len());

        // Check cache first
        let cached_data = self.cache.get_multiple(&alt_pubkeys).await;

        // Identify which ALTs need fetching
        let mut to_fetch = Vec::new();
        let mut fetch_indices = Vec::new();

        for (i, (pubkey, cached)) in alt_pubkeys.iter().zip(cached_data.iter()).enumerate() {
            if cached.is_none() {
                to_fetch.push(pubkey.clone());
                fetch_indices.push(i);
            }
        }

        // Fetch missing ALTs in one batch call
        let mut all_alt_data = cached_data;

        if !to_fetch.is_empty() {
            log::debug!("📥 Fetching {} ALTs from RPC (batch)", to_fetch.len());
            let fetched = (self.batch_fetcher)(&to_fetch).await?;

            // Update cache with newly fetched data
            let mut cache_updates = Vec::new();
            for (pubkey, data) in to_fetch.iter().zip(fetched.iter()) {
                if let Some(data) = data {
                    cache_updates.push((pubkey.clone(), data.clone()));
                }
            }
            if !cache_updates.is_empty() {
                self.cache.set_multiple(cache_updates).await;
            }

            // Merge fetched data back into the results
            for (fetch_idx, alt_idx) in fetch_indices.iter().enumerate() {
                all_alt_data[*alt_idx] = fetched[fetch_idx].clone();
            }
        } else {
            log::debug!("✅ All {} ALTs found in cache", alt_pubkeys.len());
        }

        // Now resolve addresses from the ALT data
        let mut writable = Vec::new();
        let mut readonly = Vec::new();

        for (lookup, alt_data) in message
            .address_table_lookups
            .iter()
            .zip(all_alt_data.iter())
        {
            let alt_pubkey = lookup.account_key.to_string();

            let data = alt_data
                .as_ref()
                .ok_or_else(|| anyhow!("ALT account not found: {}", alt_pubkey))?;

            // Deserialize the ALT account
            let alt = AddressLookupTable::deserialize(data)
                .map_err(|e| anyhow!("Failed to deserialize ALT {}: {}", alt_pubkey, e))?;

            // Resolve writable indices
            for &index in &lookup.writable_indexes {
                let address = alt.addresses.get(index as usize).ok_or_else(|| {
                    anyhow!("Invalid writable index {} in ALT {}", index, alt_pubkey)
                })?;
                writable.push(*address);
            }

            // Resolve readonly indices
            for &index in &lookup.readonly_indexes {
                let address = alt.addresses.get(index as usize).ok_or_else(|| {
                    anyhow!("Invalid readonly index {} in ALT {}", index, alt_pubkey)
                })?;
                readonly.push(*address);
            }
        }

        log::debug!(
            "✅ Resolved {} writable, {} readonly addresses from {} ALTs",
            writable.len(),
            readonly.len(),
            alt_pubkeys.len()
        );

        Ok(ResolvedAddresses { writable, readonly })
    }
}

/// Legacy function for backwards compatibility (uses single-account fetching, no cache)
pub async fn resolve_v0_transaction(
    tx: &VersionedTransaction,
    account_fetcher: &AccountFetcher,
) -> Result<Transaction> {
    // Extract the v0 message
    let v0_message = match &tx.message {
        VersionedMessage::V0(msg) => msg,
        VersionedMessage::Legacy(_) => {
            return Err(anyhow!("Transaction is already legacy format"));
        }
        VersionedMessage::V1(_) => {
            return Err(anyhow!(
                "v1 messages do not use address lookup tables; nothing to resolve"
            ));
        }
    };

    // Resolve ALT addresses
    let resolved_addresses = resolve_alt_addresses(v0_message, account_fetcher).await?;

    // Build the full account keys list
    let mut account_keys = v0_message.account_keys.clone();

    // Add resolved addresses in order: writable first, then readonly
    account_keys.extend(resolved_addresses.writable);
    account_keys.extend(resolved_addresses.readonly);

    // Create a legacy message
    let legacy_message = solana_sdk::message::Message {
        header: solana_sdk::message::MessageHeader {
            num_required_signatures: v0_message.header.num_required_signatures,
            num_readonly_signed_accounts: v0_message.header.num_readonly_signed_accounts,
            num_readonly_unsigned_accounts: v0_message.header.num_readonly_unsigned_accounts,
        },
        account_keys,
        recent_blockhash: v0_message.recent_blockhash,
        instructions: v0_message
            .instructions
            .iter()
            .map(|ix| CompiledInstruction {
                program_id_index: ix.program_id_index,
                accounts: ix.accounts.clone(),
                data: ix.data.clone(),
            })
            .collect(),
    };

    // Create legacy transaction with the same signatures
    Ok(Transaction {
        signatures: tx.signatures.clone(),
        message: legacy_message,
    })
}

struct ResolvedAddresses {
    writable: Vec<Pubkey>,
    readonly: Vec<Pubkey>,
}

/// Fetch and resolve all ALT addresses referenced in the message (legacy, sequential)
async fn resolve_alt_addresses(
    message: &v0::Message,
    account_fetcher: &AccountFetcher,
) -> Result<ResolvedAddresses> {
    let mut writable = Vec::new();
    let mut readonly = Vec::new();

    for lookup in &message.address_table_lookups {
        let alt_pubkey = lookup.account_key.to_string();

        log::debug!("📋 Fetching ALT account: {}", alt_pubkey);

        let account_data = account_fetcher(&alt_pubkey)
            .await?
            .ok_or_else(|| anyhow!("ALT account not found: {}", alt_pubkey))?;

        // Deserialize the ALT account
        let alt = AddressLookupTable::deserialize(&account_data)
            .map_err(|e| anyhow!("Failed to deserialize ALT: {}", e))?;

        // Resolve writable indices
        for &index in &lookup.writable_indexes {
            let address = alt
                .addresses
                .get(index as usize)
                .ok_or_else(|| anyhow!("Invalid writable index {} in ALT {}", index, alt_pubkey))?;
            writable.push(*address);
        }

        // Resolve readonly indices
        for &index in &lookup.readonly_indexes {
            let address = alt
                .addresses
                .get(index as usize)
                .ok_or_else(|| anyhow!("Invalid readonly index {} in ALT {}", index, alt_pubkey))?;
            readonly.push(*address);
        }

        log::debug!(
            "✅ Resolved ALT {}: {} writable, {} readonly addresses",
            alt_pubkey,
            lookup.writable_indexes.len(),
            lookup.readonly_indexes.len()
        );
    }

    Ok(ResolvedAddresses { writable, readonly })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resolve_legacy_transaction_returns_error() {
        let fetcher: AccountFetcher = Box::new(|_| Box::pin(async { Ok(None) }));

        // Create a simple legacy transaction
        let tx = VersionedTransaction {
            signatures: vec![],
            message: VersionedMessage::Legacy(solana_sdk::message::Message {
                header: solana_sdk::message::MessageHeader {
                    num_required_signatures: 1,
                    num_readonly_signed_accounts: 0,
                    num_readonly_unsigned_accounts: 1,
                },
                account_keys: vec![],
                recent_blockhash: solana_sdk::hash::Hash::default(),
                instructions: vec![],
            }),
        };

        let result = resolve_v0_transaction(&tx, &fetcher).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already legacy"));
    }
}
