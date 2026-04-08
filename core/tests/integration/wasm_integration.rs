#[cfg(feature = "wasm-analyzers")]
mod wasm_tests {
    use parapet_core::rules::{
        analyzer::TransactionAnalyzer,
        wasm_analyzer::{load_wasm_analyzers_from_dir, WasmAnalyzer},
    };
    use solana_sdk::{
        message::Message,
        pubkey::Pubkey,
        signature::Keypair,
        signer::Signer,
        transaction::Transaction,
    };
    use solana_system_interface::instruction as system_instruction;
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    fn get_mock_wasm_path() -> Option<String> {
        // Check if mock WASM exists (it needs to be compiled first)
        let paths = vec![
            "tests/wasm_mock/target/wasm32-unknown-unknown/release/mock_wasm_analyzer.wasm",
            "target/wasm32-unknown-unknown/release/mock_wasm_analyzer.wasm",
        ];
        
        for path in paths {
            if Path::new(path).exists() {
                return Some(path.to_string());
            }
        }
        None
    }

    #[tokio::test]
    async fn test_load_wasm_analyzer_from_file() {
        let wasm_path = match get_mock_wasm_path() {
            Some(path) => path,
            None => {
                println!("Skipping WASM test: mock analyzer not compiled");
                println!("Run: cd tests/wasm_mock && cargo build --target wasm32-unknown-unknown --release");
                return;
            }
        };

        let analyzer = WasmAnalyzer::from_file(&PathBuf::from(&wasm_path), HashMap::new());
        assert!(analyzer.is_ok(), "Failed to load WASM analyzer: {:?}", analyzer.err());
        
        let analyzer = analyzer.unwrap();
        
        // Verify metadata
        let fields = analyzer.fields();
        assert!(fields.contains(&"mock_field_1".to_string()));
        assert!(fields.contains(&"mock_field_2".to_string()));
        assert!(fields.contains(&"mock_risk_score".to_string()));
        
        assert_eq!(analyzer.estimated_latency_ms(), 5);
    }

    #[tokio::test]
    async fn test_wasm_analyzer_analyze_transaction() {
        let wasm_path = match get_mock_wasm_path() {
            Some(path) => path,
            None => {
                println!("Skipping WASM test: mock analyzer not compiled");
                return;
            }
        };

        let analyzer = WasmAnalyzer::from_file(&PathBuf::from(&wasm_path), HashMap::new()).unwrap();
        
        // Create a simple transaction
        let payer = Keypair::new();
        let transfer = system_instruction::transfer(&payer.pubkey(), &Pubkey::new_unique(), 1000);
        let message = Message::new(&[transfer], Some(&payer.pubkey()));
        let tx = Transaction::new_unsigned(message);
        
        // Analyze it
        let fields = analyzer.analyze(&tx).await;
        assert!(fields.is_ok(), "Analysis failed: {:?}", fields.err());
        
        let fields = fields.unwrap();
        
        // Verify returned fields
        assert_eq!(fields.get("mock_field_1").unwrap(), &serde_json::json!("test_value"));
        assert_eq!(fields.get("mock_field_2").unwrap(), &serde_json::json!(42));
        assert_eq!(fields.get("mock_risk_score").unwrap(), &serde_json::json!(25));
    }

    #[tokio::test]
    async fn test_load_wasm_analyzers_from_directory() {
        // Create temp directory with mock WASM
        let temp_dir = std::env::temp_dir().join("parapet_wasm_test");
        fs::create_dir_all(&temp_dir).unwrap();
        
        let wasm_path = match get_mock_wasm_path() {
            Some(path) => path,
            None => {
                println!("Skipping WASM test: mock analyzer not compiled");
                return;
            }
        };
        
        // Copy mock WASM to temp directory
        let dest_path = temp_dir.join("mock_analyzer.wasm");
        fs::copy(&wasm_path, &dest_path).unwrap();
        
        // Load from directory with no config
        let analyzers = load_wasm_analyzers_from_dir(temp_dir.to_str().unwrap(), HashMap::new());
        assert!(analyzers.is_ok(), "Failed to load from directory: {:?}", analyzers.err());
        
        let analyzers = analyzers.unwrap();
        assert_eq!(analyzers.len(), 1, "Should load exactly 1 analyzer");
        
        // Cleanup
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[tokio::test]
    async fn test_wasm_analyzer_in_rules_engine() {
        use parapet_core::rules::{AnalyzerRegistry, RuleEngine, types};
        
        let wasm_path = match get_mock_wasm_path() {
            Some(path) => path,
            None => {
                println!("Skipping WASM test: mock analyzer not compiled");
                return;
            }
        };

        let analyzer = WasmAnalyzer::from_file(&PathBuf::from(&wasm_path), HashMap::new()).unwrap();
        
        // Register in engine
        let mut registry = AnalyzerRegistry::new();
        registry.register(Arc::new(analyzer));
        
        // Create rule using WASM analyzer fields
        let rule_json = r#"
        {
            "version": "1.0",
            "id": "test-wasm-rule",
            "name": "Test WASM Rule",
            "enabled": true,
            "rule": {
                "action": "block",
                "conditions": {
                    "field": "mock_risk_score",
                    "operator": "greater_than",
                    "value": 50
                },
                "message": "Mock risk too high"
            }
        }
        "#;
        
        let mut engine = RuleEngine::new(registry);
        let rule: types::RuleDefinition = serde_json::from_str(rule_json).unwrap();
        engine.load_rules(vec![rule]).unwrap();
        
        // Test with transaction
        let payer = Keypair::new();
        let transfer = system_instruction::transfer(&payer.pubkey(), &Pubkey::new_unique(), 1000);
        let message = Message::new(&[transfer], Some(&payer.pubkey()));
        let tx = Transaction::new_unsigned(message);
        
        let decision = engine.evaluate(&tx).await.unwrap();
        
        // Mock analyzer returns risk_score of 25, so rule should not match
        assert!(!decision.matched, "Rule should not match (risk_score=25 < 50)");
    }

    #[tokio::test]
    async fn test_wasm_analyzer_config_access() {
        
        let wasm_path = match get_mock_wasm_path() {
            Some(path) => path,
            None => {
                println!("Skipping WASM test: mock analyzer not compiled");
                return;
            }
        };
        
        // Load analyzer with explicit config
        let mut config = HashMap::new();
        config.insert("HELIUS_API_KEY".to_string(), "test_key_123".to_string());
        
        let analyzer = WasmAnalyzer::from_file(&PathBuf::from(&wasm_path), config).unwrap();
        
        // Create a simple transaction
        let payer = Keypair::new();
        let transfer = system_instruction::transfer(&payer.pubkey(), &Pubkey::new_unique(), 1000);
        let message = Message::new(&[transfer], Some(&payer.pubkey()));
        let tx = Transaction::new_unsigned(message);
        
        // Analyze with config
        let fields = analyzer.analyze(&tx).await.unwrap();
        
        // Should detect the config was passed
        assert_eq!(fields.get("mock_has_config").unwrap(), &serde_json::json!(true));
    }

    #[test]
    fn test_wasm_feature_enabled() {
        // Just verify the feature is enabled
        println!("WASM analyzers feature is enabled");
    }
}

#[cfg(not(feature = "wasm-analyzers"))]
#[test]
fn test_wasm_feature_disabled() {
    println!("WASM analyzers feature is disabled");
}
