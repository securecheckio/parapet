use super::analyzer::{
    AnalyzerRegistry, ConfirmedInnerInstruction, ConfirmedTransactionMetadata, TransactionAnalyzer,
};
use anyhow::Result;
use serde_json::{json, Value};
use solana_sdk::{
    hash::Hash,
    message::{Message, MessageHeader},
    pubkey::Pubkey,
    signature::Signature,
    transaction::Transaction,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Mock analyzer for testing
struct MockAnalyzer {
    name: String,
    fields: Vec<String>,
    available: bool,
    delay_ms: Option<u64>,
}

impl MockAnalyzer {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            fields: vec!["field1".to_string(), "field2".to_string()],
            available: true,
            delay_ms: None,
        }
    }

    fn with_fields(mut self, fields: Vec<String>) -> Self {
        self.fields = fields;
        self
    }

    fn with_availability(mut self, available: bool) -> Self {
        self.available = available;
        self
    }

    fn with_delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = Some(delay_ms);
        self
    }
}

#[async_trait::async_trait]
impl TransactionAnalyzer for MockAnalyzer {
    fn name(&self) -> &str {
        &self.name
    }

    fn fields(&self) -> Vec<String> {
        self.fields.clone()
    }

    async fn analyze(&self, _tx: &Transaction) -> Result<HashMap<String, Value>> {
        let mut result = HashMap::new();
        for field in &self.fields {
            result.insert(field.clone(), json!(true));
        }
        Ok(result)
    }

    fn is_available(&self) -> bool {
        self.available
    }

    fn recommended_delay_ms(&self) -> Option<u64> {
        self.delay_ms
    }
}

/// Mock analyzer that uses metadata
struct MetadataAwareAnalyzer {
    name: String,
}

#[async_trait::async_trait]
impl TransactionAnalyzer for MetadataAwareAnalyzer {
    fn name(&self) -> &str {
        &self.name
    }

    fn fields(&self) -> Vec<String> {
        vec!["log_count".to_string(), "inner_ix_count".to_string()]
    }

    async fn analyze(&self, _tx: &Transaction) -> Result<HashMap<String, Value>> {
        let mut result = HashMap::new();
        result.insert("log_count".to_string(), json!(0));
        result.insert("inner_ix_count".to_string(), json!(0));
        Ok(result)
    }

    async fn analyze_with_metadata(
        &self,
        _tx: &Transaction,
        metadata: &ConfirmedTransactionMetadata,
    ) -> Result<HashMap<String, Value>> {
        let mut result = HashMap::new();
        result.insert("log_count".to_string(), json!(metadata.logs.len()));
        result.insert(
            "inner_ix_count".to_string(),
            json!(metadata.inner_instructions.len()),
        );
        Ok(result)
    }
}

fn create_test_transaction() -> Transaction {
    Transaction {
        signatures: vec![Signature::default()],
        message: Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 0,
            },
            account_keys: vec![Pubkey::new_unique()],
            recent_blockhash: Hash::default(),
            instructions: vec![],
        },
    }
}

#[test]
fn test_registry_new() {
    let registry = AnalyzerRegistry::new();
    assert_eq!(registry.list_all().len(), 0);
}

#[test]
fn test_registry_default() {
    let registry = AnalyzerRegistry::default();
    assert_eq!(registry.list_all().len(), 0);
}

#[test]
fn test_register_analyzer() {
    let mut registry = AnalyzerRegistry::new();
    let analyzer = Arc::new(MockAnalyzer::new("test"));

    registry.register(analyzer);

    assert_eq!(registry.list_all().len(), 1);
    assert!(registry.list_all().contains(&"test".to_string()));
}

#[test]
fn test_register_multiple_analyzers() {
    let mut registry = AnalyzerRegistry::new();

    registry.register(Arc::new(MockAnalyzer::new("analyzer1")));
    registry.register(Arc::new(MockAnalyzer::new("analyzer2")));
    registry.register(Arc::new(MockAnalyzer::new("analyzer3")));

    assert_eq!(registry.list_all().len(), 3);
    assert!(registry.list_all().contains(&"analyzer1".to_string()));
    assert!(registry.list_all().contains(&"analyzer2".to_string()));
    assert!(registry.list_all().contains(&"analyzer3".to_string()));
}

#[test]
fn test_get_analyzer() {
    let mut registry = AnalyzerRegistry::new();
    let analyzer = Arc::new(MockAnalyzer::new("test"));
    registry.register(analyzer);

    let retrieved = registry.get("test");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name(), "test");
}

