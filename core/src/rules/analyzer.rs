use anyhow::Result;
use serde_json::Value;
use solana_sdk::transaction::Transaction;
use std::collections::HashMap;
use std::sync::Arc;

/// A single inner (CPI) instruction from confirmed transaction metadata.
#[derive(Debug, Clone)]
pub struct ConfirmedInnerInstruction {
    /// Index of the outer instruction that triggered this CPI
    pub outer_index: u8,
    /// Program ID (base58) of the called program
    pub program_id: String,
    /// Raw instruction data bytes
    pub data: Vec<u8>,
    /// Account indices (into the transaction's account_keys)
    pub accounts: Vec<u8>,
    /// CPI call stack depth
    pub stack_height: Option<u8>,
}

/// All confirmed transaction metadata available after a transaction lands on-chain.
/// Passed to analyze_with_metadata so analyzers can use post-execution data
/// without trusting the RPC for the transaction bytes themselves.
#[derive(Debug, Clone, Default)]
pub struct ConfirmedTransactionMetadata {
    /// Log messages from meta.logMessages
    pub logs: Vec<String>,
    /// Inner (CPI) instructions from meta.innerInstructions
    pub inner_instructions: Vec<ConfirmedInnerInstruction>,
}

/// Trait for transaction analyzers that extract fields from transactions
#[async_trait::async_trait]
pub trait TransactionAnalyzer: Send + Sync {
    /// Name of this analyzer (used in rules)
    fn name(&self) -> &str;

    /// List of fields this analyzer provides
    fn fields(&self) -> Vec<String>;

    /// Analyze a transaction and return field values
    async fn analyze(&self, tx: &Transaction) -> Result<HashMap<String, Value>>;

    /// Analyze a transaction with confirmed transaction metadata (logs + inner instructions).
    /// Default implementation delegates to analyze() and ignores metadata.
    /// Override this in analyzers that consume post-execution data.
    async fn analyze_with_metadata(
        &self,
        tx: &Transaction,
        metadata: &ConfirmedTransactionMetadata,
    ) -> Result<HashMap<String, Value>> {
        let _ = metadata;
        self.analyze(tx).await
    }

    /// Convenience wrapper — analyze with logs only (no inner instructions).
    /// Prefer analyze_with_metadata when both are available.
    async fn analyze_with_logs(
        &self,
        tx: &Transaction,
        logs: &[String],
    ) -> Result<HashMap<String, Value>> {
        let metadata = ConfirmedTransactionMetadata {
            logs: logs.to_vec(),
            inner_instructions: vec![],
        };
        self.analyze_with_metadata(tx, &metadata).await
    }

    /// Whether this analyzer is currently available
    fn is_available(&self) -> bool {
        true
    }

    /// Estimated latency in milliseconds (for single call)
    fn estimated_latency_ms(&self) -> u64 {
        1
    }

    /// Recommended delay between calls in milliseconds (for rate limiting)
    /// Returns None if no rate limit, or Some(ms) for required delay
    fn recommended_delay_ms(&self) -> Option<u64> {
        None
    }
}

/// Registry for managing analyzers
pub struct AnalyzerRegistry {
    analyzers: HashMap<String, Arc<dyn TransactionAnalyzer>>,
}

impl AnalyzerRegistry {
    pub fn new() -> Self {
        Self {
            analyzers: HashMap::new(),
        }
    }

