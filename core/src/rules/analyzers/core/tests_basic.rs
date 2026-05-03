#[cfg(test)]
mod tests {
    use super::super::basic::BasicAnalyzer;
    use crate::rules::analyzer::TransactionAnalyzer;
    use solana_sdk::{
        instruction::Instruction, message::Message, pubkey::Pubkey, signature::Keypair,
        signer::Signer, transaction::Transaction,
    };
    use solana_sdk_ids::system_program;
    use solana_system_interface::instruction as system_instruction;

    #[tokio::test]
    async fn test_basic_analyzer_instruction_count() {
        let analyzer = BasicAnalyzer::new();
        let payer = Keypair::new();

        // Transaction with 2 instructions
        let transfer1 = system_instruction::transfer(&payer.pubkey(), &Pubkey::new_unique(), 1000);
        let transfer2 = system_instruction::transfer(&payer.pubkey(), &Pubkey::new_unique(), 2000);

        let message = Message::new(&[transfer1, transfer2], Some(&payer.pubkey()));
        let tx = Transaction::new_unsigned(message);

        let fields = analyzer.analyze(&tx).await.unwrap();

        assert_eq!(
            fields.get("instruction_count").unwrap(),
            &serde_json::json!(2)
        );
        assert_eq!(
            fields.get("has_instructions").unwrap(),
            &serde_json::json!(true)
        );
    }

    #[tokio::test]
    async fn test_basic_analyzer_account_keys_count() {
        let analyzer = BasicAnalyzer::new();
        let payer = Keypair::new();
        let recipient1 = Pubkey::new_unique();
        let recipient2 = Pubkey::new_unique();

        // Transaction with multiple unique accounts
        let transfer1 = system_instruction::transfer(&payer.pubkey(), &recipient1, 1000);
        let transfer2 = system_instruction::transfer(&payer.pubkey(), &recipient2, 2000);

        let message = Message::new(&[transfer1, transfer2], Some(&payer.pubkey()));
        let tx = Transaction::new_unsigned(message);

        let fields = analyzer.analyze(&tx).await.unwrap();

        // payer, recipient1, recipient2, system_program = 4 unique accounts
        assert_eq!(
            fields.get("account_keys_count").unwrap().as_u64().unwrap(),
            4
        );
    }

    #[tokio::test]
    async fn test_basic_analyzer_signers_count() {
        let analyzer = BasicAnalyzer::new();
        let payer = Keypair::new();
        let recipient = Pubkey::new_unique();

        let transfer = system_instruction::transfer(&payer.pubkey(), &recipient, 1000);
        let message = Message::new(&[transfer], Some(&payer.pubkey()));
        let tx = Transaction::new_unsigned(message);

        let fields = analyzer.analyze(&tx).await.unwrap();

        assert_eq!(fields.get("signers_count").unwrap(), &serde_json::json!(1));
    }

    /// System transfer: payer (writable signer) + recipient (writable) + system program (readonly).
    #[tokio::test]
    async fn test_basic_analyzer_writable_accounts_count_transfer() {
        let analyzer = BasicAnalyzer::new();
        let payer = Keypair::new();
        let recipient = Pubkey::new_unique();

        let transfer = system_instruction::transfer(&payer.pubkey(), &recipient, 1000);
        let message = Message::new(&[transfer], Some(&payer.pubkey()));
        let tx = Transaction::new_unsigned(message);

        let fields = analyzer.analyze(&tx).await.unwrap();

        assert_eq!(
            fields
                .get("writable_accounts_count")
                .unwrap()
                .as_u64()
                .unwrap(),
            2
        );
    }

    #[tokio::test]
    async fn test_basic_analyzer_program_ids() {
        let analyzer = BasicAnalyzer::new();
        let payer = Keypair::new();

        let system_program = system_program::id();
        let transfer = system_instruction::transfer(&payer.pubkey(), &Pubkey::new_unique(), 1000);

        let message = Message::new(&[transfer], Some(&payer.pubkey()));
        let tx = Transaction::new_unsigned(message);

        let fields = analyzer.analyze(&tx).await.unwrap();

        let program_ids = fields.get("program_ids").unwrap().as_array().unwrap();
        assert_eq!(program_ids.len(), 1);
        assert!(program_ids.contains(&serde_json::json!(system_program.to_string())));
    }

    #[tokio::test]
    async fn test_basic_analyzer_multiple_program_ids() {
        let analyzer = BasicAnalyzer::new();
        let payer = Keypair::new();

        // System program instruction
        let transfer = system_instruction::transfer(&payer.pubkey(), &Pubkey::new_unique(), 1000);

        // Custom program instruction
        let custom_program = Pubkey::new_unique();
        let custom_ix = Instruction {
            program_id: custom_program,
            accounts: vec![],
            data: vec![],
        };

        let message = Message::new(&[transfer, custom_ix], Some(&payer.pubkey()));
        let tx = Transaction::new_unsigned(message);

        let fields = analyzer.analyze(&tx).await.unwrap();

        let program_ids = fields.get("program_ids").unwrap().as_array().unwrap();
        assert_eq!(program_ids.len(), 2);
    }

    #[tokio::test]
    async fn test_basic_analyzer_no_instructions() {
        let analyzer = BasicAnalyzer::new();
        let payer = Keypair::new();

        // Empty transaction
        let message = Message::new(&[], Some(&payer.pubkey()));
        let tx = Transaction::new_unsigned(message);

        let fields = analyzer.analyze(&tx).await.unwrap();

        assert_eq!(
            fields.get("instruction_count").unwrap(),
            &serde_json::json!(0)
        );
        assert_eq!(
            fields.get("has_instructions").unwrap(),
            &serde_json::json!(false)
        );
        assert_eq!(
            fields.get("program_ids").unwrap().as_array().unwrap().len(),
            0
        );
    }

    #[test]
    fn test_basic_analyzer_fields_list() {
        let analyzer = BasicAnalyzer::new();
        let fields = analyzer.fields();

        assert_eq!(fields.len(), 7);
        assert!(fields.contains(&"instruction_count".to_string()));
        assert!(fields.contains(&"account_keys_count".to_string()));
        assert!(fields.contains(&"writable_accounts_count".to_string()));
        assert!(fields.contains(&"signers_count".to_string()));
        assert!(fields.contains(&"amount".to_string()));
        assert!(fields.contains(&"has_instructions".to_string()));
        assert!(fields.contains(&"program_ids".to_string()));
    }

    #[test]
    fn test_basic_analyzer_name() {
        let analyzer = BasicAnalyzer::new();
        assert_eq!(analyzer.name(), "basic");
    }

    #[test]
    fn test_basic_analyzer_latency() {
        let analyzer = BasicAnalyzer::new();
        assert_eq!(analyzer.estimated_latency_ms(), 1);
    }

    #[test]
    fn test_basic_analyzer_default() {
        let analyzer = BasicAnalyzer;
        assert_eq!(analyzer.name(), "basic");
    }
}
