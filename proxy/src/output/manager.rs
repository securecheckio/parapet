use super::sink::SinkMetadata;
use super::{OutputFormatter, OutputSink, TransactionEvent};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

/// Manages output formatters and sinks
pub struct OutputManager {
    pipelines: Vec<Pipeline>,
}

struct Pipeline {
    formatter: Arc<dyn OutputFormatter>,
    sink: Arc<dyn OutputSink>,
    enabled: bool,
}

impl OutputManager {
    pub fn new() -> Self {
        Self {
            pipelines: Vec::new(),
        }
    }

    /// Add a formatter + sink pipeline
    pub fn add_pipeline(
        &mut self,
        formatter: Arc<dyn OutputFormatter>,
        sink: Arc<dyn OutputSink>,
        enabled: bool,
    ) {
        self.pipelines.push(Pipeline {
            formatter,
            sink,
            enabled,
        });
    }

    /// Write event to all enabled pipelines
    pub async fn write_event(&self, event: &TransactionEvent) -> Result<()> {
        log::debug!(
            "Writing event {} to {} pipelines",
            event.event_id,
            self.pipelines.len()
        );

        for (idx, pipeline) in self.pipelines.iter().enumerate() {
            log::debug!("Pipeline {}: enabled={}", idx, pipeline.enabled);

            if !pipeline.enabled {
                continue;
            }

            // Format the event
            let data = match pipeline.formatter.format_event(event) {
                Ok(d) => {
                    log::debug!(
                        "Formatter {} produced {} bytes",
                        pipeline.formatter.name(),
                        d.len()
                    );
                    d
                }
                Err(e) => {
                    log::error!("Formatter {} failed: {}", pipeline.formatter.name(), e);
                    continue;
                }
            };

            // Skip empty output (e.g., filtered events)
            if data.is_empty() {
                log::debug!(
                    "Formatter {} returned empty data, skipping",
                    pipeline.formatter.name()
                );
                continue;
            }

            // Write to sink
            let metadata = SinkMetadata {
                content_type: pipeline.formatter.content_type().to_string(),
                formatter_name: pipeline.formatter.name().to_string(),
            };

            log::debug!("Writing to sink {}", pipeline.sink.name());

            if let Err(e) = pipeline.sink.write(&data, &metadata).await {
                log::error!(
                    "Sink {} failed for formatter {}: {}",
                    pipeline.sink.name(),
                    pipeline.formatter.name(),
                    e
                );
            }
        }

        Ok(())
    }

    /// Number of active pipelines
    pub fn pipeline_count(&self) -> usize {
        self.pipelines.iter().filter(|p| p.enabled).count()
    }
}

/// Load output configuration from environment
pub fn load_from_env() -> anyhow::Result<OutputManager> {
    use super::formatters::*;
    use super::sinks::*;
    use std::path::PathBuf;

    let mut manager = OutputManager::new();

    // Get enabled formats
    let formats = std::env::var("OUTPUT_FORMATS").unwrap_or_default();
    if formats.is_empty() {
        log::info!("No output formatters configured");
        return Ok(manager);
    }

    for format in formats.split(',') {
        let format = format.trim();
        if format.is_empty() {
            continue;
        }

        let env_var_enabled = format.to_uppercase().replace("-", "_") + "_ENABLED";
        let enabled = std::env::var(&env_var_enabled)
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);

        if !enabled {
            log::info!("Formatter {} is disabled", format);
            continue;
        }

        // Create formatter
        let formatter: Arc<dyn OutputFormatter> = match format {
            "json-ls" => Arc::new(JsonLsFormatter),
            "iso20022" => Arc::new(Iso20022Formatter),
            "xbrl-json" => Arc::new(XbrlJsonFormatter),
            "1099-da" => Arc::new(Form1099DaFormatter),
            _ => {
                log::warn!("Unknown formatter: {}", format);
                continue;
            }
        };

        // Create sink
        let env_var_sink = format.to_uppercase().replace("-", "_") + "_SINK";
        let sink_type = std::env::var(&env_var_sink).unwrap_or_else(|_| "file".to_string());

        let sink: Arc<dyn OutputSink> = match sink_type.as_str() {
            "file" => {
                let env_var_name = format.to_uppercase().replace("-", "_") + "_PATH";
                let path =
                    std::env::var(&env_var_name).unwrap_or_else(|_| format!("./output/{}", format));
                log::debug!(
                    "File sink for {}: path={} (from {})",
                    format,
                    path,
                    env_var_name
                );
                Arc::new(FileSink::new(PathBuf::from(path)))
            }
            "http" => {
                let env_var_url = format.to_uppercase().replace("-", "_") + "_URL";
                let url = std::env::var(&env_var_url)?;

                // Parse custom headers
                let mut headers = HashMap::new();
                let env_var_token = format.to_uppercase().replace("-", "_") + "_TOKEN";
                if let Ok(token) = std::env::var(&env_var_token) {
                    headers.insert("Authorization".to_string(), format!("Bearer {}", token));
                }

                Arc::new(HttpSink::new(url, headers))
            }
            _ => {
                log::warn!("Unknown sink type for {}: {}", format, sink_type);
                continue;
            }
        };

        manager.add_pipeline(formatter, sink, enabled);
        log::info!("Configured {} formatter with {} sink", format, sink_type);
    }

    log::info!(
        "Output manager initialized with {} pipelines",
        manager.pipeline_count()
    );
    Ok(manager)
}
