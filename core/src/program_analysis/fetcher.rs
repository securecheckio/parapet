// Program bytecode fetcher - ported from txfilter
// Handles both upgradeable and non-upgradeable programs

use anyhow::{anyhow, Result};
use log::{debug, info, warn};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    commitment_config::CommitmentConfig,
};
use std::time::Instant;

use super::types::ProgramData;

pub struct ProgramFetcher {
    rpc_client: RpcClient,
}

impl ProgramFetcher {
    pub fn new(rpc_url: String) -> Self {
        Self {
            rpc_client: RpcClient::new(rpc_url),
        }
    }

    /// Fetch program data including bytecode
    pub async fn fetch_program(&self, program_address: &Pubkey) -> Result<ProgramData> {
        let start_time = Instant::now();
        info!("Fetching program data for: {}", program_address);

        // Get the account info
        let account = self.rpc_client
            .get_account_with_commitment(program_address, CommitmentConfig::confirmed())
            .map_err(|e| anyhow!("RPC error while fetching program account {}: {}", program_address, e))?
            .value
            .ok_or_else(|| anyhow!("Program account not found: {}. This program may not exist on the specified network.", program_address))?;

        debug!("Account owner: {}", account.owner);
        debug!("Account executable: {}", account.executable);
        debug!("Account data length: {}", account.data.len());

        // Check if this is an executable program
        if !account.executable {
            warn!("Account is not executable - this may not be a program");
        }

        let mut program_data = ProgramData {
            address: *program_address,
            executable_data: Vec::new(),
            is_executable: account.executable,
            is_upgradeable: false,
            authority: None,
            owner: account.owner,
            lamports: account.lamports,
        };

        // Handle different program types
        if self.is_bpf_loader_upgradeable(&account.owner) {
            // For upgradeable programs, we need to get the actual program data
            match self.fetch_upgradeable_program_data(program_address).await {
                Ok((data, authority)) => {
                    program_data.executable_data = data;
                    program_data.is_upgradeable = true;
                    program_data.authority = authority;
                }
                Err(e) => {
                    warn!("Failed to fetch upgradeable program data: {}", e);
                    // Fall back to using the account data directly
                    program_data.executable_data = account.data;
                }
            }
        } else if self.is_bpf_loader(&account.owner) {
            // For non-upgradeable BPF programs
            program_data.executable_data = account.data;
        } else {
            // For native programs or other types
            program_data.executable_data = account.data;
            debug!("Program owner {} is not a known BPF loader", account.owner);
        }

        let fetch_duration = start_time.elapsed();
        info!("Program data fetched in {:?}", fetch_duration);
        info!("Executable data size: {} bytes", program_data.executable_data.len());

        Ok(program_data)
    }

    /// Fetch upgradeable program's actual bytecode and authority
    async fn fetch_upgradeable_program_data(&self, program_address: &Pubkey) -> Result<(Vec<u8>, Option<Pubkey>)> {
        // For upgradeable programs, the program account contains metadata,
        // and we need to find the program data account
        
        // Try to parse the program account data as upgradeable loader state
        let account = self.rpc_client.get_account(program_address)?;
        
        if account.data.len() < 36 {
            return Err(anyhow!("Account data too small for upgradeable program"));
        }

        // The first 4 bytes indicate the account type, followed by the program data account pubkey
        let account_type = u32::from_le_bytes([
            account.data[0], account.data[1], account.data[2], account.data[3]
        ]);

        if account_type != 2 {
            return Err(anyhow!("Not an upgradeable program account"));
        }

        // Extract the program data account pubkey (bytes 4-36)
        let program_data_pubkey_bytes: [u8; 32] = account.data[4..36].try_into()
            .map_err(|_| anyhow!("Invalid program data pubkey"))?;
        let program_data_pubkey = Pubkey::new_from_array(program_data_pubkey_bytes);

        debug!("Program data account: {}", program_data_pubkey);

        // Fetch the program data account
        let program_data_account = self.rpc_client.get_account(&program_data_pubkey)?;
        
        // Program data account structure:
        // - 4 bytes: account type (should be 3 for program data)
        // - 32 bytes: upgrade authority (optional)
        // - 1 byte: authority option flag
        // - remaining bytes: actual program bytecode

        if program_data_account.data.len() < 37 {
            return Err(anyhow!("Program data account too small"));
        }

        let data_account_type = u32::from_le_bytes([
            program_data_account.data[0], 
            program_data_account.data[1], 
            program_data_account.data[2], 
            program_data_account.data[3]
        ]);

        if data_account_type != 3 {
            return Err(anyhow!("Invalid program data account type"));
        }

        // Check if there's an upgrade authority
        let has_authority = program_data_account.data[36] != 0;
        let authority = if has_authority {
            let authority_bytes: [u8; 32] = program_data_account.data[4..36].try_into()
                .map_err(|_| anyhow!("Invalid authority pubkey"))?;
            Some(Pubkey::new_from_array(authority_bytes))
        } else {
            None
        };

        // The actual program data starts at byte 37
        let program_data = program_data_account.data[37..].to_vec();

        Ok((program_data, authority))
    }

    fn is_bpf_loader_upgradeable(&self, owner: &Pubkey) -> bool {
        // BPF Loader Upgradeable program ID
        *owner == solana_sdk::bpf_loader_upgradeable::id()
    }

    fn is_bpf_loader(&self, owner: &Pubkey) -> bool {
        // BPF Loader v1 and v2 program IDs
        *owner == solana_sdk::bpf_loader::id() || 
        *owner == solana_sdk::bpf_loader_deprecated::id()
    }

    /// Validate RPC connection health
    pub fn validate_connection(&self) -> Result<()> {
        match self.rpc_client.get_health() {
            Ok(_) => {
                info!("RPC connection healthy");
                Ok(())
            }
            Err(e) => {
                Err(anyhow!("RPC connection failed: {}", e))
            }
        }
    }

    /// Get reference to underlying RPC client
    pub fn get_rpc_client(&self) -> &RpcClient {
        &self.rpc_client
    }
}
