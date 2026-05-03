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
//! ```no_run
//! use parapet_core::rules::analyzers::{BasicAnalyzer, CoreSecurityAnalyzer};
//! use parapet_core::rules::{AnalyzerRegistry, RuleEngine};
//! use std::sync::Arc;
//!
//! let mut registry = AnalyzerRegistry::new();
//! registry.register(Arc::new(BasicAnalyzer::new()));
//! registry.register(Arc::new(CoreSecurityAnalyzer::new(std::collections::HashSet::new())));
//!
//! let mut engine = RuleEngine::new(registry);
//! engine.load_rules_from_file("./rules/default.json").expect("load rules");
//! ```

// Typed errors for library consumers
pub mod error;

// Rules engine module
pub mod enrichment;
pub mod rules;

pub use error::ParapetCoreError;

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
