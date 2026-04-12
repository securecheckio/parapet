pub mod analyzer;
pub mod analyzer_config;
pub mod analyzers;
pub mod dynamic;
pub mod engine;
pub mod feed_updater;
pub mod flowbits;
pub mod performance;
pub mod types;
pub mod wasm_analyzer;
pub mod wasm_config;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_analyzer;

#[cfg(test)]
mod tests_types;

pub use analyzer::{AnalyzerRegistry, TransactionAnalyzer};
pub use analyzer_config::{AnalyzerConfig, AnalyzersConfig};
pub use dynamic::{DynamicRule, DynamicRuleStore, RuleSource};
pub use engine::RuleEngine;
pub use feed_updater::{
    FeedConfig, FeedSource, FeedUpdater, FetchResult, MergedRuleFeed, RuleFeed,
};
pub use flowbits::{FlowbitStateManager, FlowbitValue};
pub use performance::{
    AnalyzerPerformanceMetrics, EnginePerformanceMetrics, PerformanceTracker,
    RulePerformanceMetrics,
};
pub use types::{MatchedRule, Rule, RuleAction, RuleCondition, RuleDecision, RuleDefinition};
pub use wasm_analyzer::load_wasm_analyzers_from_dir;
