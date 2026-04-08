#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::output::event::{RiskLevel, TransactionEvent, TransactionOutcome};
    use crate::output::formatters::*;
    use crate::output::sinks::*;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn create_test_event() -> TransactionEvent {
        let mut event = TransactionEvent::new(
            "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU".to_string(),
            "sendTransaction".to_string(),
        );

        event.user_id = Some("employee_42".to_string());
        event.identity = Some("John Doe".to_string());
        event.risk_score = 85;
        event.risk_level = RiskLevel::High;
        event.issues = vec!["Unlimited delegation detected".to_string()];
        event.outcome = TransactionOutcome::Blocked;
        event.block_reason = Some("High risk score".to_string());
        event.expected_action = Some("Swap 10 SOL to USDC".to_string());
        event.destination = Some("Binance Hot Wallet".to_string());
        event.programs = vec!["JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB".to_string()];
        event.program_names = vec!["Jupiter Aggregator v4".to_string()];
        event.action_type = Some("swap".to_string());
        event.protocol = Some("Jupiter".to_string());
        event.amount = Some("10 SOL".to_string());
        event.tokens = vec!["SOL".to_string(), "USDC".to_string()];
        event.summary = "John Doe attempted to swap 10 SOL to USDC via Jupiter".to_string();
        event.signature = Some("5j7s3...".to_string());
        event.slot = Some(12345678);
        event.compute_units = Some(50000);

        event
    }

    #[test]
    fn test_json_ls_formatter() {
        let formatter = JsonLsFormatter;
        let event = create_test_event();

        let result = formatter.format_event(&event).unwrap();
        let output = String::from_utf8(result).unwrap();

        // Should be valid JSON Lines (ends with newline)
        assert!(output.ends_with('\n'));

        // Parse as JSON
        let json: serde_json::Value = serde_json::from_str(output.trim()).unwrap();

        // Check key fields
        assert_eq!(json["event_type"], "transaction_blocked");
        assert_eq!(json["risk_score"], 85);
        assert_eq!(json["risk_level"], "HIGH");
        assert_eq!(
            json["wallet"],
            "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU"
        );
        assert_eq!(json["user_id"], "employee_42");
        assert_eq!(json["sourcetype"], "securecheck:transaction");

        assert_eq!(formatter.content_type(), "application/x-ndjson");
        assert_eq!(formatter.name(), "json-ls");
    }

    #[test]
    fn test_iso20022_formatter() {
        let formatter = Iso20022Formatter;
        let mut event = create_test_event();
        event.outcome = TransactionOutcome::Allowed; // Only formats allowed

        let result = formatter.format_event(&event).unwrap();
        let output = String::from_utf8(result).unwrap();

        // Should be valid XML
        assert!(output.starts_with("<?xml version=\"1.0\""));
        assert!(output.contains("<Document xmlns=\"urn:iso:std:iso:20022"));
        assert!(output.contains("<MsgId>"));
        assert!(output.contains("<RiskScore>85</RiskScore>"));
        assert!(output.contains("<RiskLevel>HIGH</RiskLevel>"));

        assert_eq!(formatter.content_type(), "application/xml");
        assert_eq!(formatter.name(), "iso20022");
    }

    #[test]
    fn test_iso20022_filters_blocked() {
        let formatter = Iso20022Formatter;
        let event = create_test_event(); // Blocked by default

        let result = formatter.format_event(&event).unwrap();

        // Should return empty for blocked transactions
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_xbrl_json_formatter() {
        let formatter = XbrlJsonFormatter;
        let mut event = create_test_event();
        event.outcome = TransactionOutcome::Allowed;

        let result = formatter.format_event(&event).unwrap();
        let output = String::from_utf8(result).unwrap();

        // Parse as JSON
        let json: serde_json::Value = serde_json::from_str(&output).unwrap();

        // Check xBRL structure
        assert!(json["documentInfo"].is_object());
        assert!(json["facts"].is_object());
        assert_eq!(
            json["facts"]["crypto:DigitalAssetTransferAmount"]["dimensions"]["crypto:DigitalAsset"],
            "SOL"
        );
        assert_eq!(json["facts"]["securecheck:RiskScore"]["value"], 85);
        assert_eq!(json["facts"]["securecheck:RiskLevel"]["value"], "HIGH");

        assert_eq!(formatter.content_type(), "application/json");
        assert_eq!(formatter.name(), "xbrl-json");
    }

    #[test]
    fn test_form1099da_formatter() {
        let formatter = Form1099DaFormatter;
        let mut event = create_test_event();
        event.outcome = TransactionOutcome::Allowed;
        event.action_type = Some("swap".to_string());

        let result = formatter.format_event(&event).unwrap();
        let output = String::from_utf8(result).unwrap();

        // Parse as JSON
        let json: serde_json::Value = serde_json::from_str(&output).unwrap();

        // Check IRS form structure
        assert_eq!(json["formType"], "1099-DA");
        assert_eq!(
            json["payee"]["walletAddress"],
            "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU"
        );
        assert_eq!(json["transaction"]["type"], "SALE");
        assert_eq!(json["transaction"]["riskScore"], 85);
        assert_eq!(json["complianceChecks"]["amlScreening"], false); // High risk

        assert_eq!(formatter.content_type(), "application/json");
        assert_eq!(formatter.name(), "1099-da");
    }

    #[test]
    fn test_form1099da_filters_non_transfers() {
        let formatter = Form1099DaFormatter;
        let mut event = create_test_event();
        event.outcome = TransactionOutcome::Allowed;
        event.action_type = Some("stake".to_string()); // Not a transfer/swap

        let result = formatter.format_event(&event).unwrap();

        // Should return empty for non-transfer actions
        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_file_sink() {
        let temp_dir = TempDir::new().unwrap();
        let sink = FileSink::new(temp_dir.path().to_path_buf());

        let data = b"test event data\n";
        let metadata = sink::SinkMetadata {
            content_type: "application/json".to_string(),
            formatter_name: "test".to_string(),
        };

        sink.write(data, &metadata).await.unwrap();

        // Check file was created
        let now = chrono::Utc::now();
        let filename = format!("test-{}.log", now.format("%Y%m%d"));
        let file_path = temp_dir.path().join(filename);

        assert!(file_path.exists());

        let contents = std::fs::read_to_string(file_path).unwrap();
        assert_eq!(contents, "test event data\n");

        assert_eq!(sink.name(), "file");
    }

    #[tokio::test]
    async fn test_http_sink() {
        use std::collections::HashMap;

        // Note: This test would need a mock HTTP server
        // For now, just test creation
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer test".to_string());

        let sink = HttpSink::new("http://localhost:8080".to_string(), headers);
        assert_eq!(sink.name(), "http");
    }

    #[tokio::test]
    async fn test_output_manager() {
        use std::sync::Arc;

        let temp_dir = TempDir::new().unwrap();

        let mut manager = manager::OutputManager::new();

        // Add JSON-LS formatter with file sink
        let formatter = Arc::new(JsonLsFormatter);
        let sink = Arc::new(FileSink::new(temp_dir.path().to_path_buf()));
        manager.add_pipeline(formatter, sink, true);

        assert_eq!(manager.pipeline_count(), 1);

        // Write event
        let event = create_test_event();
        manager.write_event(&event).await.unwrap();

        // Check file was created
        let now = chrono::Utc::now();
        let filename = format!("json-ls-{}.log", now.format("%Y%m%d"));
        let file_path = temp_dir.path().join(filename);

        assert!(file_path.exists());
    }

    #[tokio::test]
    async fn test_output_manager_disabled_pipeline() {
        let mut manager = manager::OutputManager::new();

        let formatter = Arc::new(JsonLsFormatter);
        let sink = Arc::new(FileSink::new(PathBuf::from("/tmp/test")));
        manager.add_pipeline(formatter, sink, false); // Disabled

        assert_eq!(manager.pipeline_count(), 0); // Counts only enabled

        let event = create_test_event();
        manager.write_event(&event).await.unwrap(); // Should not error
    }

    #[test]
    fn test_transaction_event_new() {
        let event = TransactionEvent::new("test_wallet".to_string(), "sendTransaction".to_string());

        assert_eq!(event.wallet, "test_wallet");
        assert_eq!(event.method, "sendTransaction");
        assert_eq!(event.risk_score, 0);
        assert!(matches!(event.risk_level, RiskLevel::Low));
        assert!(matches!(event.outcome, TransactionOutcome::Allowed));
        assert!(!event.engine_version.is_empty());
    }
}
