use anyhow::Result;
use serial_test::serial;
use tempfile::TempDir;

#[tokio::test]
#[serial]
async fn test_output_event_emission() -> Result<()> {
    // Initialize logger
    let _ = env_logger::builder().is_test(true).try_init();

    // Create temp directory for output
    let temp_dir = TempDir::new()?;
    let output_path = temp_dir.path().to_path_buf();

    // Set environment for JSON-LS formatter with file sink
    let path_str = output_path.to_str().unwrap();
    std::env::set_var("OUTPUT_FORMATS", "json-ls");
    std::env::set_var("JSON_LS_ENABLED", "true");
    std::env::set_var("JSON_LS_SINK", "file");
    std::env::set_var("JSON_LS_PATH", path_str);

    println!("Test output path: {}", path_str);

    // Load output manager
    let output_manager = parapet_proxy::output::load_from_env()?;

    println!("Pipeline count: {}", output_manager.pipeline_count());
    assert_eq!(
        output_manager.pipeline_count(),
        1,
        "Should have 1 pipeline configured"
    );

    // Create a test event
    let event = parapet_proxy::output::EventBuilder::new(
        "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU".to_string(),
        "sendTransaction".to_string(),
    )
    .allowed()
    .build();

    // Emit event
    println!("Event ID: {}", event.event_id);
    println!("Event wallet: {}", event.wallet);
    println!("Event method: {}", event.method);

    let result = output_manager.write_event(&event).await;
    println!("Write result: {:?}", result);
    result?;

    // Give a moment for async write to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Check file was created
    let now = chrono::Utc::now();
    let filename = format!("json-ls-{}.log", now.format("%Y%m%d"));
    let file_path = output_path.join(&filename);

    // Debug: list files in output directory
    let entries: Vec<_> = std::fs::read_dir(&output_path)?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    assert!(
        file_path.exists(),
        "Event file should be created. Expected: {}, Found: {:?}",
        filename,
        entries
    );

    // Read and verify content
    let contents = std::fs::read_to_string(file_path)?;
    assert!(!contents.is_empty(), "Event file should not be empty");

    // Parse as JSON
    let json: serde_json::Value = serde_json::from_str(contents.trim())?;
    assert_eq!(json["event_type"], "transaction_allowed");
    assert_eq!(
        json["wallet"],
        "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU"
    );
    assert_eq!(json["method"], "sendTransaction");
    assert_eq!(json["sourcetype"], "securecheck:transaction");

    // Cleanup
    std::env::remove_var("OUTPUT_FORMATS");
    std::env::remove_var("JSON_LS_SINK");
    std::env::remove_var("JSON_LS_PATH");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_blocked_transaction_event() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_path = temp_dir.path().to_path_buf();

    std::env::remove_var("OUTPUT_FORMATS");
    std::env::remove_var("JSON_LS_SINK");
    std::env::remove_var("JSON_LS_PATH");

    std::env::set_var("OUTPUT_FORMATS", "json-ls");
    std::env::set_var("JSON_LS_SINK", "file");
    std::env::set_var("JSON_LS_PATH", output_path.to_str().unwrap());

    let output_manager = parapet_proxy::output::load_from_env()?;

    // Create a blocked transaction event
    let decision = parapet_core::rules::types::RuleDecision {
        matched: true,
        rule_id: "test_block_rule".to_string(),
        rule_name: "Test Block Rule".to_string(),
        action: parapet_core::rules::types::RuleAction::Block,
        message: "Unlimited delegation detected".to_string(),
    };

    let event = parapet_proxy::output::EventBuilder::new(
        "test_wallet".to_string(),
        "sendTransaction".to_string(),
    )
    .with_rule_decision(&decision)
    .build();

    output_manager.write_event(&event).await?;

    // Give a moment for async write to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify file
    let now = chrono::Utc::now();
    let filename = format!("json-ls-{}.log", now.format("%Y%m%d"));
    let file_path = output_path.join(filename);

    assert!(file_path.exists(), "Blocked event file should be created");

    let contents = std::fs::read_to_string(file_path)?;
    let json: serde_json::Value = serde_json::from_str(contents.trim())?;

    assert_eq!(json["event_type"], "transaction_blocked");
    assert_eq!(json["risk_level"], "HIGH");
    assert_eq!(json["block_reason"], "Unlimited delegation detected");
    assert!(json["issues"].as_array().unwrap().len() > 0);

    // Cleanup
    std::env::remove_var("OUTPUT_FORMATS");
    std::env::remove_var("JSON_LS_SINK");
    std::env::remove_var("JSON_LS_PATH");

    Ok(())
}
