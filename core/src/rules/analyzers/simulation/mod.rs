pub mod balance;
pub mod compute;
pub mod cpi;
pub mod failure;
pub mod logs;
pub mod token_balance;

#[cfg(test)]
mod tests;

pub use balance::SimulationBalanceAnalyzer;
pub use compute::SimulationComputeAnalyzer;
pub use cpi::SimulationCpiAnalyzer;
pub use failure::SimulationFailureAnalyzer;
pub use logs::SimulationLogAnalyzer;
pub use token_balance::SimulationTokenAnalyzer;

use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;

/// Trait for analyzers that extract fields from simulation responses
#[async_trait::async_trait]
pub trait SimulationAnalyzer: Send + Sync {
    /// Name of this analyzer (used in rules)
    fn name(&self) -> &str;

    /// List of fields this analyzer provides
    fn fields(&self) -> Vec<String>;

    /// Analyze a simulation response and return field values
    /// The response should be the "value" object from the simulation result
    async fn analyze(&self, simulation_result: &Value) -> Result<HashMap<String, Value>>;

    /// Whether this analyzer is currently available
    fn is_available(&self) -> bool {
        true
    }

    /// Estimated latency in milliseconds (for single call)
    fn estimated_latency_ms(&self) -> u64 {
        1
    }
}

/// Registry for managing simulation analyzers
pub struct SimulationAnalyzerRegistry {
    analyzers: HashMap<String, Box<dyn SimulationAnalyzer>>,
}

impl SimulationAnalyzerRegistry {
    pub fn new() -> Self {
        Self {
            analyzers: HashMap::new(),
        }
    }

    pub fn register(&mut self, analyzer: Box<dyn SimulationAnalyzer>) {
        let name = analyzer.name().to_string();
        log::info!(
            "📋 Registered simulation analyzer: {} ({} fields)",
            name,
            analyzer.fields().len()
        );
        self.analyzers.insert(name, analyzer);
    }

    pub fn get(&self, name: &str) -> Option<&dyn SimulationAnalyzer> {
        self.analyzers.get(name).map(|a| a.as_ref())
    }

    pub fn list_all(&self) -> Vec<String> {
        self.analyzers.keys().cloned().collect()
    }

    /// Analyze simulation response with all registered analyzers
    pub async fn analyze_all(&self, simulation_result: &Value) -> Result<HashMap<String, Value>> {
        let mut all_fields = HashMap::new();

        for (name, analyzer) in &self.analyzers {
            if !analyzer.is_available() {
                continue;
            }

            match analyzer.analyze(simulation_result).await {
                Ok(fields) => {
                    // Prefix fields with analyzer name to avoid conflicts
                    for (field, value) in fields {
                        let prefixed_key = format!("{}:{}", name, &field);
                        all_fields.insert(prefixed_key, value.clone());
                        // Also add without prefix for convenience
                        all_fields.entry(field.clone()).or_insert(value);
                    }
                }
                Err(e) => {
                    log::warn!("Simulation analyzer {} failed: {}", name, e);
                }
            }
        }

        Ok(all_fields)
    }
}

impl Default for SimulationAnalyzerRegistry {
    fn default() -> Self {
        Self::new()
    }
}
