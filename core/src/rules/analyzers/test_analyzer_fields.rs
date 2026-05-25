#[cfg(test)]
mod field_coverage_test {
    use crate::rules::analyzers::core::*;
    use crate::rules::analyzers::simulation::{
        SimulationAnalyzerRegistry, SimulationBalanceAnalyzer, SimulationComputeAnalyzer,
        SimulationCpiAnalyzer, SimulationFailureAnalyzer, SimulationLogAnalyzer,
        SimulationTokenAnalyzer,
    };
    use crate::rules::analyzers::third_party::SquadsV4Analyzer;
    use crate::rules::AnalyzerRegistry;
    use serde::Serialize;
    use serde_json::json;
    use std::collections::HashSet;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;

    #[derive(Serialize)]
    struct AnalyzerManifest {
        /// Structural transaction analyzers (`analyzer:field` prefix in rules).
        analyzers: Vec<AnalyzerEntry>,
        /// Simulation-response analyzers (`simulation_*:` prefix in rules).
        simulation_analyzers: Vec<AnalyzerEntry>,
    }

    #[derive(Serialize)]
    struct AnalyzerEntry {
        name: String,
        fields: Vec<String>,
    }

    fn register_default_structural(registry: &mut AnalyzerRegistry) {
        registry.register(Arc::new(BasicAnalyzer::new()));
        registry.register(Arc::new(CoreSecurityAnalyzer::new(Default::default())));
        registry.register(Arc::new(TokenInstructionAnalyzer::new()));
        registry.register(Arc::new(SystemProgramAnalyzer::new()));
        registry.register(Arc::new(ProgramComplexityAnalyzer::new()));
        registry.register(Arc::new(AccountsAnalyzer::new()));
        registry.register(Arc::new(TransactionLogAnalyzer::new()));
        registry.register(Arc::new(InstructionPaddingAnalyzer::new()));
        registry.register(Arc::new(InnerInstructionAnalyzer::new()));
        registry.register(Arc::new(
            InstructionDataAnalyzer::with_authority_fingerprints_embedded(),
        ));
        registry.register(Arc::new(CanonicalTransactionAnalyzer::new()));
        registry.register(Arc::new(SquadsV4Analyzer::new()));

        #[cfg(feature = "program-analysis")]
        if let Ok(pa) =
            crate::rules::analyzers::core::ProgramAnalyzer::with_empty_blocklists(String::new())
        {
            registry.register(Arc::new(pa));
        }
    }

    fn default_simulation_registry() -> SimulationAnalyzerRegistry {
        let mut reg = SimulationAnalyzerRegistry::new();
        reg.register(Box::new(SimulationBalanceAnalyzer::new()));
        reg.register(Box::new(SimulationComputeAnalyzer::new()));
        reg.register(Box::new(SimulationCpiAnalyzer::new()));
        reg.register(Box::new(SimulationFailureAnalyzer::new()));
        reg.register(Box::new(SimulationLogAnalyzer::new()));
        reg.register(Box::new(SimulationTokenAnalyzer::new()));
        reg
    }

    #[test]
    fn analyzers_schema_snapshot() {
        let mut registry = AnalyzerRegistry::new();
        register_default_structural(&mut registry);

        let mut analyzers: Vec<AnalyzerEntry> = registry
            .list_all()
            .into_iter()
            .map(|name| {
                let a = registry.get(&name).expect("registered");
                let mut fields = a.fields();
                fields.sort();
                AnalyzerEntry { name, fields }
            })
            .collect();
        analyzers.sort_by(|a, b| a.name.cmp(&b.name));

        let sim_reg = default_simulation_registry();
        let mut simulation_analyzers: Vec<AnalyzerEntry> = sim_reg
            .list_all()
            .into_iter()
            .map(|name| {
                let a = sim_reg.get(&name).expect("simulation analyzer");
                let mut fields = a.fields();
                fields.sort();
                AnalyzerEntry { name, fields }
            })
            .collect();
        simulation_analyzers.sort_by(|a, b| a.name.cmp(&b.name));

        let manifest = AnalyzerManifest {
            analyzers,
            simulation_analyzers,
        };

        let pretty = serde_json::to_string_pretty(&manifest).expect("serialize manifest");
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("analyzers.schema.json");

        if std::env::var("UPDATE_SCHEMA").ok().as_deref() == Some("1") {
            fs::write(&path, pretty + "\n").expect("write analyzers.schema.json");
            return;
        }

        let expected = fs::read_to_string(&path).unwrap_or_else(|_| {
            panic!(
                "Missing {}. Run: UPDATE_SCHEMA=1 cargo test -p parapet-core analyzers_schema_snapshot -- --nocapture",
                path.display()
            )
        });
        assert_eq!(
            pretty.trim_end(),
            expected.trim_end(),
            "Analyzer field manifest drift. Run with UPDATE_SCHEMA=1 to refresh {}",
            path.display()
        );
    }

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
        assert!(all_fields.contains_key("programs"));

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
        assert!(all_fields.get("basic").unwrap().len() >= 12);
        assert!(all_fields.get("token_instructions").unwrap().len() >= 30);
        assert!(all_fields.get("system").unwrap().len() >= 12);
        assert!(all_fields.get("programs").unwrap().len() >= 10);
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

    #[tokio::test]
    async fn squads_v4_non_squads_tx_emits_referenced_neutral_fields() {
        use solana_sdk::{
            hash::Hash,
            message::compiled_instruction::CompiledInstruction,
            message::{Message, MessageHeader},
            pubkey::Pubkey,
            signature::Signature,
            transaction::Transaction,
        };

        let mut registry = AnalyzerRegistry::new();
        registry.register(Arc::new(SquadsV4Analyzer::new()));

        let tx = Transaction {
            signatures: vec![Signature::default()],
            message: Message {
                header: MessageHeader {
                    num_required_signatures: 1,
                    num_readonly_signed_accounts: 0,
                    num_readonly_unsigned_accounts: 0,
                },
                account_keys: vec![Pubkey::new_unique()],
                recent_blockhash: Hash::default(),
                instructions: vec![CompiledInstruction {
                    program_id_index: 0,
                    accounts: vec![],
                    data: vec![1, 2, 3],
                }],
            },
        };

        let mut req = HashSet::new();
        req.insert("squads_v4:has_threshold_change".to_string());

        let out = registry
            .analyze_selected(&tx, &["squads_v4".to_string()], Some(&req))
            .await
            .unwrap();

        assert_eq!(
            out.get("squads_v4:is_squads_transaction"),
            Some(&json!(false))
        );
        assert_eq!(
            out.get("squads_v4:squads_instruction_count"),
            Some(&json!(0))
        );
        assert_eq!(
            out.get("squads_v4:has_threshold_change"),
            Some(&json!(false))
        );
        assert!(!out.contains_key("squads_v4:primary_operation"));
    }
}
