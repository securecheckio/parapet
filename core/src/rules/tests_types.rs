use super::types::*;
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_rule_action_display() {
    assert_eq!(RuleAction::Block.to_string(), "block");
    assert_eq!(RuleAction::Alert.to_string(), "alert");
    assert_eq!(RuleAction::Pass.to_string(), "pass");
}

#[test]
fn test_rule_action_from_str() {
    assert_eq!("block".parse::<RuleAction>().unwrap(), RuleAction::Block);
    assert_eq!("alert".parse::<RuleAction>().unwrap(), RuleAction::Alert);
    assert_eq!("pass".parse::<RuleAction>().unwrap(), RuleAction::Pass);

    // Case insensitive
    assert_eq!("BLOCK".parse::<RuleAction>().unwrap(), RuleAction::Block);
    assert_eq!("Alert".parse::<RuleAction>().unwrap(), RuleAction::Alert);
    assert_eq!("PaSs".parse::<RuleAction>().unwrap(), RuleAction::Pass);
}

#[test]
fn test_rule_action_from_str_invalid() {
    let result = "invalid".parse::<RuleAction>();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid action"));

    let result = "".parse::<RuleAction>();
    assert!(result.is_err());

    let result = "warn".parse::<RuleAction>();
    assert!(result.is_err());
}

#[test]
fn test_rule_action_serialization() {
    let action = RuleAction::Block;
    let json = serde_json::to_string(&action).unwrap();
    assert_eq!(json, "\"block\"");

    let action = RuleAction::Alert;
    let json = serde_json::to_string(&action).unwrap();
    assert_eq!(json, "\"alert\"");

    let action = RuleAction::Pass;
    let json = serde_json::to_string(&action).unwrap();
    assert_eq!(json, "\"pass\"");
}

#[test]
fn test_rule_action_deserialization() {
    let action: RuleAction = serde_json::from_str("\"block\"").unwrap();
    assert_eq!(action, RuleAction::Block);

    let action: RuleAction = serde_json::from_str("\"alert\"").unwrap();
    assert_eq!(action, RuleAction::Alert);

    let action: RuleAction = serde_json::from_str("\"pass\"").unwrap();
    assert_eq!(action, RuleAction::Pass);
}

#[test]
fn test_action_override_all() {
    let override_action = ActionOverride::All(RuleAction::Alert);

    assert_eq!(override_action.apply(RuleAction::Block), RuleAction::Alert);
    assert_eq!(override_action.apply(RuleAction::Pass), RuleAction::Alert);
    assert_eq!(override_action.apply(RuleAction::Alert), RuleAction::Alert);
}

#[test]
fn test_action_override_specific() {
    let mut map = HashMap::new();
    map.insert(RuleAction::Block, RuleAction::Alert);
    map.insert(RuleAction::Pass, RuleAction::Alert);

    let override_action = ActionOverride::Specific(map);

    assert_eq!(override_action.apply(RuleAction::Block), RuleAction::Alert);
    assert_eq!(override_action.apply(RuleAction::Pass), RuleAction::Alert);
    assert_eq!(override_action.apply(RuleAction::Alert), RuleAction::Alert); // No mapping, returns original
}

#[test]
fn test_action_override_from_env_str_all() {
    let override_action = ActionOverride::from_env_str("alert").unwrap();
    assert_eq!(override_action.apply(RuleAction::Block), RuleAction::Alert);

    let override_action = ActionOverride::from_env_str("block").unwrap();
    assert_eq!(override_action.apply(RuleAction::Pass), RuleAction::Block);

    let override_action = ActionOverride::from_env_str("pass").unwrap();
    assert_eq!(override_action.apply(RuleAction::Block), RuleAction::Pass);
}

#[test]
fn test_action_override_from_env_str_specific_single() {
    let override_action = ActionOverride::from_env_str("block:alert").unwrap();
    assert_eq!(override_action.apply(RuleAction::Block), RuleAction::Alert);
    assert_eq!(override_action.apply(RuleAction::Pass), RuleAction::Pass); // No mapping
}

#[test]
fn test_action_override_from_env_str_specific_multiple() {
    let override_action = ActionOverride::from_env_str("block:alert,pass:alert").unwrap();
    assert_eq!(override_action.apply(RuleAction::Block), RuleAction::Alert);
    assert_eq!(override_action.apply(RuleAction::Pass), RuleAction::Alert);
    assert_eq!(override_action.apply(RuleAction::Alert), RuleAction::Alert); // No mapping
}

