#[cfg(test)]
mod tests {
    use crate::rules::analyzer::TransactionAnalyzer;
    use crate::rules::analyzers::core::{program_complexity, system_program, token_instructions};
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        message::Message,
        pubkey::Pubkey,
        signature::Keypair,
        signer::Signer,
        transaction::Transaction,
    };
    use solana_system_interface::instruction as system_instruction;
    use std::str::FromStr;

    // SPL Token instruction discriminators
    const CLOSE_ACCOUNT: u8 = 9;
    const TRANSFER_CHECKED: u8 = 12;

    fn create_test_keypair() -> Keypair {
        Keypair::new()
    }

    fn create_token_approve_instruction(
        token_program_id: &Pubkey,
        approve_amount: u64,
    ) -> solana_sdk::instruction::Instruction {
        // SPL Token Approve instruction
        let mut data = vec![4u8]; // APPROVE discriminator
        data.extend_from_slice(&approve_amount.to_le_bytes());

        solana_sdk::instruction::Instruction {
            program_id: *token_program_id,
            accounts: vec![],
            data,
        }
    }

    fn create_token_revoke_instruction(
        token_program_id: &Pubkey,
    ) -> solana_sdk::instruction::Instruction {
        let data = vec![5u8]; // REVOKE discriminator

        solana_sdk::instruction::Instruction {
            program_id: *token_program_id,
            accounts: vec![],
            data,
        }
    }

    fn create_token_freeze_instruction(
        token_program_id: &Pubkey,
    ) -> solana_sdk::instruction::Instruction {
        let data = vec![10u8]; // FREEZE_ACCOUNT discriminator

        solana_sdk::instruction::Instruction {
            program_id: *token_program_id,
            accounts: vec![],
            data,
        }
    }

    fn create_token_burn_instruction(
        token_program_id: &Pubkey,
        burn_amount: u64,
    ) -> solana_sdk::instruction::Instruction {
        let mut data = vec![8u8]; // BURN discriminator
        data.extend_from_slice(&burn_amount.to_le_bytes());

        solana_sdk::instruction::Instruction {
            program_id: *token_program_id,
            accounts: vec![],
            data,
        }
    }

    #[tokio::test]
    async fn test_token_instruction_analyzer_unlimited_approve() {
        let analyzer = token_instructions::TokenInstructionAnalyzer::new();
        let payer = create_test_keypair();

        let token_program =
            Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
        let approve_instruction = create_token_approve_instruction(&token_program, u64::MAX);

        let message = Message::new(&[approve_instruction], Some(&payer.pubkey()));
        let tx = Transaction::new_unsigned(message);

        let fields = analyzer.analyze(&tx).await.unwrap();

        assert_eq!(fields.get("has_approve").unwrap(), &serde_json::json!(true));
        assert_eq!(fields.get("approve_count").unwrap(), &serde_json::json!(1));
        assert_eq!(
            fields.get("unlimited_approve_count").unwrap(),
            &serde_json::json!(1)
        );
        assert_eq!(
            fields.get("net_delegation_change").unwrap(),
            &serde_json::json!(1)
        );
    }

    #[tokio::test]
    async fn test_token_instruction_analyzer_revoke() {
        let analyzer = token_instructions::TokenInstructionAnalyzer::new();
        let payer = create_test_keypair();

        let token_program =
            Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
        let revoke_instruction = create_token_revoke_instruction(&token_program);

        let message = Message::new(&[revoke_instruction], Some(&payer.pubkey()));
        let tx = Transaction::new_unsigned(message);

        let fields = analyzer.analyze(&tx).await.unwrap();

        assert_eq!(fields.get("has_revoke").unwrap(), &serde_json::json!(true));
        assert_eq!(fields.get("revoke_count").unwrap(), &serde_json::json!(1));
        assert_eq!(
            fields.get("net_delegation_change").unwrap(),
            &serde_json::json!(-1)
        );
    }

    #[tokio::test]
    async fn test_token_instruction_analyzer_dangerous_combo() {
        let analyzer = token_instructions::TokenInstructionAnalyzer::new();
        let payer = create_test_keypair();

        let token_program =
            Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();

        // Freeze + Approve combo
        let freeze_instruction = create_token_freeze_instruction(&token_program);
        let approve_instruction = create_token_approve_instruction(&token_program, 1000);

        let message = Message::new(
            &[freeze_instruction, approve_instruction],
            Some(&payer.pubkey()),
        );
        let tx = Transaction::new_unsigned(message);

        let fields = analyzer.analyze(&tx).await.unwrap();

        assert_eq!(fields.get("has_freeze").unwrap(), &serde_json::json!(true));
        assert_eq!(fields.get("has_approve").unwrap(), &serde_json::json!(true));
        assert_eq!(
            fields.get("dangerous_operation_combo").unwrap(),
            &serde_json::json!(true)
        );
    }

    #[tokio::test]
    async fn test_token_instruction_analyzer_burn_detection() {
        let analyzer = token_instructions::TokenInstructionAnalyzer::new();
        let payer = create_test_keypair();

        let token_program =
            Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
        let burn_instruction = create_token_burn_instruction(&token_program, 5000);

        let message = Message::new(&[burn_instruction], Some(&payer.pubkey()));
        let tx = Transaction::new_unsigned(message);

        let fields = analyzer.analyze(&tx).await.unwrap();

        assert_eq!(fields.get("has_burn").unwrap(), &serde_json::json!(true));
        assert_eq!(fields.get("burn_count").unwrap(), &serde_json::json!(1));
        assert_eq!(
            fields.get("total_burn_amount").unwrap(),
            &serde_json::json!(5000)
        );
    }

    #[tokio::test]
    async fn test_system_program_analyzer_sol_transfer() {
        let analyzer = system_program::SystemProgramAnalyzer::new();
        let payer = create_test_keypair();
        let recipient = Pubkey::new_unique();

        let transfer = system_instruction::transfer(&payer.pubkey(), &recipient, 2_000_000_000); // 2 SOL
        let message = Message::new(&[transfer], Some(&payer.pubkey()));
        let tx = Transaction::new_unsigned(message);

        let fields = analyzer.analyze(&tx).await.unwrap();

        assert_eq!(
            fields.get("has_sol_transfer").unwrap(),
            &serde_json::json!(true)
        );
        assert_eq!(
            fields.get("sol_transfer_count").unwrap(),
            &serde_json::json!(1)
        );
        assert_eq!(
            fields.get("max_sol_transfer").unwrap(),
            &serde_json::json!(2_000_000_000u64)
        );
        assert_eq!(
            fields.get("large_sol_transfer").unwrap(),
            &serde_json::json!(true)
        );
    }

    #[tokio::test]
    async fn test_system_program_analyzer_account_creation() {
        let analyzer = system_program::SystemProgramAnalyzer::new();
        let payer = create_test_keypair();

        // Create account instruction with 1000 lamports rent
        let mut data = vec![0u8, 0, 0, 0]; // CREATE_ACCOUNT discriminator
        data.extend_from_slice(&1000u64.to_le_bytes()); // lamports
        data.extend_from_slice(&100u64.to_le_bytes()); // space
        data.extend_from_slice(&Pubkey::new_unique().to_bytes()); // owner

        let instruction = solana_sdk::instruction::Instruction {
            program_id: Pubkey::from_str("11111111111111111111111111111111").unwrap(),
            accounts: vec![],
            data,
        };

        let message = Message::new(&[instruction], Some(&payer.pubkey()));
        let tx = Transaction::new_unsigned(message);

        let fields = analyzer.analyze(&tx).await.unwrap();

        assert_eq!(
            fields.get("creates_accounts").unwrap(),
            &serde_json::json!(true)
        );
        assert_eq!(
            fields.get("account_creation_count").unwrap(),
            &serde_json::json!(1)
        );
    }

    #[tokio::test]
    async fn test_complexity_analyzer_non_core_programs() {
        let analyzer = program_complexity::ProgramComplexityAnalyzer::new();
        let payer = create_test_keypair();

        // Create instruction with non-core program
        let non_core_program = Pubkey::new_unique();
        let instruction = solana_sdk::instruction::Instruction {
            program_id: non_core_program,
            accounts: vec![],
            data: vec![],
        };

        let message = Message::new(&[instruction], Some(&payer.pubkey()));
        let tx = Transaction::new_unsigned(message);

        let fields = analyzer.analyze(&tx).await.unwrap();

        assert_eq!(
            fields.get("non_core_program_count").unwrap(),
            &serde_json::json!(1)
        );
        assert!(
            fields
                .get("non_core_programs")
                .unwrap()
                .as_array()
                .unwrap()
                .len()
                > 0
        );
    }

    #[tokio::test]
    async fn test_complexity_analyzer_core_programs() {
        let analyzer = program_complexity::ProgramComplexityAnalyzer::new();
        let payer = create_test_keypair();

        // System program (core)
        let transfer = system_instruction::transfer(&payer.pubkey(), &Pubkey::new_unique(), 1000);

        let message = Message::new(&[transfer], Some(&payer.pubkey()));
        let tx = Transaction::new_unsigned(message);

        let fields = analyzer.analyze(&tx).await.unwrap();

        assert_eq!(
            fields.get("uses_system_program").unwrap(),
            &serde_json::json!(true)
        );
        assert_eq!(
            fields.get("core_program_count").unwrap(),
            &serde_json::json!(1)
        );
        assert_eq!(
            fields.get("non_core_program_count").unwrap(),
            &serde_json::json!(0)
        );
    }

    #[tokio::test]
    async fn test_approve_then_revoke_net_zero() {
        let analyzer = token_instructions::TokenInstructionAnalyzer::new();
        let payer = create_test_keypair();

        let token_program =
            Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
        let approve_instruction = create_token_approve_instruction(&token_program, 1000);
        let revoke_instruction = create_token_revoke_instruction(&token_program);

        let message = Message::new(
            &[approve_instruction, revoke_instruction],
            Some(&payer.pubkey()),
        );
        let tx = Transaction::new_unsigned(message);

        let fields = analyzer.analyze(&tx).await.unwrap();

        // Should have both operations
        assert_eq!(fields.get("has_approve").unwrap(), &serde_json::json!(true));
        assert_eq!(fields.get("has_revoke").unwrap(), &serde_json::json!(true));

        // Net change should be zero (1 approve - 1 revoke = 0)
        assert_eq!(
            fields.get("net_delegation_change").unwrap(),
            &serde_json::json!(0)
        );
    }

    #[tokio::test]
    async fn test_ownership_detection_non_owned_close() {
        let analyzer = token_instructions::TokenInstructionAnalyzer::new();
        let payer = create_test_keypair();
        let non_signer = Pubkey::new_unique(); // Different from payer

        let token_program =
            Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();

        // CloseAccount: [account, destination, authority]
        // Authority is NOT a signer (non_signer)
        let close_ix = Instruction {
            program_id: token_program,
            accounts: vec![
                AccountMeta::new(Pubkey::new_unique(), false),
                AccountMeta::new(Pubkey::new_unique(), false),
                AccountMeta::new_readonly(non_signer, false), // NOT a signer
            ],
            data: vec![CLOSE_ACCOUNT], // discriminator 9
        };

        let message = Message::new(&[close_ix], Some(&payer.pubkey()));
        let tx = Transaction::new_unsigned(message);

        let fields = analyzer.analyze(&tx).await.unwrap();

        // Should detect non-owned close
        assert_eq!(
            fields.get("closes_non_owned_account").unwrap(),
            &serde_json::json!(true)
        );
        assert_eq!(
            fields
                .get("non_owned_operations_count")
                .unwrap()
                .as_u64()
                .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn test_ownership_detection_owned_close() {
        let analyzer = token_instructions::TokenInstructionAnalyzer::new();
        let owner = create_test_keypair();

        let token_program =
            Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();

        // CloseAccount: [account, destination, authority]
        // Authority IS the payer (index 0 in account_keys)
        let close_ix = Instruction {
            program_id: token_program,
            accounts: vec![
                AccountMeta::new(Pubkey::new_unique(), false),
                AccountMeta::new(Pubkey::new_unique(), false),
                AccountMeta::new_readonly(owner.pubkey(), true), // IS a signer
            ],
            data: vec![CLOSE_ACCOUNT],
        };

        let message = Message::new(&[close_ix], Some(&owner.pubkey()));
        let tx = Transaction::new_unsigned(message);

        let fields = analyzer.analyze(&tx).await.unwrap();

        // Should NOT detect non-owned close
        assert_eq!(
            fields.get("closes_non_owned_account").unwrap(),
            &serde_json::json!(false)
        );
        assert_eq!(
            fields
                .get("non_owned_operations_count")
                .unwrap()
                .as_u64()
                .unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn test_mint_extraction_from_transfer_checked() {
        let analyzer = token_instructions::TokenInstructionAnalyzer::new();
        let payer = create_test_keypair();

        let token_program =
            Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
        let mint1 = Pubkey::new_unique();
        let mint2 = Pubkey::new_unique();

        // TransferChecked: [source, mint, destination, owner]
        let transfer1 = Instruction {
            program_id: token_program,
            accounts: vec![
                AccountMeta::new(Pubkey::new_unique(), false),
                AccountMeta::new(Pubkey::new_unique(), false),
                AccountMeta::new_readonly(mint1, false), // Mint address
                AccountMeta::new_readonly(payer.pubkey(), true),
            ],
            data: vec![TRANSFER_CHECKED, 100, 0, 0, 0, 0, 0, 0, 0, 6], // amount + decimals
        };

        let transfer2 = Instruction {
            program_id: token_program,
            accounts: vec![
                AccountMeta::new(Pubkey::new_unique(), false),
                AccountMeta::new(Pubkey::new_unique(), false),
                AccountMeta::new_readonly(mint2, false), // Different mint
                AccountMeta::new_readonly(payer.pubkey(), true),
            ],
            data: vec![TRANSFER_CHECKED, 50, 0, 0, 0, 0, 0, 0, 0, 9],
        };

        let message = Message::new(&[transfer1, transfer2], Some(&payer.pubkey()));
        let tx = Transaction::new_unsigned(message);

        let fields = analyzer.analyze(&tx).await.unwrap();

        // Should extract both mints
        let mints = fields.get("mints_involved").unwrap().as_array().unwrap();
        assert_eq!(mints.len(), 2);
        assert!(mints.contains(&serde_json::json!(mint1.to_string())));
        assert!(mints.contains(&serde_json::json!(mint2.to_string())));

        // Should count TransferChecked usage
        assert_eq!(
            fields.get("transfer_checked_count").unwrap(),
            &serde_json::json!(2)
        );
        assert_eq!(
            fields.get("uses_transfer_checked").unwrap(),
            &serde_json::json!(true)
        );
    }
}
