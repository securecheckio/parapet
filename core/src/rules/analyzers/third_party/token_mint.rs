use crate::rules::analyzer::TransactionAnalyzer;
use crate::rules::analyzers::third_party::rate_limiter::ApiRateLimiter;
use anyhow::Result;
use serde_json::{json, Value};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Token-2022 Extension Data
#[derive(Clone, Debug, Default)]
struct ExtensionData {
    has_permanent_delegate: bool,
    permanent_delegate_address: Option<String>,
    has_transfer_hook: bool,
    transfer_hook_program: Option<String>,
    has_transfer_fee: bool,
    transfer_fee_basis_points: Option<u16>,
    has_metadata_pointer: bool,
    has_default_frozen_state: bool,
}

/// Cached mint information
#[derive(Clone, Debug)]
struct MintInfo {
    has_freeze_authority: bool,
    freeze_authority: Option<String>,
    mint_authority: Option<String>,
    supply: u64,
    decimals: u8,
    is_token_2022: bool,
    extensions: ExtensionData,
}

/// Cache entry with TTL
struct CacheEntry {
    info: MintInfo,
    expires_at: Instant,
}

/// Token mint analyzer - fetches mint account data via RPC
pub struct TokenMintAnalyzer {
    rpc_client: Arc<RpcClient>,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    cache_ttl: Duration,
    rate_limiter: ApiRateLimiter,
}

impl TokenMintAnalyzer {
    pub fn new(rpc_url: String) -> Self {
        // Configure rate limiter from env or use RPC defaults
        // Free RPC: ~40 req/10s, Paid: varies by provider
        let rate_limiter = ApiRateLimiter::from_env_or_default(
            "RPC_RATE_LIMIT",
            40, // Conservative free tier default
            10, // 10 second window
        );

        Self {
            rpc_client: Arc::new(RpcClient::new(rpc_url)),
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(3600), // 1 hour cache
            rate_limiter,
        }
    }

    /// Fetch mint info from RPC (with caching and rate limiting)
    async fn fetch_mint_info(&self, mint_address: &str) -> Result<MintInfo> {
        let now = Instant::now();

        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(mint_address) {
                if entry.expires_at > now {
                    return Ok(entry.info.clone());
                }
            }
        }

        // Cache miss or expired - fetch from RPC
        // Acquire rate limit permit
        let _permit = self.rate_limiter.acquire().await;

        let pubkey = Pubkey::from_str(mint_address)?;
        let account_data = self.rpc_client.get_account(&pubkey)?;

        // Parse SPL Token Mint account
        // Mint account layout:
        // - 0..4: mint_authority (Option<Pubkey>) - 4 bytes (0 = None, 1 = Some + 32 bytes pubkey)
        // - 36..44: supply (u64)
        // - 44: decimals (u8)
        // - 45: is_initialized (bool)
        // - 46..50: freeze_authority (Option<Pubkey>) - same as mint_authority

        if account_data.data.len() < 82 {
            anyhow::bail!("Invalid mint account data size");
        }

        let data = &account_data.data;

        // Parse mint_authority (offset 0)
        let mint_authority = if data[0] == 1 {
            let authority_bytes = &data[4..36];
            Some(bs58::encode(authority_bytes).into_string())
        } else {
            None
        };

        // Parse supply (offset 36)
        let supply = u64::from_le_bytes(data[36..44].try_into()?);

        // Parse decimals (offset 44)
        let decimals = data[44];

        // Parse freeze_authority (offset 46)
        let freeze_authority = if data[46] == 1 {
            let authority_bytes = &data[50..82];
            Some(bs58::encode(authority_bytes).into_string())
        } else {
            None
        };

        // Check if this is Token-2022 and parse extensions
        let is_token_2022 =
            account_data.owner.to_string() == "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
        let extensions = if is_token_2022 {
            self.parse_token_2022_extensions(&account_data.data)
        } else {
            ExtensionData::default()
        };