#[test]
fn test_action_override_from_env_str_with_whitespace() {
    let override_action = ActionOverride::from_env_str(" block : alert , pass : alert ").unwrap();
    assert_eq!(override_action.apply(RuleAction::Block), RuleAction::Alert);
    assert_eq!(override_action.apply(RuleAction::Pass), RuleAction::Alert);
}

#[test]
fn test_action_override_from_env_str_empty() {
    let result = ActionOverride::from_env_str("");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("cannot be empty"));

    let result = ActionOverride::from_env_str("   ");
    assert!(result.is_err());
}

#[test]
fn test_action_override_from_env_str_invalid_format() {
    let result = ActionOverride::from_env_str("block:alert:pass");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid override pair"));

    let result = ActionOverride::from_env_str("block:");
    assert!(result.is_err());

    let result = ActionOverride::from_env_str(":alert");
    assert!(result.is_err());
}

#[test]
fn test_action_override_from_env_str_invalid_action() {
    let result = ActionOverride::from_env_str("invalid");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid action"));

    let result = ActionOverride::from_env_str("block:invalid");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid replacement action"));

    let result = ActionOverride::from_env_str("invalid:alert");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid original action"));
}

#[test]
fn test_rule_decision_no_match() {
    let decision = RuleDecision::no_match();

    assert_eq!(decision.action, RuleAction::Pass);
    assert_eq!(decision.rule_id, "");
    assert_eq!(decision.rule_name, "");
    assert_eq!(decision.message, "");
    assert!(!decision.matched);
    assert_eq!(decision.total_risk, 0);
    assert!(decision.matched_rules.is_empty());
    assert_eq!(decision.structural_risk, None);
    assert_eq!(decision.simulation_risk, None);
    assert!(!decision.is_simulation);
}

#[test]
fn test_simple_condition_serialization() {
    let condition = SimpleCondition {
        field: "has_sol_transfer".to_string(),
        operator: ComparisonOperator::Equals,
        value: json!(true),
    };

    let json = serde_json::to_value(&condition).unwrap();
    assert_eq!(json["field"], "has_sol_transfer");
    assert_eq!(json["operator"], "equals");
    assert_eq!(json["value"], true);
}

#[test]
fn test_simple_condition_deserialization() {
    let json = json!({
        "field": "risk_score",
        "operator": "greater_than",
        "value": 50
    });

    let condition: SimpleCondition = serde_json::from_value(json).unwrap();
    assert_eq!(condition.field, "risk_score");
    assert_eq!(condition.value, 50);
}

#[test]
fn test_comparison_operator_serialization() {
    assert_eq!(
        serde_json::to_string(&ComparisonOperator::Equals).unwrap(),
        "\"equals\""
    );
    assert_eq!(
        serde_json::to_string(&ComparisonOperator::NotEquals).unwrap(),
        "\"not_equals\""
    );
    assert_eq!(
        serde_json::to_string(&ComparisonOperator::GreaterThan).unwrap(),
        "\"greater_than\""
    );
    assert_eq!(
        serde_json::to_string(&ComparisonOperator::LessThan).unwrap(),
        "\"less_than\""
    );
    assert_eq!(
        serde_json::to_string(&ComparisonOperator::GreaterThanOrEqual).unwrap(),
        "\"greater_than_or_equal\""
    );
    assert_eq!(
        serde_json::to_string(&ComparisonOperator::LessThanOrEqual).unwrap(),
        "\"less_than_or_equal\""
    );
    assert_eq!(
        serde_json::to_string(&ComparisonOperator::In).unwrap(),
        "\"in\""
    );
    assert_eq!(
        serde_json::to_string(&ComparisonOperator::NotIn).unwrap(),
        "\"not_in\""
    );
    assert_eq!(
        serde_json::to_string(&ComparisonOperator::Contains).unwrap(),
        "\"contains\""
    );
    assert_eq!(
        serde_json::to_string(&ComparisonOperator::IsNotSet).unwrap(),
        "\"isnotset\""
    );
    assert_eq!(
        serde_json::to_string(&ComparisonOperator::Exists).unwrap(),
        "\"exists\""
    );
}

