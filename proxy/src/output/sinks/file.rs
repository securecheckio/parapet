use crate::output::sink::{OutputSink, SinkMetadata};
use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;

/// File sink for writing formatted events to filesystem
pub struct FileSink {
    base_path: PathBuf,
}

impl FileSink {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }
}

#[async_trait]
impl OutputSink for FileSink {
    async fn write(&self, data: &[u8], metadata: &SinkMetadata) -> Result<()> {
        // Create directory if needed
        fs::create_dir_all(&self.base_path).await?;

        // Generate filename based on formatter and date
        let now = chrono::Utc::now();
        let filename = format!("{}-{}.log", metadata.formatter_name, now.format("%Y%m%d"));

        let file_path = self.base_path.join(&filename);

        log::debug!("Writing event to: {:?}", file_path);

        // Append to file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await?;

        file.write_all(data).await?;
        file.flush().await?;

        log::debug!("Successfully wrote {} bytes to {:?}", data.len(), file_path);

        Ok(())
    }

    fn name(&self) -> &str {
        "file"
    }
}
