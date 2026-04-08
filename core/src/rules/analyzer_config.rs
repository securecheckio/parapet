use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Configuration for a single analyzer
#[derive(Debug, Clone, Deserialize)]
pub struct AnalyzerConfig {
    pub enabled: bool,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub requires_api_key: Option<String>,
    #[serde(default)]
    pub requires_feature: Option<String>,
}

/// Top-level analyzer configuration
#[derive(Debug, Clone, Deserialize)]
pub struct AnalyzersConfig {
    pub analyzers: HashMap<String, AnalyzerConfig>,
}

impl AnalyzersConfig {
    /// Load analyzer configuration from TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())?;
        let config: AnalyzersConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Check if an analyzer is enabled
    pub fn is_enabled(&self, analyzer_name: &str) -> bool {
        self.analyzers
            .get(analyzer_name)
            .map(|config| config.enabled)
            .unwrap_or(true) // Default to enabled if not in config
    }

    /// Check if an analyzer's requirements are met
    pub fn requirements_met(&self, analyzer_name: &str) -> bool {
        let Some(config) = self.analyzers.get(analyzer_name) else {
            return true; // No config = no requirements
        };

        // Check API key requirement
        if let Some(api_key_var) = &config.requires_api_key {
            if std::env::var(api_key_var).is_err() {
                log::warn!(
                    "Analyzer '{}' requires {} but it's not set",
                    analyzer_name,
                    api_key_var
                );
                return false;
            }
        }

        // Check feature flag requirement
        if let Some(feature) = &config.requires_feature {
            // Feature flags are compile-time, so we can't check them at runtime
            // This is just for documentation in the config file
            log::debug!(
                "Analyzer '{}' requires feature '{}' (compile-time check)",
                analyzer_name,
                feature
            );
        }

        true
    }

    /// Check if analyzer should be registered (enabled + requirements met)
    pub fn should_register(&self, analyzer_name: &str) -> bool {
        if !self.is_enabled(analyzer_name) {
            log::info!(
                "⏭️  Skipping analyzer '{}' (disabled in config)",
                analyzer_name
            );
            return false;
        }

        if !self.requirements_met(analyzer_name) {
            log::info!(
                "⏭️  Skipping analyzer '{}' (requirements not met)",
                analyzer_name
            );
            return false;
        }

        true
    }