#[test]
fn test_get_nonexistent_analyzer() {
    let registry = AnalyzerRegistry::new();
    let retrieved = registry.get("nonexistent");
    assert!(retrieved.is_none());
}

#[test]
fn test_get_all_fields() {
    let mut registry = AnalyzerRegistry::new();

    registry.register(Arc::new(
        MockAnalyzer::new("analyzer1")
            .with_fields(vec!["field_a".to_string(), "field_b".to_string()]),
    ));
    registry.register(Arc::new(
        MockAnalyzer::new("analyzer2")
            .with_fields(vec!["field_x".to_string(), "field_y".to_string()]),
    ));

    let all_fields = registry.get_all_fields();

    assert_eq!(all_fields.len(), 2);
    assert_eq!(all_fields.get("analyzer1").unwrap().len(), 2);
    assert_eq!(all_fields.get("analyzer2").unwrap().len(), 2);
    assert!(all_fields
        .get("analyzer1")
        .unwrap()
        .contains(&"field_a".to_string()));
    assert!(all_fields
        .get("analyzer2")
        .unwrap()
        .contains(&"field_x".to_string()));
}

#[test]
fn test_has_field_unprefixed() {
    let mut registry = AnalyzerRegistry::new();
    registry.register(Arc::new(
        MockAnalyzer::new("test").with_fields(vec!["my_field".to_string()]),
    ));

    assert!(registry.has_field("my_field"));
    assert!(!registry.has_field("nonexistent_field"));
}

#[test]
fn test_has_field_prefixed() {
    let mut registry = AnalyzerRegistry::new();
    registry.register(Arc::new(
        MockAnalyzer::new("test").with_fields(vec!["my_field".to_string()]),
    ));

    assert!(registry.has_field("test:my_field"));
    assert!(!registry.has_field("test:nonexistent"));
    assert!(!registry.has_field("wrong_analyzer:my_field"));
}

#[tokio::test]
async fn test_analyze_selected_single() {
    let mut registry = AnalyzerRegistry::new();
    registry.register(Arc::new(
        MockAnalyzer::new("test").with_fields(vec!["field1".to_string()]),
    ));

    let tx = create_test_transaction();
    let result = registry
        .analyze_selected(&tx, &vec!["test".to_string()])
        .await
        .unwrap();

    assert!(result.contains_key("field1"));
    assert!(result.contains_key("test:field1"));
    assert_eq!(result["field1"], json!(true));
}

#[tokio::test]
async fn test_analyze_selected_multiple() {
    let mut registry = AnalyzerRegistry::new();
    registry.register(Arc::new(
        MockAnalyzer::new("analyzer1").with_fields(vec!["field_a".to_string()]),
    ));
    registry.register(Arc::new(
        MockAnalyzer::new("analyzer2").with_fields(vec!["field_b".to_string()]),
    ));

    let tx = create_test_transaction();
    let result = registry
        .analyze_selected(&tx, &vec!["analyzer1".to_string(), "analyzer2".to_string()])
        .await
        .unwrap();

    assert!(result.contains_key("field_a"));
    assert!(result.contains_key("field_b"));
    assert!(result.contains_key("analyzer1:field_a"));
    assert!(result.contains_key("analyzer2:field_b"));
}

#[tokio::test]
async fn test_analyze_selected_filters_unavailable() {
    let mut registry = AnalyzerRegistry::new();
    registry.register(Arc::new(
        MockAnalyzer::new("available").with_fields(vec!["field_a".to_string()]),
    ));
    registry.register(Arc::new(
        MockAnalyzer::new("unavailable")
            .with_fields(vec!["field_b".to_string()])
            .with_availability(false),
    ));

    let tx = create_test_transaction();
    let result = registry
        .analyze_selected(
            &tx,
            &vec!["available".to_string(), "unavailable".to_string()],
        )
        .await
        .unwrap();

    assert!(result.contains_key("field_a"));
    assert!(!result.contains_key("field_b")); // Unavailable analyzer not run
}

#[tokio::test]
async fn test_analyze_selected_only_requested() {
    let mut registry = AnalyzerRegistry::new();
    registry.register(Arc::new(
        MockAnalyzer::new("requested").with_fields(vec!["field_a".to_string()]),
    ));
    registry.register(Arc::new(
        MockAnalyzer::new("not_requested").with_fields(vec!["field_b".to_string()]),
    ));

    let tx = create_test_transaction();
    let result = registry
        .analyze_selected(&tx, &vec!["requested".to_string()])
        .await
        .unwrap();

    assert!(result.contains_key("field_a"));
    assert!(!result.contains_key("field_b")); // Not requested
}

