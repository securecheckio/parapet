use solana_sdk::{
    message::compiled_instruction::CompiledInstruction,
    message::VersionedMessage,
    pubkey::Pubkey,
    transaction::{Transaction, VersionedTransaction},
};
use solana_sdk_ids::{compute_budget, system_program};
use std::collections::HashMap;

/// Trait for program-specific decoders (pluggable architecture)
pub trait ProgramDecoder: Send + Sync {
    fn program_id(&self) -> Pubkey;
    fn program_name(&self) -> &str;
    fn decode(
        &self,
        ix: &CompiledInstruction,
        account_keys: &[Pubkey],
    ) -> Option<DecodedInstruction>;
}

/// Registry for instruction decoders
pub struct DecoderRegistry {
    decoders: HashMap<Pubkey, Box<dyn ProgramDecoder>>,
    program_names: HashMap<Pubkey, String>,
}

impl DecoderRegistry {
    pub fn new() -> Self {
        Self {
            decoders: HashMap::new(),
            program_names: Self::init_known_programs(),
        }
    }

    /// Create registry with default base Solana program decoders
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        // Register built-in decoders for base Solana programs
        registry.register(Box::new(SystemProgramDecoder));
        registry.register(Box::new(TokenProgramDecoder));
        registry.register(Box::new(ComputeBudgetDecoder));

        registry
    }

    /// Register a decoder
    pub fn register(&mut self, decoder: Box<dyn ProgramDecoder>) {
        let program_id = decoder.program_id();
        let name = decoder.program_name().to_string();

        log::info!("📝 Registered decoder: {} ({})", name, program_id);

        self.decoders.insert(program_id, decoder);
        self.program_names.insert(program_id, name);
    }

    /// Decode a full transaction
    pub fn decode_transaction(&self, tx: &Transaction) -> Vec<DecodedInstruction> {
        tx.message
            .instructions
            .iter()
            .map(|ix| self.decode_instruction(ix, &tx.message.account_keys))
            .collect()
    }

    /// Decode a versioned transaction
    pub fn decode_versioned_transaction(
        &self,
        tx: &VersionedTransaction,
    ) -> Vec<DecodedInstruction> {
        let (instructions, account_keys) = match &tx.message {
            VersionedMessage::V0(v0_msg) => (
                v0_msg.instructions.as_slice(),
                v0_msg.account_keys.as_slice(),
            ),
            VersionedMessage::Legacy(legacy_msg) => (
                legacy_msg.instructions.as_slice(),
                legacy_msg.account_keys.as_slice(),
            ),
            VersionedMessage::V1(v1_msg) => (
                v1_msg.instructions.as_slice(),
                v1_msg.account_keys.as_slice(),
            ),
        };

        instructions
            .iter()
            .map(|ix| self.decode_instruction(ix, account_keys))
            .collect()
    }

    /// Decode a single instruction
    pub fn decode_instruction(
        &self,
        ix: &CompiledInstruction,
        account_keys: &[Pubkey],
    ) -> DecodedInstruction {
        let Some(program_id) = account_keys.get(ix.program_id_index as usize).copied() else {
            return DecodedInstruction::Unknown {
                program_id: Pubkey::default(),
                program_name: Some("invalid_program_id_index".to_string()),
                instruction_data: hex::encode(&ix.data),
                account_count: ix.accounts.len(),
            };
        };

        // Try registered decoder
        if let Some(decoder) = self.decoders.get(&program_id) {
            if let Some(decoded) = decoder.decode(ix, account_keys) {
                return decoded;
            }
        }

        // Unknown program - display as raw data
        DecodedInstruction::Unknown {
            program_id,
            program_name: self.program_names.get(&program_id).cloned(),
            instruction_data: hex::encode(&ix.data),
            account_count: ix.accounts.len(),
        }
    }

    /// Initialize map of known program names
    fn init_known_programs() -> HashMap<Pubkey, String> {
        let mut map = HashMap::new();

        // Common programs
        if let Ok(pid) = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".parse() {
            map.insert(pid, "Jupiter Aggregator v6".to_string());
        }
        if let Ok(pid) = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc".parse() {
            map.insert(pid, "Orca Whirlpool".to_string());
        }
        if let Ok(pid) = "MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD".parse() {
            map.insert(pid, "Marinade Staking".to_string());
        }

        map
    }
}

