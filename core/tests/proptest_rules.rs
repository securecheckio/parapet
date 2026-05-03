//! Property tests: parsing and small invariants must not panic on arbitrary input.

use parapet_core::rules::types::RuleDefinition;
use proptest::prelude::*;
use serde_json::json;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn serde_json_from_random_utf8_lossy_does_not_panic(bytes in prop::collection::vec(any::<u8>(), 0..1024)) {
        let text = String::from_utf8_lossy(&bytes);
        let _: Result<serde_json::Value, _> = serde_json::from_str(&text);
    }

    /// RuleDefinition JSON with arbitrary field name: parse is fallible but must not panic.
    #[test]
    fn rule_definition_parse_bounded_field_never_panics(
        field in "[a-zA-Z0-9_:]{1,48}",
        message in prop::collection::vec(32u8..126u8, 0..80)
    ) {
        let msg = String::from_utf8_lossy(&message).into_owned();
        let raw = json!({
            "version": "1",
            "id": "prop-test",
            "name": "property test",
            "enabled": true,
            "rule": {
                "action": "pass",
                "conditions": {
                    "field": field,
                    "operator": "equals",
                    "value": 1
                },
                "message": msg
            }
        });
        let text = raw.to_string();
        let _: Result<RuleDefinition, _> = serde_json::from_str(&text);
    }

    /// Nested `all` / `any` compound conditions: bounded depth, no panic on parse.
    #[test]
    fn rule_definition_parse_nested_compound_never_panics(
        depth in 0usize..6
    ) {
        let mut cond = json!({
            "field": "basic:instruction_count",
            "operator": "greater_than",
            "value": 0
        });
        for _ in 0..depth {
            cond = json!({ "all": [cond] });
        }
        let raw = json!({
            "version": "1",
            "id": "nested",
            "name": "nested compound",
            "enabled": true,
            "rule": {
                "action": "alert",
                "conditions": cond,
                "message": "nested"
            }
        });
        let text = raw.to_string();
        let _: Result<RuleDefinition, _> = serde_json::from_str(&text);
    }
}
