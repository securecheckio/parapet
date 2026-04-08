/// Inner Instruction (CPI) Analyzer
/// 
/// Analyzes cross-program invocations (CPIs) to detect:
/// - Hidden token transfers in CPIs
/// - Unexpected program calls in instruction chains
/// - Complex CPI patterns that might hide malicious behavior
use anyhow::{Result, Context};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_sdk::transaction::Transaction;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::rules::analyzer::{ConfirmedTransactionMetadata, TransactionAnalyzer};

/// Structure for known-safe programs configuration file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownSafeProgramsConfig {
    pub version: String,
    pub name: String,
    pub description: Option<String>,
    pub feed_url: Option<String>,
    pub programs: Vec<SafeProgram>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeProgram {
    pub address: String,
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
}

/// Structure for known-safe program owners/deployers configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownSafeOwnersConfig {
    pub version: String,
    pub name: String,
    pub description: Option<String>,
    pub feed_url: Option<String>,
    pub owners: Vec<SafeOwner>,
    pub organizations: Option<Vec<SafeOrganization>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeOwner {
    pub address: String,
    pub name: String,
    #[serde(rename = "type")]
    pub owner_type: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeOrganization {
    pub name: String,
    pub description: Option<String>,
    pub addresses: Vec<String>,
    pub verified: Option<bool>,
}

pub struct InnerInstructionAnalyzer {
    /// Programs to ignore in deep scan (by program ID)
    known_safe_programs: HashSet<String>,
    /// Trusted program owners/deployers (by owner address)
    known_safe_owners: HashSet<String>,
    /// Where the known-safe lists were loaded from
    source: String,
}

impl InnerInstructionAnalyzer {
    /// Create analyzer with default known-safe programs from embedded config
    pub fn new() -> Self {
        Self::with_default_config()
    }
    
    /// Create analyzer with default known-safe programs
    pub fn with_default_config() -> Self {
        // Try to load from default location
        let default_paths = vec![
            "parapet/proxy/config/known-safe-programs.json",
            "../proxy/config/known-safe-programs.json",
            "../../proxy/config/known-safe-programs.json",
            "proxy/config/known-safe-programs.json",
        ];
        
        for path in default_paths {
            if let Ok(analyzer) = Self::from_file(path) {
                info!("✅ Loaded known-safe programs from: {}", path);
                return analyzer;
            }
        }
        
        // Fallback to minimal hardcoded list if file not found
        warn!("⚠️  Could not load known-safe-programs.json, using minimal fallback list");
        Self::with_minimal_defaults()
    }
    
    /// Create analyzer with minimal hardcoded defaults (fallback)
    fn with_minimal_defaults() -> Self {
        let minimal_safe = vec![
            "11111111111111111111111111111111",             // System Program
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // Token Program
            "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb", // Token-2022 Program
            "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL", // Associated Token Program
            "ComputeBudget111111111111111111111111111111",  // Compute Budget
        ];
        
        Self {
            known_safe_programs: minimal_safe.into_iter().map(String::from).collect(),
            known_safe_owners: HashSet::new(),
            source: "hardcoded-fallback".to_string(),
        }
    }
    
    /// Load known-safe owners from a JSON file
    fn load_owners_from_file<P: AsRef<Path>>(path: P) -> Result<HashSet<String>> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read known-safe owners from: {}", path.display()))?;
        
        let config: KnownSafeOwnersConfig = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse known-safe owners JSON from: {}", path.display()))?;
        
        let mut owners: HashSet<String> = config.owners
            .into_iter()
            .map(|o| o.address)
            .collect();
        
        // Add organization addresses
        if let Some(orgs) = config.organizations {
            for org in orgs {
                owners.extend(org.addresses);
            }
        }
        
        info!(
            "📋 Loaded {} known-safe owners from: {}",
            owners.len(),
            path.display()
        );
        
        Ok(owners)
    }
    
    /// Load known-safe programs from a JSON file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read known-safe programs from: {}", path.display()))?;
        
        let config: KnownSafeProgramsConfig = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse known-safe programs JSON from: {}", path.display()))?;
        
        let known_safe_programs: HashSet<String> = config.programs
            .into_iter()
            .map(|p| p.address)
            .collect();
        
        // Try to load owners from companion file
        let owners_path = path.parent()
            .map(|p| p.join("known-safe-owners.json"));
        
        let known_safe_owners = if let Some(ref owners_path) = owners_path {
            if owners_path.exists() {
                Self::load_owners_from_file(owners_path).unwrap_or_default()
            } else {
                HashSet::new()
            }
        } else {
            HashSet::new()
        };
        
        info!(
            "📋 Loaded {} known-safe programs + {} owners from: {}",
            known_safe_programs.len(),
            known_safe_owners.len(),
            path.display()
        );
        
        Ok(Self {
            known_safe_programs,
            known_safe_owners,
            source: path.display().to_string(),
        })
    }
    
    /// Load and merge known-safe programs from multiple sources
    pub fn with_custom_list<P: AsRef<Path>>(custom_path: P) -> Result<Self> {
        // Start with default list
        let mut analyzer = Self::with_default_config();
        
        // Load custom list
        let custom = Self::from_file(custom_path)?;
        
        // Merge the sets
        let original_programs = analyzer.known_safe_programs.len();
        let original_owners = analyzer.known_safe_owners.len();
        
        analyzer.known_safe_programs.extend(custom.known_safe_programs);
        analyzer.known_safe_owners.extend(custom.known_safe_owners);
        
        let new_programs = analyzer.known_safe_programs.len();
        let new_owners = analyzer.known_safe_owners.len();
        
        info!(
            "✅ Merged custom lists: {} programs ({} new), {} owners ({} new)",
            new_programs,
            new_programs - original_programs,
            new_owners,
            new_owners - original_owners
        );
        
        analyzer.source = format!("default + {}", custom.source);
        
        Ok(analyzer)
    }
    
    /// Get the number of known-safe programs loaded
    pub fn known_safe_count(&self) -> usize {
        self.known_safe_programs.len()
    }
    
    /// Get the number of known-safe owners loaded
    pub fn known_safe_owners_count(&self) -> usize {
        self.known_safe_owners.len()
    }
    
    /// Get the source of the known-safe programs list
    pub fn source(&self) -> &str {
        &self.source
    }
    
    /// Extract inner program IDs from confirmed transaction metadata.
    fn extract_inner_programs_from_metadata(
        &self,
        metadata: &ConfirmedTransactionMetadata,
    ) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        metadata
            .inner_instructions
            .iter()
            .filter_map(|ix| {
                if seen.insert(ix.program_id.clone()) {
                    Some(ix.program_id.clone())
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// Check if a program is in the known-safe list (by program ID)
    fn is_known_safe_program(&self, program_id: &str) -> bool {
        self.known_safe_programs.contains(program_id)
    }
    
    /// Check if a program is safe (by ID or owner)
    /// Note: Currently only checks by ID - owner checking requires on-chain data
    fn is_known_safe(&self, program_id: &str) -> bool {
        // Check if program itself is in safe list
        if self.is_known_safe_program(program_id) {
            return true;
        }
        
        // TODO: Fetch program's upgrade authority/owner and check if it's in safe owners list
        // This requires on-chain RPC call, so we'll implement it when we have the infrastructure
        
        false
    }
    
    /// Calculate depth score (more nested CPIs = higher risk)
    fn calculate_depth_score(&self, inner_programs: &[String]) -> u32 {
        // Each level of nesting adds risk
        inner_programs.len() as u32
    }
    
    /// Calculate unknown program score
    fn calculate_unknown_score(&self, inner_programs: &[String]) -> u32 {
        inner_programs
            .iter()
            .filter(|p| !self.is_known_safe(p))
            .count() as u32
    }
}

impl Default for InnerInstructionAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TransactionAnalyzer for InnerInstructionAnalyzer {
    fn name(&self) -> &str {
        "inner_instruction"
    }

    fn fields(&self) -> Vec<String> {
        vec![
            "inner_program_count".to_string(),
            "inner_programs".to_string(),
            "unknown_inner_programs".to_string(),
            "unknown_program_count".to_string(),
            "cpi_depth".to_string(),
            "has_unknown_inner_programs".to_string(),
            "cpi_depth_score".to_string(),
            "cpi_risk_score".to_string(),
        ]
    }

    fn is_available(&self) -> bool {
        true // Always available, no external dependencies
    }

    async fn analyze(&self, _tx: &Transaction) -> Result<HashMap<String, Value>> {
        // Without metadata, inner instructions are unavailable — return empty/zero fields
        let mut result = HashMap::new();
        result.insert("inner_program_count".to_string(), json!(0));
        result.insert("inner_programs".to_string(), json!(Vec::<String>::new()));
        result.insert("unknown_inner_programs".to_string(), json!(Vec::<String>::new()));
        result.insert("unknown_program_count".to_string(), json!(0));
        result.insert("cpi_depth".to_string(), json!(0));
        result.insert("has_unknown_inner_programs".to_string(), json!(false));
        result.insert("cpi_depth_score".to_string(), json!(0));
        result.insert("cpi_risk_score".to_string(), json!(0));
        Ok(result)
    }

    async fn analyze_with_metadata(
        &self,
        _tx: &Transaction,
        metadata: &ConfirmedTransactionMetadata,
    ) -> Result<HashMap<String, Value>> {
        let inner_programs = self.extract_inner_programs_from_metadata(metadata);

        debug!(
            "InnerInstructionAnalyzer: found {} inner programs in transaction",
            inner_programs.len()
        );

        let unknown_programs: Vec<String> = inner_programs
            .iter()
            .filter(|p| !self.is_known_safe(p))
            .cloned()
            .collect();

        let depth_score = self.calculate_depth_score(&inner_programs);
        let unknown_score = self.calculate_unknown_score(&inner_programs);

        let has_unknown_programs = !unknown_programs.is_empty();
        let risk_score = depth_score + (unknown_score * 2);

        let mut result = HashMap::new();
        result.insert("inner_program_count".to_string(), json!(inner_programs.len()));
        result.insert("inner_programs".to_string(), json!(inner_programs));
        result.insert("unknown_inner_programs".to_string(), json!(unknown_programs));
        result.insert("unknown_program_count".to_string(), json!(unknown_programs.len()));
        result.insert("cpi_depth".to_string(), json!(inner_programs.len()));
        result.insert("has_unknown_inner_programs".to_string(), json!(has_unknown_programs));
        result.insert("cpi_depth_score".to_string(), json!(depth_score));
        result.insert("cpi_risk_score".to_string(), json!(risk_score));

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_safe_programs() {
        let analyzer = InnerInstructionAnalyzer::new();
        assert!(analyzer.is_known_safe("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"));
        assert!(!analyzer.is_known_safe("UnknownProgram111111111111111111111111111"));
    }

    #[test]
    fn test_depth_score() {
        let analyzer = InnerInstructionAnalyzer::new();
        let programs = vec!["prog1".to_string(), "prog2".to_string(), "prog3".to_string()];
        assert_eq!(analyzer.calculate_depth_score(&programs), 3);
    }

    #[test]
    fn test_unknown_score() {
        let analyzer = InnerInstructionAnalyzer::new();
        let programs = vec![
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
            "UnknownProgram111111111111111111111111111".to_string(),
        ];
        assert_eq!(analyzer.calculate_unknown_score(&programs), 1);
    }
}