impl Default for DecoderRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Decoded instruction representation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DecodedInstruction {
    Transfer {
        from: String,
        to: String,
        amount_lamports: u64,
    },
    TokenTransfer {
        from: String,
        to: String,
        amount: u64,
        mint: Option<String>,
    },
    TokenApprove {
        owner: String,
        delegate: String,
        amount: u64,
    },
    ComputeBudget {
        compute_units: Option<u32>,
        priority_fee_lamports: Option<u64>,
    },
    Unknown {
        program_id: Pubkey,
        program_name: Option<String>,
        instruction_data: String,
        account_count: usize,
    },
}

impl DecodedInstruction {
    pub fn to_human_readable(&self) -> String {
        fn short_addr(addr: &str) -> String {
            if addr.len() >= 8 {
                addr[..8].to_string()
            } else {
                addr.to_string()
            }
        }
        match self {
            Self::Transfer {
                from,
                to,
                amount_lamports,
            } => {
                format!(
                    "Transfer {} SOL from {}... to {}...",
                    *amount_lamports as f64 / 1_000_000_000.0,
                    short_addr(from),
                    short_addr(to)
                )
            }
            Self::TokenTransfer {
                from, to, amount, ..
            } => {
                format!(
                    "Token transfer: {} tokens from {}... to {}...",
                    amount,
                    short_addr(from),
                    short_addr(to)
                )
            }
            Self::TokenApprove {
                owner,
                delegate,
                amount,
            } => {
                format!(
                    "Token approve: {} tokens to {}... (owner: {}...)",
                    amount,
                    short_addr(delegate),
                    short_addr(owner)
                )
            }
            Self::ComputeBudget {
                compute_units,
                priority_fee_lamports,
            } => {
                let mut parts = Vec::new();
                if let Some(cu) = compute_units {
                    parts.push(format!("{} CU", cu));
                }
                if let Some(fee) = priority_fee_lamports {
                    parts.push(format!("{} lamports priority fee", fee));
                }
                format!("Compute Budget: {}", parts.join(", "))
            }
            Self::Unknown {
                program_id,
                program_name,
                instruction_data,
                account_count,
            } => {
                let name = program_name.as_deref().unwrap_or("Unknown Program");
                let pid = program_id.to_string();
                format!(
                    "{} ({}...): {} bytes data, {} accounts",
                    name,
                    short_addr(&pid),
                    instruction_data.len() / 2,
                    account_count
                )
            }
        }
    }
}

// ============================================================================
// Built-in Decoders for Base Solana Programs
// ============================================================================

/// System Program Decoder
pub struct SystemProgramDecoder;

impl ProgramDecoder for SystemProgramDecoder {
    fn program_id(&self) -> Pubkey {
        system_program::id()
    }

    fn program_name(&self) -> &str {
        "System Program"
    }

    fn decode(
        &self,
        ix: &CompiledInstruction,
        account_keys: &[Pubkey],
    ) -> Option<DecodedInstruction> {
        if ix.data.len() < 4 {
            return None;
        }

        let discriminator = u32::from_le_bytes(ix.data[0..4].try_into().ok()?);

        match discriminator {
            2 => {
                // Transfer
                if ix.data.len() < 12 || ix.accounts.len() < 2 {
                    return None;
                }

                let lamports = u64::from_le_bytes(ix.data[4..12].try_into().ok()?);
                let from = *account_keys.get(ix.accounts[0] as usize)?;
                let to = *account_keys.get(ix.accounts[1] as usize)?;

                Some(DecodedInstruction::Transfer {
                    from: from.to_string(),
                    to: to.to_string(),
                    amount_lamports: lamports,
                })
            }
            _ => None,
        }
    }
}