#[tokio::test]
async fn test_analyze_all() {
    let mut registry = AnalyzerRegistry::new();
    registry.register(Arc::new(
        MockAnalyzer::new("analyzer1").with_fields(vec!["field_a".to_string()]),
    ));
    registry.register(Arc::new(
        MockAnalyzer::new("analyzer2").with_fields(vec!["field_b".to_string()]),
    ));

    let tx = create_test_transaction();
    let result = registry.analyze_all(&tx).await.unwrap();

    assert!(result.contains_key("field_a"));
    assert!(result.contains_key("field_b"));
}

#[tokio::test]
async fn test_analyze_with_metadata() {
    let mut registry = AnalyzerRegistry::new();
    registry.register(Arc::new(MetadataAwareAnalyzer {
        name: "metadata_analyzer".to_string(),
    }));

    let tx = create_test_transaction();
    let metadata = ConfirmedTransactionMetadata {
        logs: vec!["log1".to_string(), "log2".to_string(), "log3".to_string()],
        inner_instructions: vec![ConfirmedInnerInstruction {
            outer_index: 0,
            program_id: "program1".to_string(),
            data: vec![],
            accounts: vec![],
            stack_height: Some(1),
        }],
    };

    let result = registry
        .analyze_selected_with_metadata(&tx, &vec!["metadata_analyzer".to_string()], &metadata)
        .await
        .unwrap();

    assert_eq!(result["log_count"], json!(3));
    assert_eq!(result["inner_ix_count"], json!(1));
}

#[tokio::test]
async fn test_analyze_with_logs() {
    let mut registry = AnalyzerRegistry::new();
    registry.register(Arc::new(MetadataAwareAnalyzer {
        name: "metadata_analyzer".to_string(),
    }));

    let tx = create_test_transaction();
    let logs = vec!["log1".to_string(), "log2".to_string()];

    let result = registry
        .analyze_selected_with_logs(&tx, &vec!["metadata_analyzer".to_string()], &logs)
        .await
        .unwrap();

    assert_eq!(result["log_count"], json!(2));
    assert_eq!(result["inner_ix_count"], json!(0)); // No inner instructions
}

#[test]
fn test_get_recommended_delay_no_analyzers() {
    let registry = AnalyzerRegistry::new();
    assert_eq!(registry.get_recommended_delay_ms(), 0);
}

#[test]
fn test_get_recommended_delay_no_limits() {
    let mut registry = AnalyzerRegistry::new();
    registry.register(Arc::new(MockAnalyzer::new("test1")));
    registry.register(Arc::new(MockAnalyzer::new("test2")));

    assert_eq!(registry.get_recommended_delay_ms(), 0);
}

#[test]
fn test_get_recommended_delay_single_limit() {
    let mut registry = AnalyzerRegistry::new();
    registry.register(Arc::new(MockAnalyzer::new("test1").with_delay(100)));
    registry.register(Arc::new(MockAnalyzer::new("test2")));

    assert_eq!(registry.get_recommended_delay_ms(), 100);
}

#[test]
fn test_get_recommended_delay_multiple_limits() {
    let mut registry = AnalyzerRegistry::new();
    registry.register(Arc::new(MockAnalyzer::new("test1").with_delay(100)));
    registry.register(Arc::new(MockAnalyzer::new("test2").with_delay(250)));
    registry.register(Arc::new(MockAnalyzer::new("test3").with_delay(50)));

    // Should return the maximum delay
    assert_eq!(registry.get_recommended_delay_ms(), 250);
}

#[test]
fn test_get_recommended_delay_ignores_unavailable() {
    let mut registry = AnalyzerRegistry::new();
    registry.register(Arc::new(MockAnalyzer::new("available").with_delay(100)));
    registry.register(Arc::new(
        MockAnalyzer::new("unavailable")
            .with_delay(500)
            .with_availability(false),
    ));

    // Should only consider available analyzers
    assert_eq!(registry.get_recommended_delay_ms(), 100);
}

#[test]
fn test_confirmed_inner_instruction_creation() {
    let inner_ix = ConfirmedInnerInstruction {
        outer_index: 0,
        program_id: "test_program".to_string(),
        data: vec![1, 2, 3],
        accounts: vec![0, 1, 2],
        stack_height: Some(2),
    };

    assert_eq!(inner_ix.outer_index, 0);
    assert_eq!(inner_ix.program_id, "test_program");
    assert_eq!(inner_ix.data, vec![1, 2, 3]);
    assert_eq!(inner_ix.accounts, vec![0, 1, 2]);
    assert_eq!(inner_ix.stack_height, Some(2));
}

