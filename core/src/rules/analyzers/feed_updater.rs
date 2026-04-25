/// Feed Updater for Known-Safe Programs and Owners
///
/// Allows open-source users to pull curated lists from the SaaS platform
use anyhow::{Context, Result};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use super::core::inner_instruction::{KnownSafeOwnersConfig, KnownSafeProgramsConfig};

/// Feed metadata and update info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedMetadata {
    pub version: String,
    pub last_updated: String,
    pub feed_url: Option<String>,
}

/// Feed updater for pulling remote safe programs/owners lists
pub struct SafeListFeedUpdater {
    client: reqwest::blocking::Client,
}

impl SafeListFeedUpdater {
    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| reqwest::blocking::Client::new()),
        }
    }

    /// Update safe programs list from remote feed
    pub fn update_programs<P: AsRef<Path>>(
        &self,
        local_path: P,
        feed_url: Option<&str>,
    ) -> Result<bool> {
        let local_path = local_path.as_ref();

        // Determine feed URL (explicit arg > config file > default)
        let url = if let Some(url) = feed_url {
            url.to_string()
        } else {
            // Try to read feed_url from existing local file
            if local_path.exists() {
                if let Ok(content) = fs::read_to_string(local_path) {
                    if let Ok(config) = serde_json::from_str::<KnownSafeProgramsConfig>(&content) {
                        config
                            .feed_url
                            .unwrap_or_else(|| self.default_programs_feed_url())
                    } else {
                        self.default_programs_feed_url()
                    }
                } else {
                    self.default_programs_feed_url()
                }
            } else {
                self.default_programs_feed_url()
            }
        };

        info!("📡 Fetching safe programs feed from: {}", url);

        // Fetch remote feed
        let response = self
            .client
            .get(&url)
            .header("User-Agent", "Parapet/1.0")
            .send()
            .with_context(|| format!("Failed to fetch safe programs feed from: {}", url))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Feed server returned error: {} {}",
                response.status().as_u16(),
                response.status().canonical_reason().unwrap_or("Unknown")
            ));
        }

        // Parse remote config
        let remote_config: KnownSafeProgramsConfig = response
            .json()
            .context("Failed to parse remote safe programs feed")?;

        // Check if update is needed
        let needs_update = if local_path.exists() {
            if let Ok(content) = fs::read_to_string(local_path) {
                if let Ok(local_config) = serde_json::from_str::<KnownSafeProgramsConfig>(&content)
                {
                    // Compare versions or last_updated
                    local_config.version != remote_config.version
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            true
        };

        if !needs_update {
            info!(
                "✅ Safe programs list is already up to date (version: {})",
                remote_config.version
            );
            return Ok(false);
        }

        // Create parent directory if needed
        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        // Write updated config
        let json = serde_json::to_string_pretty(&remote_config)
            .context("Failed to serialize remote config")?;

        fs::write(local_path, json).with_context(|| {
            format!(
                "Failed to write updated config to: {}",
                local_path.display()
            )
        })?;

        info!(
            "✅ Updated safe programs list: {} programs (version: {})",
            remote_config.programs.len(),
            remote_config.version
        );

        Ok(true)
    }

    /// Update safe owners list from remote feed
    pub fn update_owners<P: AsRef<Path>>(
        &self,
        local_path: P,
        feed_url: Option<&str>,
    ) -> Result<bool> {
        let local_path = local_path.as_ref();

        // Determine feed URL
        let url = if let Some(url) = feed_url {
            url.to_string()
        } else {
            if local_path.exists() {
                if let Ok(content) = fs::read_to_string(local_path) {
                    if let Ok(config) = serde_json::from_str::<KnownSafeOwnersConfig>(&content) {
                        config
                            .feed_url
                            .unwrap_or_else(|| self.default_owners_feed_url())
                    } else {
                        self.default_owners_feed_url()
                    }
                } else {
                    self.default_owners_feed_url()
                }
            } else {
                self.default_owners_feed_url()
            }
        };

        info!("📡 Fetching safe owners feed from: {}", url);

        // Fetch remote feed
        let response = self
            .client
            .get(&url)
            .header("User-Agent", "Parapet/1.0")
            .send()
            .with_context(|| format!("Failed to fetch safe owners feed from: {}", url))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Feed server returned error: {} {}",
                response.status().as_u16(),
                response.status().canonical_reason().unwrap_or("Unknown")
            ));
        }

        // Parse remote config
        let remote_config: KnownSafeOwnersConfig = response
            .json()
            .context("Failed to parse remote safe owners feed")?;

        // Check if update is needed
        let needs_update = if local_path.exists() {
            if let Ok(content) = fs::read_to_string(local_path) {
                if let Ok(local_config) = serde_json::from_str::<KnownSafeOwnersConfig>(&content) {
                    local_config.version != remote_config.version
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            true
        };

        if !needs_update {
            info!(
                "✅ Safe owners list is already up to date (version: {})",
                remote_config.version
            );
            return Ok(false);
        }

        // Create parent directory if needed
        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        // Write updated config
        let json = serde_json::to_string_pretty(&remote_config)
            .context("Failed to serialize remote config")?;

        fs::write(local_path, json).with_context(|| {
            format!(
                "Failed to write updated config to: {}",
                local_path.display()
            )
        })?;

        info!(
            "✅ Updated safe owners list: {} owners (version: {})",
            remote_config.owners.len(),
            remote_config.version
        );

        Ok(true)
    }

    /// Update both programs and owners from remote feeds
    pub fn update_all<P: AsRef<Path>>(&self, config_dir: P) -> Result<(bool, bool)> {
        let config_dir = config_dir.as_ref();

        let programs_path = config_dir.join("known-safe-programs.json");
        let owners_path = config_dir.join("known-safe-owners.json");

        let programs_updated = self
            .update_programs(&programs_path, None)
            .unwrap_or_else(|e| {
                warn!("Failed to update safe programs: {}", e);
                false
            });

        let owners_updated = self.update_owners(&owners_path, None).unwrap_or_else(|e| {
            warn!("Failed to update safe owners: {}", e);
            false
        });

        Ok((programs_updated, owners_updated))
    }

    /// Get default feed URL for programs
    fn default_programs_feed_url(&self) -> String {
        std::env::var("SAFE_PROGRAMS_FEED_URL")
            .unwrap_or_else(|_| "https://api.securecheck.sh/v1/safe-programs/feed.json".to_string())
    }

    /// Get default feed URL for owners
    fn default_owners_feed_url(&self) -> String {
        std::env::var("SAFE_OWNERS_FEED_URL")
            .unwrap_or_else(|_| "https://api.securecheck.sh/v1/safe-owners/feed.json".to_string())
    }
}

impl Default for SafeListFeedUpdater {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_feed_urls() {
        let updater = SafeListFeedUpdater::new();
        assert!(updater.default_programs_feed_url().starts_with("https://"));
        assert!(updater.default_owners_feed_url().starts_with("https://"));
    }
}
