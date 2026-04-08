#[cfg(test)]
mod field_coverage_test {
    use crate::rules::analyzers::core::*;
    use crate::rules::AnalyzerRegistry;
    use std::sync::Arc;

    #[test]
    fn test_all_analyzers_registered() {
        let mut registry = AnalyzerRegistry::new();

        // Register all analyzers
        registry.register(Arc::new(BasicAnalyzer::new()));
        registry.register(Arc::new(TokenInstructionAnalyzer::new()));
        registry.register(Arc::new(SystemProgramAnalyzer::new()));
        registry.register(Arc::new(ProgramComplexityAnalyzer::new()));

        let all_fields = registry.get_all_fields();

        // Should have 4 analyzers
        assert_eq!(all_fields.len(), 4);

        // Verify each analyzer
        assert!(all_fields.contains_key("basic"));
        assert!(all_fields.contains_key("token_instructions"));
        assert!(all_fields.contains_key("system"));
        assert!(all_fields.contains_key("complexity"));

        // Print field counts for documentation
        println!("\n📊 Analyzer Field Coverage:");
        let mut total_fields = 0;
        for (name, fields) in &all_fields {
            println!("  • {}: {} fields", name, fields.len());
            total_fields += fields.len();
        }
        println!("  ━━━━━━━━━━━━━━━━━━━━━━");
        println!("  Total: {} fields\n", total_fields);

        // Verify minimum field counts
        assert!(all_fields.get("basic").unwrap().len() >= 7);
        assert!(all_fields.get("token_instructions").unwrap().len() >= 30);
        assert!(all_fields.get("system").unwrap().len() >= 12);
        assert!(all_fields.get("complexity").unwrap().len() >= 12);
    }

    #[test]
    fn test_critical_fields_available() {
        let mut registry = AnalyzerRegistry::new();
        registry.register(Arc::new(TokenInstructionAnalyzer::new()));
        registry.register(Arc::new(SystemProgramAnalyzer::new()));

        // Critical fields for security rules
        let critical_fields = vec![
            "token_instructions:unlimited_approve_count",
            "token_instructions:has_freeze",
            "token_instructions:has_burn",
            "token_instructions:net_delegation_change",
            "token_instructions:has_revoke",
            "system:max_sol_transfer",
            "system:account_creation_count",
            "system:large_sol_transfer",
        ];

        for field in critical_fields {
            assert!(
                registry.has_field(field),
                "Critical field '{}' not available",
                field
            );
        }
    }
}
