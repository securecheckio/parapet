use parapet_core::rules::analyzers::simulation::{
    SimulationAnalyzerRegistry, SimulationBalanceAnalyzer, SimulationLogAnalyzer,
    SimulationTokenAnalyzer,
};
use serde_json::json;

#[tokio::test]
async fn test_simulation_enrichment_structure() {
    // This test validates the structure of the solShield response field

    let simulation_response = json!({
        "preBalances": [5000000000u64, 2000000000u64],
        "postBalances": [4000000000u64, 2000000000u64],
        "logs": [
            "Program ABC123 invoke [1]",
            "Program log: Transfer complete",
            "Program ABC123 success"
        ],
        "unitsConsumed": 50000u64,
        "err": null
    });

    let mut registry = SimulationAnalyzerRegistry::new();
    registry.register(Box::new(SimulationBalanceAnalyzer::new()));
    registry.register(Box::new(SimulationTokenAnalyzer::new()));
    registry.register(Box::new(SimulationLogAnalyzer::new()));

    let fields = registry.analyze_all(&simulation_response).await.unwrap();

    // Verify expected fields are present
    assert!(fields.contains_key("sol_balance_change"));
    assert!(fields.contains_key("log_count"));
    assert!(fields.contains_key("token_transfers_out"));

    // Verify balance change calculation
    let balance_change = fields.get("sol_balance_change").unwrap().as_f64().unwrap();
    assert_eq!(balance_change, -1.0); // Lost 1 SOL
}

#[tokio::test]
async fn test_simulation_high_risk_detection() {
    // Test detecting high-risk patterns in simulation

    let simulation_response = json!({
        "preBalances": [5000000000u64],
        "postBalances": [500000000u64],  // 90% loss
        "preTokenBalances": [],
        "postTokenBalances": [],
        "logs": [
            "Program DRAINxxx invoke [1]",
            "Program log: drain initiated",
            "Program DRAINxxx success"
        ],
        "unitsConsumed": 50000u64,
        "err": null
    });

    let mut registry = SimulationAnalyzerRegistry::new();
    registry.register(Box::new(SimulationBalanceAnalyzer::new()));
    registry.register(Box::new(SimulationLogAnalyzer::new()));
    registry.register(Box::new(SimulationTokenAnalyzer::new()));

    let fields = registry.analyze_all(&simulation_response).await.unwrap();

    // Should detect large balance loss
    let balance_change = fields.get("sol_balance_change").unwrap().as_f64().unwrap();
    assert!(balance_change < -4.0);

    // Should detect suspicious keywords
    let suspicious = fields
        .get("suspicious_keywords")
        .unwrap()
        .as_bool()
        .unwrap();
    assert_eq!(suspicious, true);

    // Should detect no token inflows (pure drain)
    let tokens_in = fields.get("token_transfers_in").unwrap().as_u64().unwrap();
    assert_eq!(tokens_in, 0);
}

#[tokio::test]
async fn test_simulation_legitimate_swap() {
    // Test that legitimate DeFi operations don't trigger false positives

    let simulation_response = json!({
        "preBalances": [5000000000u64],
        "postBalances": [4999995000u64],  // Small fee
        "preTokenBalances": [
            {
                "accountIndex": 1,
                "mint": "SOLMint",
                "uiTokenAmount": {
                    "amount": "1000000000",
                    "decimals": 9,
                    "uiAmount": 1.0
                }
            }
        ],
        "postTokenBalances": [
            {
                "accountIndex": 1,
                "mint": "SOLMint",
                "uiTokenAmount": {
                    "amount": "0",
                    "decimals": 9,
                    "uiAmount": 0.0
                }
            },
            {
                "accountIndex": 2,
                "mint": "USDCMint",
                "uiTokenAmount": {
                    "amount": "35000000",
                    "decimals": 6,
                    "uiAmount": 35.0
                }
            }
        ],
        "logs": [
            "Program JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB invoke [1]",
            "Program log: Swap: 1 SOL -> 35 USDC",
            "Program JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB success"
        ],
        "unitsConsumed": 75000u64,
        "err": null
    });

    let mut registry = SimulationAnalyzerRegistry::new();
    registry.register(Box::new(SimulationBalanceAnalyzer::new()));
    registry.register(Box::new(SimulationTokenAnalyzer::new()));
    registry.register(Box::new(SimulationLogAnalyzer::new()));

    let fields = registry.analyze_all(&simulation_response).await.unwrap();

    // Small SOL loss (fees)
    let balance_change = fields.get("sol_balance_change").unwrap().as_f64().unwrap();
    assert!(balance_change > -0.01); // Less than 0.01 SOL lost

    // Balanced token flow (1 out, 1 in)
    let tokens_out = fields.get("token_transfers_out").unwrap().as_u64().unwrap();
    let tokens_in = fields.get("token_transfers_in").unwrap().as_u64().unwrap();
    assert_eq!(tokens_out, 1);
    assert_eq!(tokens_in, 1);

    // No suspicious keywords
    let suspicious = fields
        .get("suspicious_keywords")
        .unwrap()
        .as_bool()
        .unwrap();
    assert_eq!(suspicious, false);
}

#[tokio::test]
async fn test_parapet_metadata_format() {
    // Test the format of the solShield metadata that gets injected
    use parapet_core::rules::types::{MatchedRule, RuleAction, RuleDecision};

    let decision = RuleDecision {
        action: RuleAction::Alert,
        rule_id: "test-rule".to_string(),
        rule_name: "Test Rule".to_string(),
        message: "Test message".to_string(),
        matched: true,
        total_risk: 75,
        matched_rules: vec![MatchedRule {
            rule_id: "rule-1".to_string(),
            rule_name: "Rule 1".to_string(),
            action: RuleAction::Alert,
            weight: 50,
            message: "High risk detected".to_string(),
        }],
        structural_risk: Some(40),
        simulation_risk: Some(35),
        is_simulation: true,
    };

    let threshold = 70u8;

    // Build metadata (this mimics the build_parapet_metadata function)
    let decision_label = if decision.total_risk >= threshold {
        "would_block"
    } else {
        "safe"
    };

    assert_eq!(decision_label, "would_block");
    assert_eq!(decision.total_risk, 75);
    assert_eq!(decision.structural_risk.unwrap(), 40);
    assert_eq!(decision.simulation_risk.unwrap(), 35);
    assert_eq!(decision.is_simulation, true);
}