    /// Get list of enabled analyzers
    pub fn enabled_analyzers(&self) -> Vec<String> {
        self.analyzers
            .iter()
            .filter(|(_, config)| config.enabled)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Get list of disabled analyzers
    pub fn disabled_analyzers(&self) -> Vec<String> {
        self.analyzers
            .iter()
            .filter(|(_, config)| !config.enabled)
            .map(|(name, _)| name.clone())
            .collect()
    }
}

impl Default for AnalyzersConfig {
    fn default() -> Self {
        Self {
            analyzers: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AnalyzersConfig::default();
        // Unknown analyzers default to enabled
        assert!(config.is_enabled("unknown_analyzer"));
    }

    #[test]
    fn test_should_register() {
        let mut config = AnalyzersConfig::default();

        // Add a disabled analyzer
        config.analyzers.insert(
            "disabled_analyzer".to_string(),
            AnalyzerConfig {
                enabled: false,
                description: "Test".to_string(),
                requires_api_key: None,
                requires_feature: None,
            },
        );

        // Add an enabled analyzer
        config.analyzers.insert(
            "enabled_analyzer".to_string(),
            AnalyzerConfig {
                enabled: true,
                description: "Test".to_string(),
                requires_api_key: None,
                requires_feature: None,
            },
        );

        assert!(!config.should_register("disabled_analyzer"));
        assert!(config.should_register("enabled_analyzer"));
    }

    #[test]
    fn test_is_enabled_with_config() {
        let mut config = AnalyzersConfig::default();

        config.analyzers.insert(
            "test_analyzer".to_string(),
            AnalyzerConfig {
                enabled: true,
                description: "Test".to_string(),
                requires_api_key: None,
                requires_feature: None,
            },
        );

        assert!(config.is_enabled("test_analyzer"));
    }

    #[test]
    fn test_is_enabled_disabled() {
        let mut config = AnalyzersConfig::default();

        config.analyzers.insert(
            "test_analyzer".to_string(),
            AnalyzerConfig {
                enabled: false,
                description: "Test".to_string(),
                requires_api_key: None,
                requires_feature: None,
            },
        );

        assert!(!config.is_enabled("test_analyzer"));
    }

    #[test]
    fn test_requirements_met_no_requirements() {
        let mut config = AnalyzersConfig::default();

        config.analyzers.insert(
            "test_analyzer".to_string(),
            AnalyzerConfig {
                enabled: true,
                description: "Test".to_string(),
                requires_api_key: None,
                requires_feature: None,
            },
        );

        assert!(config.requirements_met("test_analyzer"));
    }

    #[test]
    fn test_requirements_met_unknown_analyzer() {
        let config = AnalyzersConfig::default();
        assert!(config.requirements_met("unknown"));
    }

    #[test]
    fn test_requirements_met_with_api_key() {
        let mut config = AnalyzersConfig::default();

        // Set a test API key
        std::env::set_var("TEST_API_KEY", "test_value");

        config.analyzers.insert(
            "test_analyzer".to_string(),
            AnalyzerConfig {
                enabled: true,
                description: "Test".to_string(),
                requires_api_key: Some("TEST_API_KEY".to_string()),
                requires_feature: None,
            },
        );

        assert!(config.requirements_met("test_analyzer"));

        // Clean up
        std::env::remove_var("TEST_API_KEY");
    }

    #[test]
    fn test_requirements_not_met_missing_api_key() {
        let mut config = AnalyzersConfig::default();

        // Make sure the key doesn't exist
        std::env::remove_var("MISSING_API_KEY");

        config.analyzers.insert(
            "test_analyzer".to_string(),
            AnalyzerConfig {
                enabled: true,
                description: "Test".to_string(),
                requires_api_key: Some("MISSING_API_KEY".to_string()),
                requires_feature: None,
            },
        );

        assert!(!config.requirements_met("test_analyzer"));
    }

    #[test]
    fn test_requirements_met_with_feature() {
        let mut config = AnalyzersConfig::default();

        config.analyzers.insert(
            "test_analyzer".to_string(),
            AnalyzerConfig {
                enabled: true,
                description: "Test".to_string(),
                requires_api_key: None,
                requires_feature: Some("test_feature".to_string()),
            },
        );

        // Feature flags are compile-time, so this always returns true
        assert!(config.requirements_met("test_analyzer"));
    }

    #[test]
    fn test_should_register_unknown_analyzer() {
        let config = AnalyzersConfig::default();
        // Unknown analyzers default to enabled
        assert!(config.should_register("unknown"));
    }

    #[test]
    fn test_should_register_missing_api_key() {
        let mut config = AnalyzersConfig::default();

        std::env::remove_var("MISSING_KEY");

        config.analyzers.insert(
            "test_analyzer".to_string(),
            AnalyzerConfig {
                enabled: true,
                description: "Test".to_string(),
                requires_api_key: Some("MISSING_KEY".to_string()),
                requires_feature: None,
            },
        );

        assert!(!config.should_register("test_analyzer"));
    }

    #[test]
    fn test_enabled_analyzers() {
        let mut config = AnalyzersConfig::default();

        config.analyzers.insert(
            "enabled1".to_string(),
            AnalyzerConfig {
                enabled: true,
                description: "Test".to_string(),
                requires_api_key: None,
                requires_feature: None,
            },
        );

        config.analyzers.insert(
            "enabled2".to_string(),
            AnalyzerConfig {
                enabled: true,
                description: "Test".to_string(),
                requires_api_key: None,
                requires_feature: None,
            },
        );

        config.analyzers.insert(
            "disabled".to_string(),
            AnalyzerConfig {
                enabled: false,
                description: "Test".to_string(),
                requires_api_key: None,
                requires_feature: None,
            },
        );

        let enabled = config.enabled_analyzers();
        assert_eq!(enabled.len(), 2);
        assert!(enabled.contains(&"enabled1".to_string()));
        assert!(enabled.contains(&"enabled2".to_string()));
        assert!(!enabled.contains(&"disabled".to_string()));
    }

    #[test]
    fn test_disabled_analyzers() {
        let mut config = AnalyzersConfig::default();

        config.analyzers.insert(
            "enabled".to_string(),
            AnalyzerConfig {
                enabled: true,
                description: "Test".to_string(),
                requires_api_key: None,
                requires_feature: None,
            },
        );

        config.analyzers.insert(
            "disabled1".to_string(),
            AnalyzerConfig {
                enabled: false,
                description: "Test".to_string(),
                requires_api_key: None,
                requires_feature: None,
            },
        );

        config.analyzers.insert(
            "disabled2".to_string(),
            AnalyzerConfig {
                enabled: false,
                description: "Test".to_string(),
                requires_api_key: None,
                requires_feature: None,
            },
        );

        let disabled = config.disabled_analyzers();
        assert_eq!(disabled.len(), 2);
        assert!(disabled.contains(&"disabled1".to_string()));
        assert!(disabled.contains(&"disabled2".to_string()));
        assert!(!disabled.contains(&"enabled".to_string()));
    }

    #[test]
    fn test_enabled_analyzers_empty() {
        let config = AnalyzersConfig::default();
        assert!(config.enabled_analyzers().is_empty());
    }

    #[test]
    fn test_disabled_analyzers_empty() {
        let config = AnalyzersConfig::default();
        assert!(config.disabled_analyzers().is_empty());
    }

    #[test]
    fn test_from_file_invalid_path() {
        let result = AnalyzersConfig::from_file("/nonexistent/path/config.toml");
        assert!(result.is_err());
    }

    #[test]
    fn test_analyzer_config_with_description() {
        let mut config = AnalyzersConfig::default();

        config.analyzers.insert(
            "test".to_string(),
            AnalyzerConfig {
                enabled: true,
                description: "This is a test analyzer".to_string(),
                requires_api_key: None,
                requires_feature: None,
            },
        );

        let analyzer_config = config.analyzers.get("test").unwrap();
        assert_eq!(analyzer_config.description, "This is a test analyzer");
    }

    #[test]
    fn test_analyzer_config_clone() {
        let config = AnalyzerConfig {
            enabled: true,
            description: "Test".to_string(),
            requires_api_key: Some("KEY".to_string()),
            requires_feature: Some("feature".to_string()),
        };

        let cloned = config.clone();
        assert_eq!(cloned.enabled, config.enabled);
        assert_eq!(cloned.description, config.description);
        assert_eq!(cloned.requires_api_key, config.requires_api_key);
        assert_eq!(cloned.requires_feature, config.requires_feature);
    }

    #[test]
    fn test_analyzers_config_clone() {
        let mut config = AnalyzersConfig::default();

        config.analyzers.insert(
            "test".to_string(),
            AnalyzerConfig {
                enabled: true,
                description: "Test".to_string(),
                requires_api_key: None,
                requires_feature: None,
            },
        );

        let cloned = config.clone();
        assert_eq!(cloned.analyzers.len(), config.analyzers.len());
        assert!(cloned.analyzers.contains_key("test"));
    }

    #[test]
    fn test_from_toml_string() {
        let toml_str = r#"
[analyzers.test_analyzer]
enabled = true
description = "Test analyzer"
"#;

        let config: AnalyzersConfig = toml::from_str(toml_str).unwrap();
        assert!(config.is_enabled("test_analyzer"));
        assert_eq!(
            config.analyzers.get("test_analyzer").unwrap().description,
            "Test analyzer"
        );
    }

    #[test]
    fn test_from_toml_with_api_key() {
        let toml_str = r#"
[analyzers.test_analyzer]
enabled = true
description = "Test"
requires_api_key = "MY_API_KEY"
"#;

        let config: AnalyzersConfig = toml::from_str(toml_str).unwrap();
        let analyzer_config = config.analyzers.get("test_analyzer").unwrap();
        assert_eq!(
            analyzer_config.requires_api_key,
            Some("MY_API_KEY".to_string())
        );
    }

    #[test]
    fn test_from_toml_with_feature() {
        let toml_str = r#"
[analyzers.test_analyzer]
enabled = true
description = "Test"
requires_feature = "my_feature"
"#;

        let config: AnalyzersConfig = toml::from_str(toml_str).unwrap();
        let analyzer_config = config.analyzers.get("test_analyzer").unwrap();
        assert_eq!(
            analyzer_config.requires_feature,
            Some("my_feature".to_string())
        );
    }

    #[test]
    fn test_from_toml_disabled() {
        let toml_str = r#"
[analyzers.test_analyzer]
enabled = false
description = "Disabled analyzer"
"#;

        let config: AnalyzersConfig = toml::from_str(toml_str).unwrap();
        assert!(!config.is_enabled("test_analyzer"));
    }
}
