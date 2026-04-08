use super::event::TransactionEvent;
use anyhow::Result;
use async_trait::async_trait;

/// Output formatter trait for different compliance/audit formats
#[async_trait]
pub trait OutputFormatter: Send + Sync {
    /// Format a transaction event
    fn format_event(&self, event: &TransactionEvent) -> Result<Vec<u8>>;

    /// Content type for HTTP sinks
    fn content_type(&self) -> &str;

    /// Formatter name
    fn name(&self) -> &str;
}
