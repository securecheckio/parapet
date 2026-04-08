use crate::output::sink::{OutputSink, SinkMetadata};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use std::collections::HashMap;

/// HTTP sink for sending formatted events to remote endpoints
pub struct HttpSink {
    url: String,
    headers: HashMap<String, String>,
    client: Client,
}

impl HttpSink {
    pub fn new(url: String, headers: HashMap<String, String>) -> Self {
        Self {
            url,
            headers,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl OutputSink for HttpSink {
    async fn write(&self, data: &[u8], metadata: &SinkMetadata) -> Result<()> {
        let mut request = self
            .client
            .post(&self.url)
            .header("Content-Type", &metadata.content_type)
            .body(data.to_vec());

        // Add custom headers
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            anyhow::bail!(
                "HTTP sink failed: {} - {}",
                response.status(),
                response.text().await?
            );
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "http"
    }
}