#[test]
fn test_compound_condition_all_serialization() {
    let condition = CompoundCondition {
        all: Some(vec![
            RuleCondition::Simple(SimpleCondition {
                field: "field1".to_string(),
                operator: ComparisonOperator::Equals,
                value: json!(true),
            }),
            RuleCondition::Simple(SimpleCondition {
                field: "field2".to_string(),
                operator: ComparisonOperator::GreaterThan,
                value: json!(10),
            }),
        ]),
        any: None,
        not: None,
    };

    let json = serde_json::to_value(&condition).unwrap();
    assert!(json["all"].is_array());
    assert_eq!(json["all"].as_array().unwrap().len(), 2);
}

#[test]
fn test_compound_condition_any_serialization() {
    let condition = CompoundCondition {
        all: None,
        any: Some(vec![RuleCondition::Simple(SimpleCondition {
            field: "field1".to_string(),
            operator: ComparisonOperator::Equals,
            value: json!(true),
        })]),
        not: None,
    };

    let json = serde_json::to_value(&condition).unwrap();
    assert!(json["any"].is_array());
    assert_eq!(json.get("all"), None);
}

#[test]
fn test_compound_condition_not_serialization() {
    let condition = CompoundCondition {
        all: None,
        any: None,
        not: Some(Box::new(RuleCondition::Simple(SimpleCondition {
            field: "field1".to_string(),
            operator: ComparisonOperator::Equals,
            value: json!(false),
        }))),
    };

    let json = serde_json::to_value(&condition).unwrap();
    assert!(json["not"].is_object());
}

#[test]
fn test_rule_condition_simple_variant() {
    let condition = RuleCondition::Simple(SimpleCondition {
        field: "test".to_string(),
        operator: ComparisonOperator::Equals,
        value: json!(true),
    });

    let json = serde_json::to_value(&condition).unwrap();
    assert_eq!(json["field"], "test");
}

#[test]
fn test_rule_condition_compound_variant() {
    let condition = RuleCondition::Compound(CompoundCondition {
        all: Some(vec![]),
        any: None,
        not: None,
    });

    let json = serde_json::to_value(&condition).unwrap();
    assert!(json["all"].is_array());
}

#[test]
fn test_rule_serialization() {
    let rule = Rule {
        action: RuleAction::Block,
        conditions: RuleCondition::Simple(SimpleCondition {
            field: "has_sol_transfer".to_string(),
            operator: ComparisonOperator::Equals,
            value: json!(true),
        }),
        message: "Test message".to_string(),
        flowstate: None,
    };

    let json = serde_json::to_value(&rule).unwrap();
    assert_eq!(json["action"], "block");
    assert_eq!(json["message"], "Test message");
    assert_eq!(json["conditions"]["field"], "has_sol_transfer");
}

#[test]
fn test_rule_deserialization() {
    let json = json!({
        "action": "alert",
        "conditions": {
            "field": "risk_score",
            "operator": "greater_than",
            "value": 80
        },
        "message": "High risk detected"
    });

    let rule: Rule = serde_json::from_value(json).unwrap();
    assert_eq!(rule.action, RuleAction::Alert);
    assert_eq!(rule.message, "High risk detected");
}

#[test]
fn test_rule_definition_full() {
    let mut metadata = HashMap::new();
    metadata.insert("category".to_string(), json!("security"));
    metadata.insert("severity".to_string(), json!("high"));

    let rule_def = RuleDefinition {
        version: "1.0".to_string(),
        id: "test-rule-1".to_string(),
        name: "Test Rule".to_string(),
        description: Some("A test rule".to_string()),
        author: Some("Test Author".to_string()),
        enabled: true,
        tags: vec!["security".to_string(), "test".to_string()],
        rule: Rule {
            action: RuleAction::Block,
            conditions: RuleCondition::Simple(SimpleCondition {
                field: "test".to_string(),
                operator: ComparisonOperator::Equals,
                value: json!(true),
            }),
            message: "Test".to_string(),
            flowstate: None,
        },
        metadata,
    };

    let json = serde_json::to_value(&rule_def).unwrap();
    assert_eq!(json["version"], "1.0");
    assert_eq!(json["id"], "test-rule-1");
    assert_eq!(json["name"], "Test Rule");
    assert_eq!(json["enabled"], true);
    assert_eq!(json["tags"].as_array().unwrap().len(), 2);
}

