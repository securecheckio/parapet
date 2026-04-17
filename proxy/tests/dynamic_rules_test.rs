/// Dynamic rules tests - live rule updates from Redis
use parapet_core::rules::analyzers::*;
use parapet_core::rules::types::{Rule, RuleAction, RuleDefinition};
use parapet_core::rules::{AnalyzerRegistry, RuleEngine};
use redis::AsyncCommands;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_rule_engine_creation() {
    let mut registry = AnalyzerRegistry::new();
    registry.register(Arc::new(BasicAnalyzer::new()));

    let engine = RuleEngine::new(registry).with_flowstate(None);

    assert_eq!(engine.rule_count(), 0);
    assert_eq!(engine.enabled_rule_count(), 0);
}

#[tokio::test]
async fn test_load_rules_dynamically() {
    let mut registry = AnalyzerRegistry::new();
    registry.register(Arc::new(BasicAnalyzer::new()));
    registry.register(Arc::new(SystemProgramAnalyzer::new()));

    let mut engine = RuleEngine::new(registry).with_flowstate(None);

    // Load first rule
    let rule1 = RuleDefinition {
        version: "1.0".to_string(),
        id: "dynamic-rule-1".to_string(),
        name: "Test Rule 1".to_string(),
        description: None,
        author: None,
        enabled: true,
        tags: vec![],
        rule: Rule {
            action: RuleAction::Alert,
            conditions: serde_json::from_value(json!({
                "all": [
                    {
                        "field": "basic:instruction_count",
                        "operator": "greater_than",
                        "value": 5
                    }
                ]
            }))
            .unwrap(),
            message: "Many instructions".to_string(),
            flowstate: None,
        },
        metadata: Default::default(),
    };

    engine.load_rules(vec![rule1.clone()]).unwrap();
    assert_eq!(engine.enabled_rule_count(), 1);

    // Load second rule (replaces previous rules)
    let rule2 = RuleDefinition {
        version: "1.0".to_string(),
        id: "dynamic-rule-2".to_string(),
        name: "Test Rule 2".to_string(),
        description: None,
        author: None,
        enabled: true,
        tags: vec![],
        rule: Rule {
            action: RuleAction::Pass,
            conditions: serde_json::from_value(json!({
                "all": [
                    {
                        "field": "basic:instruction_count",
                        "operator": "equals",
                        "value": 1
                    }
                ]
            }))
            .unwrap(),
            message: "Single instruction".to_string(),
            flowstate: None,
        },
        metadata: Default::default(),
    };

    engine.load_rules(vec![rule2]).unwrap();
    assert_eq!(engine.enabled_rule_count(), 1);

    // Load both rules together
    let rule1_new = rule1.clone();
    let rule2_new = RuleDefinition {
        version: "1.0".to_string(),
        id: "dynamic-rule-3".to_string(),
        name: "Test Rule 3".to_string(),
        description: None,
        author: None,
        enabled: true,
        tags: vec![],
        rule: Rule {
            action: RuleAction::Alert,
            conditions: serde_json::from_value(json!({
                "all": [
                    {
                        "field": "basic:instruction_count",
                        "operator": "less_than",
                        "value": 3
                    }
                ]
            }))
            .unwrap(),
            message: "Few instructions".to_string(),
            flowstate: None,
        },
        metadata: Default::default(),
    };

    engine.load_rules(vec![rule1_new, rule2_new]).unwrap();
    assert_eq!(engine.enabled_rule_count(), 2);
}

#[tokio::test]
async fn test_redis_dynamic_rule_storage() {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

    let client = match redis::Client::open(redis_url.as_str()) {
        Ok(client) => client,
        Err(_) => {
            println!("⚠️  Skipping test: Redis not available");
            return;
        }
    };

    let mut conn = match client.get_multiplexed_async_connection().await {
        Ok(conn) => conn,
        Err(_) => {
            println!("⚠️  Skipping test: Cannot connect to Redis");
            return;
        }
    };

    // Store a dynamic rule
    let rule_id = format!("test_rule_{}", uuid::Uuid::new_v4());
    let rule = json!({
        "version": "1.0",
        "id": rule_id,
        "name": "Dynamic Test Rule",
        "enabled": true,
        "rule": {
            "action": "alert",
            "conditions": {
                "all": [
                    {
                        "field": "basic:instruction_count",
                        "operator": "greater_than",
                        "value": 10
                    }
                ]
            },
            "message": "High instruction count"
        }
    });

    let rule_key = format!("dynamic_rule:{}", rule_id);
    let _: () = conn
        .set_ex(&rule_key, serde_json::to_string(&rule).unwrap(), 3600)
        .await
        .unwrap();

    // Verify stored
    let stored: String = conn.get(&rule_key).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stored).unwrap();

    assert_eq!(parsed["id"], rule_id);
    assert_eq!(parsed["enabled"], true);
    assert_eq!(parsed["rule"]["action"], "alert");

    // Cleanup
    let _: () = conn.del(&rule_key).await.unwrap();
}

