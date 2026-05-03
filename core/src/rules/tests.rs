#[cfg(test)]
mod rule_tests {
    use super::super::*;
    use crate::rules::analyzers::BasicAnalyzer;
    use solana_sdk::{message::Message, pubkey::Pubkey, transaction::Transaction};
    use solana_system_interface::instruction as system_instruction;
    use std::sync::Arc;

    fn create_test_transaction() -> Transaction {
        let from = Pubkey::new_unique();
        let to = Pubkey::new_unique();
        let instruction = system_instruction::transfer(&from, &to, 1_000_000);
        let message = Message::new(&[instruction], Some(&from));
        Transaction::new_unsigned(message)
    }

    #[tokio::test]
    async fn test_simple_rule_matches() {
        let rule_json = r#"
        {
            "version": "1.0",
            "id": "test-rule",
            "name": "Test Rule",
            "enabled": true,
            "rule": {
                "action": "block",
                "conditions": {
                    "field": "instruction_count",
                    "operator": "greater_than",
                    "value": 0
                },
                "message": "Test block message"
            }
        }
        "#;

        let mut registry = AnalyzerRegistry::new();
        registry.register(Arc::new(BasicAnalyzer::new()));

        let mut engine = RuleEngine::new(registry);
        let rule: types::RuleDefinition = serde_json::from_str(rule_json).unwrap();
        engine.load_rules(vec![rule]).unwrap();

        let tx = create_test_transaction();
        let decision = engine.evaluate(&tx).await.unwrap();

        assert!(decision.matched);
        assert_eq!(decision.action, types::RuleAction::Block);
        assert_eq!(decision.rule_id, "test-rule");
    }

    #[tokio::test]
    async fn test_rule_does_not_match() {
        let rule_json = r#"
        {
            "version": "1.0",
            "id": "test-rule",
            "name": "Test Rule",
            "enabled": true,
            "rule": {
                "action": "block",
                "conditions": {
                    "field": "instruction_count",
                    "operator": "greater_than",
                    "value": 100
                },
                "message": "Test block message"
            }
        }
        "#;

        let mut registry = AnalyzerRegistry::new();
        registry.register(Arc::new(BasicAnalyzer::new()));

        let mut engine = RuleEngine::new(registry);
        let rule: types::RuleDefinition = serde_json::from_str(rule_json).unwrap();
        engine.load_rules(vec![rule]).unwrap();

        let tx = create_test_transaction();
        let decision = engine.evaluate(&tx).await.unwrap();

        assert!(!decision.matched);
    }

    #[tokio::test]
    async fn test_compound_all_condition() {
        let rule_json = r#"
        {
            "version": "1.0",
            "id": "test-compound",
            "name": "Test Compound Rule",
            "enabled": true,
            "rule": {
                "action": "alert",
                "conditions": {
                    "all": [
                        {"field": "instruction_count", "operator": "greater_than", "value": 0},
                        {"field": "has_instructions", "operator": "equals", "value": true}
                    ]
                },
                "message": "Multiple conditions matched"
            }
        }
        "#;

        let mut registry = AnalyzerRegistry::new();
        registry.register(Arc::new(BasicAnalyzer::new()));

        let mut engine = RuleEngine::new(registry);
        let rule: types::RuleDefinition = serde_json::from_str(rule_json).unwrap();
        engine.load_rules(vec![rule]).unwrap();

        let tx = create_test_transaction();
        let decision = engine.evaluate(&tx).await.unwrap();

        assert!(decision.matched);
        assert_eq!(decision.action, types::RuleAction::Alert);
    }

    #[tokio::test]
    async fn test_disabled_rule() {
        let rule_json = r#"
        {
            "version": "1.0",
            "id": "disabled-rule",
            "name": "Disabled Rule",
            "enabled": false,
            "rule": {
                "action": "block",
                "conditions": {
                    "field": "instruction_count",
                    "operator": "greater_than",
                    "value": 0
                },
                "message": "Should not match"
            }
        }
        "#;

        let mut registry = AnalyzerRegistry::new();
        registry.register(Arc::new(BasicAnalyzer::new()));

        let mut engine = RuleEngine::new(registry);
        let rule: types::RuleDefinition = serde_json::from_str(rule_json).unwrap();
        engine.load_rules(vec![rule]).unwrap();

        let tx = create_test_transaction();
        let decision = engine.evaluate(&tx).await.unwrap();

        assert!(!decision.matched);
    }

    #[test]
    fn test_rule_validation_fails_on_missing_field() {
        let rule_json = r#"
        {
            "version": "1.0",
            "id": "invalid-rule",
            "name": "Invalid Rule",
            "enabled": true,
            "rule": {
                "action": "block",
                "conditions": {
                    "field": "nonexistent_field",
                    "operator": "equals",
                    "value": true
                },
                "message": "Should fail validation"
            }
        }
        "#;

        let mut registry = AnalyzerRegistry::new();
        registry.register(Arc::new(BasicAnalyzer::new()));

        let mut engine = RuleEngine::new(registry);
        let rule: types::RuleDefinition = serde_json::from_str(rule_json).unwrap();

        // Should fail during load_rules
        let result = engine.load_rules(vec![rule]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("nonexistent_field"));
    }

    #[test]
    fn test_rule_validation_passes_with_valid_fields() {
        let rule_json = r#"
        {
            "version": "1.0",
            "id": "valid-rule",
            "name": "Valid Rule",
            "enabled": true,
            "rule": {
                "action": "block",
                "conditions": {
                    "field": "instruction_count",
                    "operator": "greater_than",
                    "value": 0
                },
                "message": "Should pass validation"
            }
        }
        "#;

        let mut registry = AnalyzerRegistry::new();
        registry.register(Arc::new(BasicAnalyzer::new()));

        let mut engine = RuleEngine::new(registry);
        let rule: types::RuleDefinition = serde_json::from_str(rule_json).unwrap();

        // Should succeed
        let result = engine.load_rules(vec![rule]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_disabled_rule_skips_validation() {
        let rule_json = r#"
        {
            "version": "1.0",
            "id": "disabled-invalid",
            "name": "Disabled Invalid Rule",
            "enabled": false,
            "rule": {
                "action": "block",
                "conditions": {
                    "field": "nonexistent_field",
                    "operator": "equals",
                    "value": true
                },
                "message": "Disabled so validation should be skipped"
            }
        }
        "#;

        let mut registry = AnalyzerRegistry::new();
        registry.register(Arc::new(BasicAnalyzer::new()));

        let mut engine = RuleEngine::new(registry);
        let rule: types::RuleDefinition = serde_json::from_str(rule_json).unwrap();

        // Should succeed because rule is disabled
        let result = engine.load_rules(vec![rule]);
        assert!(result.is_ok());
    }
}
