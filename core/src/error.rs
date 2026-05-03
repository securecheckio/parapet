//! Typed errors for `parapet-core` public APIs (`RuleEngine`, rule loading, evaluation).
//!
//! Downstream crates using `anyhow::Result` can still use `?` on these errors:
//! `ParapetCoreError` implements [`std::error::Error`] and converts into [`anyhow::Error`].

use std::path::PathBuf;
use thiserror::Error;

/// Errors returned by the rule engine and related rule loading/evaluation paths.
#[derive(Debug, Error)]
pub enum ParapetCoreError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("could not load rules from {path}: {detail}")]
    RuleParse { path: String, detail: String },

    #[error("rule path is not valid UTF-8: {0}")]
    InvalidPath(PathBuf),

    #[error("no valid rule JSON files found in directory: {0}")]
    NoRulesInDirectory(String),

    #[error("rule validation: {0}")]
    RuleValidation(String),

    #[error("evaluation: {0}")]
    Evaluation(String),

    #[error("analyzer: {0}")]
    Analyzer(String),

    /// Bytecode fetch, disassembly, semantic/AI orchestration, or related I/O.
    #[error("program analysis: {0}")]
    ProgramAnalysis(String),
}

impl ParapetCoreError {
    pub(crate) fn rule_validation(msg: impl Into<String>) -> Self {
        Self::RuleValidation(msg.into())
    }

    pub(crate) fn evaluation(msg: impl Into<String>) -> Self {
        Self::Evaluation(msg.into())
    }

    pub(crate) fn analyzer(msg: impl Into<String>) -> Self {
        Self::Analyzer(msg.into())
    }
}
