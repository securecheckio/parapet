//! SecureCheck Engine - Fast Transaction Security Analysis for Solana
//!
//! A rule-based security analysis engine that detects common drain attacks
//! in Solana transactions using configurable analyzers.
//!
//! # Features
//!
//! - **Fast**: Sub-50ms analysis (no RPC calls for core analyzers)
//! - **Accurate**: Detects unlimited delegations, authority changes, suspicious patterns
//! - **Portable**: Use in RPC proxies, mobile apps, CLIs, or any Rust project
//! - **Configurable**: JSON-defined rules with pluggable analyzers
//!
//! # Example
//!
//! ```ignore
//! use sol_shield::rules::{RuleEngine, AnalyzerRegistry};
//! use sol_shield::rules::analyzers::*;
//! use std::sync::Arc;
//!
//! // Create analyzer registry
//! let mut registry = AnalyzerRegistry::new();
//! registry.register(Arc::new(BasicAnalyzer::new()));
//! registry.register(Arc::new(CoreSecurityAnalyzer::new(std::collections::HashSet::new())));
//!
//! // Create rule engine and load rules
//! let mut engine = RuleEngine::new(registry);
//! engine.load_rules_from_file("./rules/default.json").unwrap();
//! ```

// Rules engine module
pub mod rules;
pub mod enrichment;

// Program analysis module (optional)
#[cfg(feature = "program-analysis")]
pub mod program_analysis;

/// Risk level classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}
