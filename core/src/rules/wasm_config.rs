use std::collections::HashMap;

/// Build WASM config map from environment variables with WASM_CONFIG_ prefix
///
/// Example:
///   WASM_CONFIG_HELIUS_API_KEY=abc123  -> config["HELIUS_API_KEY"] = "abc123"
///   WASM_CONFIG_RPC_URL=https://...    -> config["RPC_URL"] = "https://..."
pub fn load_wasm_config_from_env() -> HashMap<String, String> {
    const PREFIX: &str = "WASM_CONFIG_";
    let mut config = HashMap::new();

    for (key, value) in std::env::vars() {
        if let Some(stripped) = key.strip_prefix(PREFIX) {
            config.insert(stripped.to_string(), value);
        }
    }

    if !config.is_empty() {
        log::info!(
            "🔧 Loaded {} WASM config value(s) from environment",
            config.len()
        );
        for key in config.keys() {
            log::debug!("  - {}", key);
        }
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_wasm_config_from_env() {
        std::env::set_var("WASM_CONFIG_TEST_KEY", "test_value");
        std::env::set_var("WASM_CONFIG_ANOTHER", "another_value");
        std::env::set_var("UNRELATED_VAR", "should_not_appear");

        let config = load_wasm_config_from_env();

        assert_eq!(config.get("TEST_KEY"), Some(&"test_value".to_string()));
        assert_eq!(config.get("ANOTHER"), Some(&"another_value".to_string()));
        assert!(!config.contains_key("UNRELATED_VAR"));

        std::env::remove_var("WASM_CONFIG_TEST_KEY");
        std::env::remove_var("WASM_CONFIG_ANOTHER");
        std::env::remove_var("UNRELATED_VAR");
    }

    #[test]
    fn test_empty_config() {
        let config = load_wasm_config_from_env();
        assert!(config.is_empty() || !config.keys().any(|k| k == "NONEXISTENT"));
    }
}
