use anyhow::Result;
use async_trait::async_trait;

/// Metadata for sink writes
pub struct SinkMetadata {
    pub content_type: String,
    pub formatter_name: String,
}

/// Output sink trait for different destinations
#[async_trait]
pub trait OutputSink: Send + Sync {
    /// Write formatted data to sink
    async fn write(&self, data: &[u8], metadata: &SinkMetadata) -> Result<()>;

    /// Sink name for logging
    fn name(&self) -> &str;
}