    pub fn register(&mut self, analyzer: Arc<dyn TransactionAnalyzer>) {
        let name = analyzer.name().to_string();
        log::info!(
            "📋 Registered analyzer: {} ({} fields)",
            name,
            analyzer.fields().len()
        );
        self.analyzers.insert(name, analyzer);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn TransactionAnalyzer>> {
        self.analyzers.get(name).cloned()
    }

    pub fn list_all(&self) -> Vec<String> {
        self.analyzers.keys().cloned().collect()
    }

    /// Get all available fields across all analyzers
    pub fn get_all_fields(&self) -> std::collections::HashMap<String, Vec<String>> {
        let mut result = std::collections::HashMap::new();

        for (name, analyzer) in &self.analyzers {
            result.insert(name.clone(), analyzer.fields());
        }

        result
    }

    /// Check if a field is available (with or without prefix)
    pub fn has_field(&self, field: &str) -> bool {
        // Check if it's a prefixed field (analyzer:field)
        if let Some((analyzer_name, field_name)) = field.split_once(':') {
            if let Some(analyzer) = self.get(analyzer_name) {
                return analyzer.fields().contains(&field_name.to_string());
            }
            return false;
        }

        // Check all analyzers for unprefixed field
        for analyzer in self.analyzers.values() {
            if analyzer.fields().contains(&field.to_string()) {
                return true;
            }
        }

        false
    }

    /// Analyze transaction with selected analyzers (lazy evaluation)
    /// Only runs analyzers that are in the required_analyzers list
    pub async fn analyze_selected(
        &self,
        tx: &Transaction,
        required_analyzers: &[String],
    ) -> Result<HashMap<String, Value>> {
        let mut all_fields = HashMap::new();

        // Filter to only required analyzers
        let tasks: Vec<_> = self
            .analyzers
            .iter()
            .filter(|(name, analyzer)| analyzer.is_available() && required_analyzers.contains(name))
            .map(|(name, analyzer)| {
                let name = name.clone();
                let analyzer = Arc::clone(analyzer);
                let tx = tx.clone();

                tokio::spawn(async move {
                    let start = std::time::Instant::now();
                    let result = analyzer.analyze(&tx).await;
                    let duration = start.elapsed();

                    if duration.as_millis() > 100 {
                        log::debug!("Analyzer {} took {}ms", name, duration.as_millis());
                    }

                    (name, result)
                })
            })
            .collect();

        // Wait for all analyzers to complete in parallel
        let results = futures::future::join_all(tasks).await;

        for task_result in results {
            match task_result {
                Ok((name, Ok(fields))) => {
                    // Prefix fields with analyzer name to avoid conflicts
                    for (field, value) in fields {
                        let prefixed_key = format!("{}:{}", name, &field);
                        all_fields.insert(prefixed_key, value.clone());
                        // Also add without prefix for convenience
                        all_fields.entry(field.clone()).or_insert(value);
                    }
                }
                Ok((name, Err(e))) => {
                    log::warn!("Analyzer {} failed: {}", name, e);
                }
                Err(e) => {
                    log::error!("Analyzer task failed: {}", e);
                }
            }
        }

        Ok(all_fields)
    }

    /// Analyze transaction with all registered analyzers (backwards compatibility)
    pub async fn analyze_all(&self, tx: &Transaction) -> Result<HashMap<String, Value>> {
        let all_analyzers: Vec<String> = self.analyzers.keys().cloned().collect();
        self.analyze_selected(tx, &all_analyzers).await
    }

    /// Analyze transaction with selected analyzers, passing confirmed transaction metadata.
    /// Calls analyze_with_metadata on each analyzer — metadata-aware analyzers use logs
    /// and inner instructions; all others fall back to their standard analyze() implementation.
    pub async fn analyze_selected_with_metadata(
        &self,
        tx: &Transaction,
        required_analyzers: &[String],
        metadata: &ConfirmedTransactionMetadata,
    ) -> Result<HashMap<String, Value>> {
        let mut all_fields = HashMap::new();

        let tasks: Vec<_> = self
            .analyzers
            .iter()
            .filter(|(name, analyzer)| analyzer.is_available() && required_analyzers.contains(name))
            .map(|(name, analyzer)| {
                let name = name.clone();
                let analyzer = Arc::clone(analyzer);
                let tx = tx.clone();
                let metadata = metadata.clone();

                tokio::spawn(async move {
                    let start = std::time::Instant::now();
                    let result = analyzer.analyze_with_metadata(&tx, &metadata).await;
                    let duration = start.elapsed();

                    if duration.as_millis() > 100 {
                        log::debug!("Analyzer {} took {}ms", name, duration.as_millis());
                    }

                    (name, result)
                })
            })
            .collect();

        let results = futures::future::join_all(tasks).await;

        for task_result in results {
            match task_result {
                Ok((name, Ok(fields))) => {
                    for (field, value) in fields {
                        let prefixed_key = format!("{}:{}", name, &field);
                        all_fields.insert(prefixed_key, value.clone());
                        all_fields.entry(field.clone()).or_insert(value);
                    }
                }
                Ok((name, Err(e))) => {
                    log::warn!("Analyzer {} failed: {}", name, e);
                }
                Err(e) => {
                    log::error!("Analyzer task failed: {}", e);
                }
            }
        }

        Ok(all_fields)
    }

    /// Convenience wrapper — analyze with logs only.
    pub async fn analyze_selected_with_logs(
        &self,
        tx: &Transaction,
        required_analyzers: &[String],
        logs: &[String],
    ) -> Result<HashMap<String, Value>> {
        let metadata = ConfirmedTransactionMetadata {
            logs: logs.to_vec(),
            inner_instructions: vec![],
        };
        self.analyze_selected_with_metadata(tx, required_analyzers, &metadata).await
    }

    /// Get the recommended delay for rate limiting across all active analyzers
    /// Returns the maximum (slowest) delay to coordinate all rate limits
    pub fn get_recommended_delay_ms(&self) -> u64 {
        self.analyzers
            .values()
            .filter(|analyzer| analyzer.is_available())
            .filter_map(|analyzer| analyzer.recommended_delay_ms())
            .max()
            .unwrap_or(0)  // No rate limits = no delay needed
    }
}

impl Default for AnalyzerRegistry {
    fn default() -> Self {
        Self::new()
    }
}