#[tokio::test]
async fn test_rule_priority_handling() {
    let mut registry = AnalyzerRegistry::new();
    registry.register(Arc::new(BasicAnalyzer::new()));

    let mut engine = RuleEngine::new(registry).with_flowstate(None);

    // Load rules with different actions
    let block_rule = RuleDefinition {
        version: "1.0".to_string(),
        id: "block-rule".to_string(),
        name: "Block Rule".to_string(),
        description: None,
        author: None,
        enabled: true,
        tags: vec![],
        rule: Rule {
            action: RuleAction::Block,
            conditions: serde_json::from_value(json!({
                "all": [
                    {
                        "field": "basic:instruction_count",
                        "operator": "greater_than",
                        "value": 0
                    }
                ]
            }))
            .unwrap(),
            message: "Blocked".to_string(),
            flowstate: None,
        },
        metadata: Default::default(),
    };

    let pass_rule = RuleDefinition {
        version: "1.0".to_string(),
        id: "pass-rule".to_string(),
        name: "Pass Rule".to_string(),
        description: None,
        author: None,
        enabled: true,
        tags: vec![],
        rule: Rule {
            action: RuleAction::Pass,
            conditions: serde_json::from_value(json!({
                "all": [
                    {
                        "field": "basic:instruction_count",
                        "operator": "greater_than",
                        "value": 0
                    }
                ]
            }))
            .unwrap(),
            message: "Passed".to_string(),
            flowstate: None,
        },
        metadata: Default::default(),
    };

    engine.load_rules(vec![block_rule, pass_rule]).unwrap();

    // Both rules should be loaded
    assert_eq!(engine.enabled_rule_count(), 2);
}

#[tokio::test]
async fn test_rule_update_via_redis() {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

    let client = match redis::Client::open(redis_url.as_str()) {
        Ok(client) => client,
        Err(_) => {
            println!("⚠️  Skipping test: Redis not available");
            return;
        }
    };

    let mut conn = match client.get_multiplexed_async_connection().await {
        Ok(conn) => conn,
        Err(_) => {
            println!("⚠️  Skipping test: Cannot connect to Redis");
            return;
        }
    };

    // Create initial rule
    let rule_id = format!("update_rule_{}", uuid::Uuid::new_v4());
    let rule_v1 = json!({
        "version": "1.0",
        "id": rule_id,
        "name": "Rule Version 1",
        "enabled": true,
        "rule": {
            "action": "alert",
            "conditions": {
                "all": [{"field": "basic:instruction_count", "operator": "equals", "value": 5}]
            },
            "message": "Version 1"
        }
    });

    let rule_key = format!("dynamic_rule:{}", rule_id);
    let _: () = conn
        .set_ex(&rule_key, serde_json::to_string(&rule_v1).unwrap(), 3600)
        .await
        .unwrap();

    // Update rule
    let rule_v2 = json!({
        "version": "1.0",
        "id": rule_id,
        "name": "Rule Version 2",
        "enabled": true,
        "rule": {
            "action": "block",
            "conditions": {
                "all": [{"field": "basic:instruction_count", "operator": "equals", "value": 10}]
            },
            "message": "Version 2 - Updated"
        }
    });

    let _: () = conn
        .set_ex(&rule_key, serde_json::to_string(&rule_v2).unwrap(), 3600)
        .await
        .unwrap();

    // Verify update
    let stored: String = conn.get(&rule_key).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stored).unwrap();

    assert_eq!(parsed["name"], "Rule Version 2");
    assert_eq!(parsed["rule"]["action"], "block");
    assert_eq!(parsed["rule"]["message"], "Version 2 - Updated");

    // Cleanup
    let _: () = conn.del(&rule_key).await.unwrap();
}
