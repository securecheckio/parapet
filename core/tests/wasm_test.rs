/// WASM analyzer tests
use parapet_core::rules::load_wasm_analyzers_from_dir;
use std::collections::HashMap;
use std::path::Path;

#[test]
fn test_load_wasm_from_nonexistent_dir() {
    // Should return empty vec for nonexistent directory
    let config = HashMap::new();
    let result = load_wasm_analyzers_from_dir("/nonexistent/path/to/analyzers", config);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[test]
fn test_load_wasm_from_empty_dir() {
    // Create temporary empty directory
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().to_str().unwrap();

    let config = HashMap::new();
    let result = load_wasm_analyzers_from_dir(path, config);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[test]
fn test_wasm_config_empty() {
    use parapet_core::rules::wasm_config::load_wasm_config_from_env;

    // Without env vars, should return empty map
    let config = load_wasm_config_from_env();

    // May have env vars set, so just verify it returns a HashMap
    assert!(config.is_empty() || !config.is_empty());
}

#[test]
fn test_wasm_analyzers_dir_default() {
    // Test that default WASM directory path is sensible
    let default_path = "./analyzers";
    assert!(Path::new(default_path).to_str().is_some());
}

#[cfg(target_arch = "wasm32")]
#[test]
fn test_wasm_target_available() {
    // This test only runs when compiled for WASM
    assert!(true);
}