#[test]
fn test_confirmed_metadata_default() {
    let metadata = ConfirmedTransactionMetadata::default();
    assert!(metadata.logs.is_empty());
    assert!(metadata.inner_instructions.is_empty());
}

#[test]
fn test_confirmed_metadata_with_data() {
    let metadata = ConfirmedTransactionMetadata {
        logs: vec!["log1".to_string(), "log2".to_string()],
        inner_instructions: vec![ConfirmedInnerInstruction {
            outer_index: 0,
            program_id: "program1".to_string(),
            data: vec![],
            accounts: vec![],
            stack_height: None,
        }],
    };

    assert_eq!(metadata.logs.len(), 2);
    assert_eq!(metadata.inner_instructions.len(), 1);
}

#[tokio::test]
async fn test_analyzer_trait_default_analyze_with_metadata() {
    let analyzer = MockAnalyzer::new("test").with_fields(vec!["field1".to_string()]);
    let tx = create_test_transaction();
    let metadata = ConfirmedTransactionMetadata::default();

    // Default implementation should delegate to analyze()
    let result = analyzer
        .analyze_with_metadata(&tx, &metadata)
        .await
        .unwrap();

    assert!(result.contains_key("field1"));
    assert_eq!(result["field1"], json!(true));
}

#[tokio::test]
async fn test_analyzer_trait_analyze_with_logs() {
    let analyzer = MockAnalyzer::new("test").with_fields(vec!["field1".to_string()]);
    let tx = create_test_transaction();
    let logs = vec!["log1".to_string()];

    let result = analyzer.analyze_with_logs(&tx, &logs).await.unwrap();

    assert!(result.contains_key("field1"));
}

#[test]
fn test_analyzer_trait_defaults() {
    let analyzer = MockAnalyzer::new("test");

    assert!(analyzer.is_available());
    assert_eq!(analyzer.estimated_latency_ms(), 1);
    assert_eq!(analyzer.recommended_delay_ms(), None);
}

#[test]
fn test_list_all_empty() {
    let registry = AnalyzerRegistry::new();
    assert!(registry.list_all().is_empty());
}

#[test]
fn test_list_all_with_analyzers() {
    let mut registry = AnalyzerRegistry::new();
    registry.register(Arc::new(MockAnalyzer::new("analyzer1")));
    registry.register(Arc::new(MockAnalyzer::new("analyzer2")));

    let list = registry.list_all();
    assert_eq!(list.len(), 2);
    assert!(list.contains(&"analyzer1".to_string()));
    assert!(list.contains(&"analyzer2".to_string()));
}

#[tokio::test]
async fn test_field_prefixing_with_conflicts() {
    let mut registry = AnalyzerRegistry::new();

    // Both analyzers provide "count" field
    registry.register(Arc::new(
        MockAnalyzer::new("analyzer1").with_fields(vec!["count".to_string()]),
    ));
    registry.register(Arc::new(
        MockAnalyzer::new("analyzer2").with_fields(vec!["count".to_string()]),
    ));

    let tx = create_test_transaction();
    let result = registry.analyze_all(&tx).await.unwrap();

    // Both prefixed versions should exist
    assert!(result.contains_key("analyzer1:count"));
    assert!(result.contains_key("analyzer2:count"));

    // Unprefixed version should exist (from first analyzer processed)
    assert!(result.contains_key("count"));
}

#[tokio::test]
async fn test_analyze_empty_selection() {
    let mut registry = AnalyzerRegistry::new();
    registry.register(Arc::new(MockAnalyzer::new("test")));

    let tx = create_test_transaction();
    let result = registry.analyze_selected(&tx, &vec![]).await.unwrap();

    // No analyzers selected, should return empty
    assert!(result.is_empty());
}

#[test]
fn test_register_overwrites_existing() {
    let mut registry = AnalyzerRegistry::new();

    registry.register(Arc::new(
        MockAnalyzer::new("test").with_fields(vec!["field1".to_string()]),
    ));
    registry.register(Arc::new(
        MockAnalyzer::new("test").with_fields(vec!["field2".to_string()]),
    ));

    // Should only have one analyzer named "test"
    assert_eq!(registry.list_all().len(), 1);

    // Should have the fields from the second registration
    let analyzer = registry.get("test").unwrap();
    assert_eq!(analyzer.fields(), vec!["field2".to_string()]);
}
