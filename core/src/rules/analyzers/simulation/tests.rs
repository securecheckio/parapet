#[cfg(test)]
mod simulation_tests {
    use super::super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_simulation_balance_analyzer() {
        let analyzer = SimulationBalanceAnalyzer::new();

        // Test data: fee payer loses 0.5 SOL
        let simulation_result = json!({
            "preBalances": [5000000000u64, 2000000000u64],  // 5 SOL, 2 SOL
            "postBalances": [4500000000u64, 2000000000u64]  // 4.5 SOL, 2 SOL
        });

        let fields = analyzer.analyze(&simulation_result).await.unwrap();

        assert_eq!(
            fields.get("sol_balance_change").unwrap().as_f64().unwrap(),
            -0.5
        );
        assert_eq!(
            fields.get("total_sol_outflow").unwrap().as_f64().unwrap(),
            0.5
        );
        assert_eq!(
            fields
                .get("accounts_losing_balance")
                .unwrap()
                .as_u64()
                .unwrap(),
            1
        );
        assert_eq!(
            fields
                .get("largest_balance_decrease")
                .unwrap()
                .as_f64()
                .unwrap(),
            0.5
        );
    }

    #[tokio::test]
    async fn test_simulation_balance_analyzer_percentage() {
        let analyzer = SimulationBalanceAnalyzer::new();

        // Test: 50% balance loss
        let simulation_result = json!({
            "preBalances": [2000000000u64],  // 2 SOL
            "postBalances": [1000000000u64]  // 1 SOL
        });

        let fields = analyzer.analyze(&simulation_result).await.unwrap();

        let percent = fields
            .get("sol_balance_change_percent")
            .unwrap()
            .as_f64()
            .unwrap();
        assert!((percent - (-50.0)).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_simulation_token_analyzer() {
        let analyzer = SimulationTokenAnalyzer::new();

        let simulation_result = json!({
            "preTokenBalances": [
                {
                    "accountIndex": 1,
                    "mint": "TokenMintABC123",
                    "uiTokenAmount": {
                        "amount": "1000000",
                        "decimals": 6,
                        "uiAmount": 1.0
                    }
                }
            ],
            "postTokenBalances": [
                {
                    "accountIndex": 1,
                    "mint": "TokenMintABC123",
                    "uiTokenAmount": {
                        "amount": "0",
                        "decimals": 6,
                        "uiAmount": 0.0
                    }
                }
            ]
        });

        let fields = analyzer.analyze(&simulation_result).await.unwrap();

        assert_eq!(
            fields.get("token_transfers_out").unwrap().as_u64().unwrap(),
            1
        );
        assert_eq!(
            fields.get("token_transfers_in").unwrap().as_u64().unwrap(),
            0
        );
        assert_eq!(
            fields
                .get("tokens_fully_drained")
                .unwrap()
                .as_u64()
                .unwrap(),
            1
        );
        assert_eq!(
            fields.get("net_token_changes").unwrap().as_i64().unwrap(),
            -1
        );
    }

    #[tokio::test]
    async fn test_simulation_token_analyzer_nft_detection() {
        let analyzer = SimulationTokenAnalyzer::new();

        // NFT: decimals=0, amount=1
        let simulation_result = json!({
            "preTokenBalances": [
                {
                    "accountIndex": 1,
                    "mint": "NFTMintXYZ789",
                    "uiTokenAmount": {
                        "amount": "1",
                        "decimals": 0,
                        "uiAmount": 1.0
                    }
                }
            ],
            "postTokenBalances": []
        });

        let fields = analyzer.analyze(&simulation_result).await.unwrap();

        let nft_transfers = fields.get("nft_transfers").unwrap().as_array().unwrap();
        assert_eq!(nft_transfers.len(), 1);
        assert_eq!(nft_transfers[0].as_str().unwrap(), "NFTMintXYZ789");
    }

    #[tokio::test]
    async fn test_simulation_log_analyzer() {
        let analyzer = SimulationLogAnalyzer::new();

        let simulation_result = json!({
            "logs": [
                "Program 11111111111111111111111111111111 invoke [1]",
                "Program log: Starting token transfer",
                "Program log: Warning: Suspicious activity detected",
                "Program log: Error: Unauthorized access attempt",
                "Program 11111111111111111111111111111111 success"
            ]
        });

        let fields = analyzer.analyze(&simulation_result).await.unwrap();

        assert_eq!(fields.get("log_count").unwrap().as_u64().unwrap(), 5);
        assert!(fields.get("has_error_logs").unwrap().as_bool().unwrap());

        let error_messages = fields.get("error_messages").unwrap().as_array().unwrap();
        assert!(!error_messages.is_empty());
    }

    #[tokio::test]
    async fn test_simulation_log_analyzer_suspicious_keywords() {
        let analyzer = SimulationLogAnalyzer::new();

        let simulation_result = json!({
            "logs": [
                "Program log: Attempting to drain all funds",
                "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA success"
            ]
        });

        let fields = analyzer.analyze(&simulation_result).await.unwrap();

        assert!(fields
            .get("suspicious_keywords")
            .unwrap()
            .as_bool()
            .unwrap());

        let error_messages = fields.get("error_messages").unwrap().as_array().unwrap();
        assert!(error_messages
            .iter()
            .any(|msg| msg.as_str().unwrap().contains("drain")));
    }

    #[tokio::test]
    async fn test_simulation_cpi_analyzer() {
        let analyzer = SimulationCpiAnalyzer::new();

        let simulation_result = json!({
            "innerInstructions": [
                {
                    "index": 0,
                    "instructions": [
                        {
                            "programIdIndex": 5,
                            "accounts": [0, 1, 2],
                            "data": "3Bxs4h24hBtQy9rw"
                        },
                        {
                            "programIdIndex": 7,
                            "accounts": [3, 4],
                            "data": "AxB3c"
                        }
                    ]
                }
            ]
        });

        let fields = analyzer.analyze(&simulation_result).await.unwrap();

        assert!(fields.get("has_cpi_calls").unwrap().as_bool().unwrap());
        assert_eq!(
            fields
                .get("cpi_instruction_count")
                .unwrap()
                .as_u64()
                .unwrap(),
            1
        );
        assert_eq!(fields.get("cpi_depth").unwrap().as_u64().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_simulation_failure_analyzer() {
        let analyzer = SimulationFailureAnalyzer::new();

        let simulation_result = json!({
            "err": {
                "InstructionError": [0, "InvalidAccountData"]
            },
            "logs": [
                "Program ABC123 invoke [1]",
                "Program log: Error: Invalid data",
                "Program ABC123 failed: InvalidAccountData"
            ]
        });

        let fields = analyzer.analyze(&simulation_result).await.unwrap();

        assert!(fields.get("simulation_failed").unwrap().as_bool().unwrap());
        assert!(fields
            .get("has_simulation_error")
            .unwrap()
            .as_bool()
            .unwrap());

        let error_msg = fields.get("simulation_error").unwrap().as_str().unwrap();
        assert!(error_msg.contains("InstructionError"));
    }

    #[tokio::test]
    async fn test_simulation_failure_analyzer_partial_failure() {
        let analyzer = SimulationFailureAnalyzer::new();

        let simulation_result = json!({
            "err": null,
            "logs": [
                "Program ABC123 invoke [1]",
                "Program ABC123 success",
                "Program DEF456 invoke [1]",
                "Program log: Error: Something failed",
                "Program DEF456 failed"
            ]
        });

        let fields = analyzer.analyze(&simulation_result).await.unwrap();

        assert!(fields.get("partial_failure").unwrap().as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_simulation_compute_analyzer() {
        let analyzer = SimulationComputeAnalyzer::new();

        let simulation_result = json!({
            "unitsConsumed": 150000u64,
            "logs": [
                "Program ABC123 invoke [1]",
                "Program ABC123 consumed 50000 compute units",
                "Program ABC123 success"
            ]
        });

        let fields = analyzer.analyze(&simulation_result).await.unwrap();

        assert_eq!(
            fields.get("compute_units_used").unwrap().as_u64().unwrap(),
            150000
        );
        let usage_percent = fields
            .get("compute_usage_percent")
            .unwrap()
            .as_f64()
            .unwrap();
        assert!((usage_percent - 75.0).abs() < 0.1); // 150k / 200k = 75%
    }

    #[tokio::test]
    async fn test_simulation_compute_analyzer_excessive() {
        let analyzer = SimulationComputeAnalyzer::new();

        // Single instruction consuming 500k CU is excessive
        let simulation_result = json!({
            "unitsConsumed": 500000u64,
            "logs": [
                "Program ABC123 invoke [1]",
                "Program ABC123 success"
            ]
        });

        let fields = analyzer.analyze(&simulation_result).await.unwrap();

        assert!(fields.get("excessive_compute").unwrap().as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_simulation_registry() {
        let mut registry = SimulationAnalyzerRegistry::new();

        registry.register(Box::new(SimulationBalanceAnalyzer::new()));
        registry.register(Box::new(SimulationTokenAnalyzer::new()));
        registry.register(Box::new(SimulationLogAnalyzer::new()));

        let simulation_result = json!({
            "preBalances": [5000000000u64],
            "postBalances": [4500000000u64],
            "logs": [
                "Program ABC123 invoke [1]",
                "Program ABC123 success"
            ]
        });

        let fields = registry.analyze_all(&simulation_result).await.unwrap();

        // Check that fields from multiple analyzers are present
        assert!(fields.contains_key("sol_balance_change"));
        assert!(fields.contains_key("log_count"));
        assert!(fields.contains_key("token_transfers_out"));
    }
}