        let info = MintInfo {
            has_freeze_authority: freeze_authority.is_some(),
            freeze_authority: freeze_authority.clone(),
            mint_authority: mint_authority.clone(),
            supply,
            decimals,
            is_token_2022,
            extensions,
        };

        // Store in cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(
                mint_address.to_string(),
                CacheEntry {
                    info: info.clone(),
                    expires_at: now + self.cache_ttl,
                },
            );
        }

        Ok(info)
    }

    /// Parse Token-2022 extensions from mint account data using manual TLV parsing
    /// This avoids dependency conflicts and provides lightweight extension detection
    fn parse_token_2022_extensions(&self, data: &[u8]) -> ExtensionData {
        let mut ext_data = ExtensionData::default();

        // Token-2022 Mint base state is 82 bytes
        // Extensions start at byte 165 (82 base + 83 padding to account discriminator)
        // Actually, extensions start right after the base mint at byte 82
        const BASE_MINT_SIZE: usize = 82;

        if data.len() <= BASE_MINT_SIZE {
            return ext_data; // No extensions
        }

        // Parse TLV entries starting at offset 82
        let mut offset = BASE_MINT_SIZE;

        while offset + 4 <= data.len() {
            // Read extension type (2 bytes, little-endian)
            let ext_type = u16::from_le_bytes([data[offset], data[offset + 1]]);

            // Read extension length (2 bytes, little-endian)
            let ext_length = u16::from_le_bytes([data[offset + 2], data[offset + 3]]) as usize;

            offset += 4; // Move past type and length

            // Ensure we have enough data for the value
            if offset + ext_length > data.len() {
                log::warn!(
                    "Token-2022: Invalid extension length at offset {}",
                    offset - 4
                );
                break;
            }

            // Parse extension based on type
            // Extension type IDs from: https://github.com/solana-labs/solana-program-library
            match ext_type {
                1 => {
                    // TransferFeeConfig (type 1)
                    // Layout: config_authority (32 bytes), withdraw_withheld_authority (32 bytes),
                    //         withheld_amount (8 bytes), older_epoch (8 bytes),
                    //         older_max_fee (8 bytes), older_fee_bps (2 bytes), newer_epoch (8 bytes),
                    //         newer_max_fee (8 bytes), newer_fee_bps (2 bytes)
                    if ext_length >= 96 {
                        // older_fee_bps at offset 94
                        let fee_bps = u16::from_le_bytes([data[offset + 94], data[offset + 95]]);
                        ext_data.has_transfer_fee = true;
                        ext_data.transfer_fee_basis_points = Some(fee_bps);

                        let fee_pct = fee_bps as f64 / 100.0;
                        if fee_pct > 10.0 {
                            log::warn!(
                                "⚠️ HIGH: Transfer fee detected: {}% ({}bps)",
                                fee_pct,
                                fee_bps
                            );
                        }
                    }
                }
                2 => {
                    // TransferFeeAmount (type 2) - not a security risk
                }
                3 => {
                    // MintCloseAuthority (type 3) - not a direct threat
                }
                4 => {
                    // ConfidentialTransferMint (type 4) - privacy feature
                }
                5 => {
                    // ConfidentialTransferFeeConfig (type 5)
                }
                6 => {
                    // DefaultAccountState (type 6)
                    // Layout: state (1 byte) - 0=Uninitialized, 1=Initialized, 2=Frozen
                    if ext_length >= 1 {
                        let state = data[offset];
                        if state == 2 {
                            ext_data.has_default_frozen_state = true;
                            log::warn!("⚠️ MEDIUM: Default account state is FROZEN");
                        }
                    }
                }
                7 => {
                    // ImmutableOwner (type 7) - safety feature
                }
                8 => {
                    // MemoTransfer (type 8) - safety feature
                }
                9 => {
                    // NonTransferable (type 9) - soulbound tokens
                }
                10 => {
                    // InterestBearingConfig (type 10)
                }
                11 => {
                    // PermanentDelegate (type 11) - CRITICAL THREAT
                    // Layout: delegate (32 bytes pubkey)
                    if ext_length >= 32 {
                        let delegate_bytes = &data[offset..offset + 32];
                        let delegate_address = bs58::encode(delegate_bytes).into_string();
                        ext_data.has_permanent_delegate = true;
                        ext_data.permanent_delegate_address = Some(delegate_address.clone());
                        log::warn!(
                            "🚨 CRITICAL: Permanent Delegate detected at {}",
                            delegate_address
                        );
                    }
                }
                12 => {
                    // ConfidentialTransferFeeAmount (type 12)
                }
                13 => {
                    // TransferHook (type 13) - CRITICAL THREAT
                    // Layout: authority (32 bytes), program_id (32 bytes)
                    if ext_length >= 64 {
                        let program_id_bytes = &data[offset + 32..offset + 64];
                        let program_id = bs58::encode(program_id_bytes).into_string();
                        ext_data.has_transfer_hook = true;
                        ext_data.transfer_hook_program = Some(program_id.clone());
                        log::warn!("🚨 CRITICAL: Transfer Hook detected: {}", program_id);
                    }
                }
                14 => {
                    // MetadataPointer (type 14)
                    ext_data.has_metadata_pointer = true;
                }
                15 => {
                    // GroupPointer (type 15)
                }
                16 => {
                    // GroupMemberPointer (type 16)
                }
                _ => {
                    log::debug!(
                        "Token-2022: Unknown extension type {} at offset {}",
                        ext_type,
                        offset - 4
                    );
                }
            }

            // Move to next extension
            offset += ext_length;

            // Extensions are padded to 8-byte alignment
            let padding = (8 - (ext_length % 8)) % 8;
            offset += padding;
        }

        ext_data
    }

    /// Extract mint addresses from transaction
    fn extract_mint_addresses(&self, tx: &Transaction) -> Vec<String> {
        let mut mints = Vec::new();

        const SPL_TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
        const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
        const TRANSFER_CHECKED: u8 = 12;
        const APPROVE_CHECKED: u8 = 13;
        const MINT_TO_CHECKED: u8 = 14;
        const BURN_CHECKED: u8 = 15;

        for instruction in &tx.message.instructions {
            if let Some(program_id) = tx
                .message
                .account_keys
                .get(instruction.program_id_index as usize)
            {
                let prog_str = program_id.to_string();

                if prog_str != SPL_TOKEN_PROGRAM && prog_str != TOKEN_2022_PROGRAM {
                    continue;
                }

                if let Some(&discriminator) = instruction.data.first() {
                    // These instructions have mint at accounts[2]
                    if discriminator == TRANSFER_CHECKED
                        || discriminator == APPROVE_CHECKED
                        || discriminator == MINT_TO_CHECKED
                        || discriminator == BURN_CHECKED
                    {
                        if let Some(&mint_idx) = instruction.accounts.get(2) {
                            if let Some(mint) = tx.message.account_keys.get(mint_idx as usize) {
                                mints.push(mint.to_string());
                            }
                        }
                    }
                }
            }
        }

        // Deduplicate
        mints.sort();
        mints.dedup();
        mints
    }
}