#[test]
fn test_rule_definition_minimal() {
    let rule_def = RuleDefinition {
        version: "1.0".to_string(),
        id: "minimal".to_string(),
        name: "Minimal".to_string(),
        description: None,
        author: None,
        enabled: false,
        tags: vec![],
        rule: Rule {
            action: RuleAction::Pass,
            conditions: RuleCondition::Simple(SimpleCondition {
                field: "test".to_string(),
                operator: ComparisonOperator::Equals,
                value: json!(true),
            }),
            message: "".to_string(),
            flowstate: None,
        },
        metadata: HashMap::new(),
    };

    let json = serde_json::to_value(&rule_def).unwrap();
    assert_eq!(json["enabled"], false);
    assert!(json["tags"].as_array().unwrap().is_empty());
    assert!(json["metadata"].as_object().unwrap().is_empty());
}

#[test]
fn test_rule_definition_deserialization() {
    let json = json!({
        "version": "1.0",
        "id": "test-1",
        "name": "Test",
        "enabled": true,
        "rule": {
            "action": "block",
            "conditions": {
                "field": "test",
                "operator": "equals",
                "value": true
            },
            "message": "Test"
        }
    });

    let rule_def: RuleDefinition = serde_json::from_value(json).unwrap();
    assert_eq!(rule_def.id, "test-1");
    assert!(rule_def.enabled);
    assert!(rule_def.tags.is_empty()); // Default
    assert!(rule_def.metadata.is_empty()); // Default
}

#[test]
fn test_matched_rule_serialization() {
    let matched = MatchedRule {
        rule_id: "rule-1".to_string(),
        rule_name: "Test Rule".to_string(),
        action: RuleAction::Alert,
        weight: 75,
        message: "Matched".to_string(),
    };

    let json = serde_json::to_value(&matched).unwrap();
    assert_eq!(json["rule_id"], "rule-1");
    assert_eq!(json["action"], "alert");
    assert_eq!(json["weight"], 75);
}

#[test]
fn test_matched_rule_deserialization() {
    let json = json!({
        "rule_id": "rule-2",
        "rule_name": "Another Rule",
        "action": "block",
        "weight": 90,
        "message": "High risk"
    });

    let matched: MatchedRule = serde_json::from_value(json).unwrap();
    assert_eq!(matched.rule_id, "rule-2");
    assert_eq!(matched.action, RuleAction::Block);
    assert_eq!(matched.weight, 90);
}

#[test]
fn test_rule_decision_with_risks() {
    let decision = RuleDecision {
        action: RuleAction::Block,
        rule_id: "test".to_string(),
        rule_name: "Test".to_string(),
        message: "Blocked".to_string(),
        matched: true,
        total_risk: 95,
        matched_rules: vec![],
        structural_risk: Some(80),
        simulation_risk: Some(15),
        is_simulation: true,
    };

    assert_eq!(decision.total_risk, 95);
    assert_eq!(decision.structural_risk, Some(80));
    assert_eq!(decision.simulation_risk, Some(15));
    assert!(decision.is_simulation);
}

#[test]
fn test_nested_compound_conditions() {
    let condition = RuleCondition::Compound(CompoundCondition {
        all: Some(vec![
            RuleCondition::Simple(SimpleCondition {
                field: "field1".to_string(),
                operator: ComparisonOperator::Equals,
                value: json!(true),
            }),
            RuleCondition::Compound(CompoundCondition {
                any: Some(vec![
                    RuleCondition::Simple(SimpleCondition {
                        field: "field2".to_string(),
                        operator: ComparisonOperator::GreaterThan,
                        value: json!(10),
                    }),
                    RuleCondition::Simple(SimpleCondition {
                        field: "field3".to_string(),
                        operator: ComparisonOperator::LessThan,
                        value: json!(5),
                    }),
                ]),
                all: None,
                not: None,
            }),
        ]),
        any: None,
        not: None,
    });

    let json = serde_json::to_value(&condition).unwrap();
    assert!(json["all"].is_array());
    assert_eq!(json["all"].as_array().unwrap().len(), 2);
}

#[test]
fn test_rule_action_equality() {
    assert_eq!(RuleAction::Block, RuleAction::Block);
    assert_ne!(RuleAction::Block, RuleAction::Alert);
    assert_ne!(RuleAction::Alert, RuleAction::Pass);
}

#[test]
fn test_rule_action_hash() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    set.insert(RuleAction::Block);
    set.insert(RuleAction::Alert);
    set.insert(RuleAction::Block); // Duplicate

    assert_eq!(set.len(), 2);
    assert!(set.contains(&RuleAction::Block));
    assert!(set.contains(&RuleAction::Alert));
    assert!(!set.contains(&RuleAction::Pass));
}
