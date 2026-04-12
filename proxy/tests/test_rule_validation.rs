use parapet_core::rules::analyzers::*;
use parapet_core::rules::{AnalyzerRegistry, RuleEngine};
use std::path::PathBuf;
use std::sync::Arc;

fn fixture_preset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/rules/presets")
        .join(name)
}

fn create_full_registry() -> AnalyzerRegistry {
    let mut registry = AnalyzerRegistry::new();
    registry.register(Arc::new(BasicAnalyzer::new()));

    // Register CoreSecurityAnalyzer (needed for blocked_program_detected field)
    let blocklist = std::collections::HashSet::new();
    registry.register(Arc::new(CoreSecurityAnalyzer::new(blocklist)));

    registry.register(Arc::new(TokenInstructionAnalyzer::new()));
    registry.register(Arc::new(SystemProgramAnalyzer::new()));
    registry.register(Arc::new(ProgramComplexityAnalyzer::new()));

    registry
}

#[test]
fn test_comprehensive_protection_rules_valid() {
    let registry = create_full_registry();
    let mut engine = RuleEngine::new(registry);

    // Load comprehensive protection rules
    let result = engine.load_rules_from_file(
        fixture_preset("comprehensive-protection.json")
            .to_str()
            .expect("utf8 path"),
    );

    assert!(
        result.is_ok(),
        "Failed to load comprehensive-protection.json: {:?}",
        result.err()
    );
    assert!(engine.enabled_rule_count() > 0, "No rules were loaded");

    println!(
        "✅ Loaded {} rules from comprehensive-protection.json",
        engine.enabled_rule_count()
    );
}

#[test]
fn test_trading_bot_protection_rules_valid() {
    let registry = create_full_registry();
    let mut engine = RuleEngine::new(registry);

    // Load trading bot protection rules
    let result = engine.load_rules_from_file(
        fixture_preset("trading-bot-protection.json")
            .to_str()
            .expect("utf8 path"),
    );

    assert!(
        result.is_ok(),
        "Failed to load trading-bot-protection.json: {:?}",
        result.err()
    );
    assert!(engine.enabled_rule_count() > 0, "No rules were loaded");

    println!(
        "✅ Loaded {} rules from trading-bot-protection.json",
        engine.enabled_rule_count()
    );
}

#[test]
fn test_enhanced_security_rules_valid() {
    let registry = create_full_registry();
    let mut engine = RuleEngine::new(registry);

    // Load enhanced security rules
    let result = engine.load_rules_from_file(
        fixture_preset("enhanced-security.json")
            .to_str()
            .expect("utf8 path"),
    );

    assert!(
        result.is_ok(),
        "Failed to load enhanced-security.json: {:?}",
        result.err()
    );
    assert!(engine.enabled_rule_count() > 0, "No rules were loaded");

    println!(
        "✅ Loaded {} rules from enhanced-security.json",
        engine.enabled_rule_count()
    );
}

#[test]
fn test_all_critical_fields_available() {
    let registry = create_full_registry();

    // These are the most critical fields for security rules
    let critical_fields = vec![
        // Delegation attacks
        "token_instructions:unlimited_approve_count",
        "token_instructions:net_delegation_change",
        "token_instructions:has_revoke",
        // Freeze attacks
        "token_instructions:has_freeze",
        "token_instructions:dangerous_operation_combo",
        // Burn attacks
        "token_instructions:has_burn",
        // SOL drains
        "system:max_sol_transfer",
        "system:large_sol_transfer",
        // Account spam
        "system:account_creation_count",
        "system:high_rent_spam",
        // Obfuscation
        "complexity:complexity_score",
        "complexity:non_core_program_count",
        // Account confusion
        "complexity:writable_non_signer_count",
    ];

    for field in &critical_fields {
        assert!(
            registry.has_field(field),
            "Critical security field '{}' not available",
            field
        );
    }

    println!(
        "✅ All {} critical security fields available",
        critical_fields.len()
    );
}