/// SPL Token Program Decoder
pub struct TokenProgramDecoder;

impl ProgramDecoder for TokenProgramDecoder {
    fn program_id(&self) -> Pubkey {
        // `spl_token_interface::id()` is the canonical SPL Token program address; bridge to
        // `solana_sdk::Pubkey` (different `solana-pubkey` major lines in the dependency graph).
        Pubkey::new_from_array(*spl_token_interface::id().as_array())
    }

    fn program_name(&self) -> &str {
        "SPL Token Program"
    }

    fn decode(
        &self,
        ix: &CompiledInstruction,
        account_keys: &[Pubkey],
    ) -> Option<DecodedInstruction> {
        if ix.data.is_empty() {
            return None;
        }

        let instruction_type = ix.data[0];

        match instruction_type {
            3 => {
                // Transfer
                if ix.data.len() < 9 || ix.accounts.len() < 3 {
                    return None;
                }

                let amount = u64::from_le_bytes(ix.data[1..9].try_into().ok()?);
                let from = *account_keys.get(ix.accounts[0] as usize)?;
                let to = *account_keys.get(ix.accounts[1] as usize)?;

                Some(DecodedInstruction::TokenTransfer {
                    from: from.to_string(),
                    to: to.to_string(),
                    amount,
                    mint: None,
                })
            }
            4 => {
                // Approve
                if ix.data.len() < 9 || ix.accounts.len() < 3 {
                    return None;
                }

                let amount = u64::from_le_bytes(ix.data[1..9].try_into().ok()?);
                let owner = *account_keys.get(ix.accounts[0] as usize)?;
                let delegate = *account_keys.get(ix.accounts[1] as usize)?;

                Some(DecodedInstruction::TokenApprove {
                    owner: owner.to_string(),
                    delegate: delegate.to_string(),
                    amount,
                })
            }
            _ => None,
        }
    }
}

/// Compute Budget Program Decoder
pub struct ComputeBudgetDecoder;

impl ProgramDecoder for ComputeBudgetDecoder {
    fn program_id(&self) -> Pubkey {
        compute_budget::id()
    }

    fn program_name(&self) -> &str {
        "Compute Budget"
    }

    fn decode(
        &self,
        ix: &CompiledInstruction,
        _account_keys: &[Pubkey],
    ) -> Option<DecodedInstruction> {
        if ix.data.is_empty() {
            return None;
        }

        let instruction_type = ix.data[0];

        match instruction_type {
            0 => {
                // Request units (deprecated)
                if ix.data.len() >= 9 {
                    let units = u32::from_le_bytes(ix.data[1..5].try_into().ok()?);
                    let additional_fee = u32::from_le_bytes(ix.data[5..9].try_into().ok()?);

                    return Some(DecodedInstruction::ComputeBudget {
                        compute_units: Some(units),
                        priority_fee_lamports: Some(additional_fee as u64),
                    });
                }
                None
            }
            2 => {
                // Set compute unit limit
                if ix.data.len() >= 5 {
                    let units = u32::from_le_bytes(ix.data[1..5].try_into().ok()?);
                    return Some(DecodedInstruction::ComputeBudget {
                        compute_units: Some(units),
                        priority_fee_lamports: None,
                    });
                }
                None
            }
            3 => {
                // Set compute unit price
                if ix.data.len() >= 9 {
                    let micro_lamports = u64::from_le_bytes(ix.data[1..9].try_into().ok()?);
                    return Some(DecodedInstruction::ComputeBudget {
                        compute_units: None,
                        priority_fee_lamports: Some(micro_lamports),
                    });
                }
                None
            }
            _ => None,
        }
    }
}