#[async_trait::async_trait]
impl TransactionAnalyzer for TokenMintAnalyzer {
    fn name(&self) -> &str {
        "token_mint"
    }

    fn fields(&self) -> Vec<String> {
        vec![
            // Freeze authority detection
            "has_freeze_authority".to_string(),
            "freeze_authority_addresses".to_string(),
            "risky_mint_count".to_string(),
            // Mint authorities
            "mint_authority_addresses".to_string(),
            // Mint details
            "mints_analyzed".to_string(),
            "mint_details".to_string(),
            // Summary flags
            "all_mints_freezable".to_string(),
            "any_mint_freezable".to_string(),
            // Token-2022 detection
            "is_token_2022".to_string(),
            "any_token_2022".to_string(),
            // Token-2022 Extension Detection (Critical Threats)
            "has_permanent_delegate".to_string(),
            "permanent_delegate_addresses".to_string(),
            "has_transfer_hook".to_string(),
            "transfer_hook_programs".to_string(),
            // Token-2022 Extension Detection (High/Medium Threats)
            "has_transfer_fee".to_string(),
            "max_transfer_fee_bps".to_string(),
            "has_metadata_pointer".to_string(),
            "has_default_frozen_state".to_string(),
        ]
    }

    async fn analyze(&self, tx: &Transaction) -> Result<HashMap<String, Value>> {
        let mut fields = HashMap::new();

        // Extract mint addresses from transaction
        let mint_addresses = self.extract_mint_addresses(tx);

        if mint_addresses.is_empty() {
            // No mints to analyze - insert default values
            fields.insert("mints_analyzed".to_string(), json!(0));
            fields.insert("has_freeze_authority".to_string(), json!(false));
            fields.insert("freeze_authority_addresses".to_string(), json!([]));
            fields.insert("mint_authority_addresses".to_string(), json!([]));
            fields.insert("risky_mint_count".to_string(), json!(0));
            fields.insert("mint_details".to_string(), json!({}));
            fields.insert("all_mints_freezable".to_string(), json!(false));
            fields.insert("any_mint_freezable".to_string(), json!(false));
            fields.insert("is_token_2022".to_string(), json!(false));
            fields.insert("any_token_2022".to_string(), json!(false));
            fields.insert("has_permanent_delegate".to_string(), json!(false));
            fields.insert("permanent_delegate_addresses".to_string(), json!([]));
            fields.insert("has_transfer_hook".to_string(), json!(false));
            fields.insert("transfer_hook_programs".to_string(), json!([]));
            fields.insert("has_transfer_fee".to_string(), json!(false));
            fields.insert("max_transfer_fee_bps".to_string(), json!(null));
            fields.insert("has_metadata_pointer".to_string(), json!(false));
            fields.insert("has_default_frozen_state".to_string(), json!(false));
            return Ok(fields);
        }

        // Fetch mint info for each mint
        let mut mint_infos = Vec::new();
        let mut freeze_authorities = Vec::new();
        let mut mint_authorities = Vec::new();
        let mut mint_details_map = serde_json::Map::new();

        // Extension aggregation
        let mut permanent_delegate_addresses = Vec::new();
        let mut transfer_hook_programs = Vec::new();
        let mut max_fee_bps: Option<u16> = None;
        let mut any_token_2022 = false;

        for mint in &mint_addresses {
            match self.fetch_mint_info(mint).await {
                Ok(info) => {
                    if let Some(ref freeze_auth) = info.freeze_authority {
                        freeze_authorities.push(freeze_auth.clone());
                    }
                    if let Some(ref mint_auth) = info.mint_authority {
                        mint_authorities.push(mint_auth.clone());
                    }

                    // Aggregate Token-2022 extension data
                    if info.is_token_2022 {
                        any_token_2022 = true;
                    }

                    let ext = &info.extensions;
                    if let Some(ref addr) = ext.permanent_delegate_address {
                        permanent_delegate_addresses.push(addr.clone());
                    }
                    if let Some(ref prog) = ext.transfer_hook_program {
                        transfer_hook_programs.push(prog.clone());
                    }
                    if let Some(fee_bps) = ext.transfer_fee_basis_points {
                        max_fee_bps =
                            Some(max_fee_bps.map_or(fee_bps, |existing| existing.max(fee_bps)));
                    }

                    // Add to details map
                    mint_details_map.insert(
                        mint.clone(),
                        json!({
                            "has_freeze_authority": info.has_freeze_authority,
                            "freeze_authority": info.freeze_authority,
                            "mint_authority": info.mint_authority,
                            "supply": info.supply,
                            "decimals": info.decimals,
                            "is_token_2022": info.is_token_2022,
                            "extensions": {
                                "has_permanent_delegate": ext.has_permanent_delegate,
                                "permanent_delegate": ext.permanent_delegate_address,
                                "has_transfer_hook": ext.has_transfer_hook,
                                "transfer_hook_program": ext.transfer_hook_program,
                                "has_transfer_fee": ext.has_transfer_fee,
                                "transfer_fee_bps": ext.transfer_fee_basis_points,
                                "has_metadata_pointer": ext.has_metadata_pointer,
                                "has_default_frozen_state": ext.has_default_frozen_state,
                            }
                        }),
                    );

                    mint_infos.push(info);
                }
                Err(e) => {
                    log::warn!("Failed to fetch mint info for {}: {}", mint, e);
                    // Add error entry
                    mint_details_map.insert(
                        mint.clone(),
                        json!({
                            "error": format!("Failed to fetch: {}", e)
                        }),
                    );
                }
            }
        }

        let risky_count = mint_infos.iter().filter(|m| m.has_freeze_authority).count();
        let all_freezable =
            !mint_infos.is_empty() && mint_infos.iter().all(|m| m.has_freeze_authority);
        let any_freezable = mint_infos.iter().any(|m| m.has_freeze_authority);

        // Base fields
        fields.insert("mints_analyzed".to_string(), json!(mint_infos.len()));
        fields.insert("has_freeze_authority".to_string(), json!(any_freezable));
        fields.insert(
            "freeze_authority_addresses".to_string(),
            json!(freeze_authorities),
        );
        fields.insert(
            "mint_authority_addresses".to_string(),
            json!(mint_authorities),
        );
        fields.insert("risky_mint_count".to_string(), json!(risky_count));
        fields.insert("mint_details".to_string(), json!(mint_details_map));
        fields.insert("all_mints_freezable".to_string(), json!(all_freezable));
        fields.insert("any_mint_freezable".to_string(), json!(any_freezable));

        // Token-2022 fields
        fields.insert("is_token_2022".to_string(), json!(any_token_2022));
        fields.insert("any_token_2022".to_string(), json!(any_token_2022));

        // Extension threat detection - expose simple facts, let rules do the scoring
        let has_permanent_delegate = !permanent_delegate_addresses.is_empty();
        let has_transfer_hook = !transfer_hook_programs.is_empty();
        let has_transfer_fee = max_fee_bps.is_some();
        let has_metadata_pointer = mint_infos.iter().any(|m| m.extensions.has_metadata_pointer);
        let has_default_frozen = mint_infos
            .iter()
            .any(|m| m.extensions.has_default_frozen_state);

        fields.insert(
            "has_permanent_delegate".to_string(),
            json!(has_permanent_delegate),
        );
        fields.insert(
            "permanent_delegate_addresses".to_string(),
            json!(permanent_delegate_addresses),
        );
        fields.insert("has_transfer_hook".to_string(), json!(has_transfer_hook));
        fields.insert(
            "transfer_hook_programs".to_string(),
            json!(transfer_hook_programs),
        );
        fields.insert("has_transfer_fee".to_string(), json!(has_transfer_fee));
        fields.insert("max_transfer_fee_bps".to_string(), json!(max_fee_bps));
        fields.insert(
            "has_metadata_pointer".to_string(),
            json!(has_metadata_pointer),
        );
        fields.insert(
            "has_default_frozen_state".to_string(),
            json!(has_default_frozen),
        );

        Ok(fields)
    }

    fn estimated_latency_ms(&self) -> u64 {
        // First lookup: ~50-100ms per mint
        // Cached lookup: ~1ms
        // Return average assuming 50% cache hit rate
        50
    }
}
