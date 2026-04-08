use super::SystemProgramAnalyzer;
use crate::rules::analyzer::TransactionAnalyzer;
use serde_json::json;
use solana_sdk::{
    hash::Hash,
    instruction::CompiledInstruction,
    message::{Message, MessageHeader},
    pubkey::Pubkey,
    signature::Signature,
    transaction::Transaction,
};
use std::str::FromStr;

const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";

// System program instruction discriminators (u32 little-endian)
const CREATE_ACCOUNT: u32 = 0;
const ASSIGN: u32 = 1;
const TRANSFER: u32 = 2;
const CREATE_ACCOUNT_WITH_SEED: u32 = 3;
const ADVANCE_NONCE_ACCOUNT: u32 = 4;
const ALLOCATE: u32 = 8;
const ALLOCATE_WITH_SEED: u32 = 9;

/// Helper to create a basic transaction with system program
fn create_system_transaction() -> Transaction {
    let payer = Pubkey::new_unique();
    let system_program = Pubkey::from_str(SYSTEM_PROGRAM).unwrap();
    
    Transaction {
        signatures: vec![Signature::default()],
        message: Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec![payer, system_program],
            recent_blockhash: Hash::default(),
            instructions: vec![],
        },
    }
}

/// Helper to create a SOL transfer instruction
fn create_transfer_instruction(lamports: u64) -> CompiledInstruction {
    let mut data = vec![];
    data.extend_from_slice(&TRANSFER.to_le_bytes()); // discriminator
    data.extend_from_slice(&lamports.to_le_bytes()); // amount
    
    CompiledInstruction {
        program_id_index: 1, // system program at index 1
        accounts: vec![0, 2], // from, to (add recipient to account_keys)
        data,
    }
}

/// Helper to create a CreateAccount instruction
fn create_create_account_instruction(lamports: u64, space: u64) -> CompiledInstruction {
    let mut data = vec![];
    data.extend_from_slice(&CREATE_ACCOUNT.to_le_bytes()); // discriminator
    data.extend_from_slice(&lamports.to_le_bytes()); // lamports
    data.extend_from_slice(&space.to_le_bytes()); // space
    data.extend_from_slice(&[0u8; 32]); // owner pubkey
    
    CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 2], // from, new_account
        data,
    }
}

#[tokio::test]
async fn test_analyzer_name() {
    let analyzer = SystemProgramAnalyzer::new();
    assert_eq!(analyzer.name(), "system");
}

#[tokio::test]
async fn test_analyzer_fields() {
    let analyzer = SystemProgramAnalyzer::new();
    let fields = analyzer.fields();
    
    assert!(fields.contains(&"has_sol_transfer".to_string()));
    assert!(fields.contains(&"sol_transfer_count".to_string()));
    assert!(fields.contains(&"total_sol_transferred".to_string()));
    assert!(fields.contains(&"max_sol_transfer".to_string()));
    assert!(fields.contains(&"sol_recipients".to_string()));
    assert!(fields.contains(&"creates_accounts".to_string()));
    assert!(fields.contains(&"account_creation_count".to_string()));
    assert!(fields.contains(&"total_rent_required".to_string()));
    assert!(fields.contains(&"assigns_program_ownership".to_string()));
    assert!(fields.contains(&"assign_count".to_string()));
    assert!(fields.contains(&"uses_durable_nonce".to_string()));
    assert!(fields.contains(&"allocate_count".to_string()));
    assert!(fields.contains(&"high_rent_spam".to_string()));
    assert!(fields.contains(&"large_sol_transfer".to_string()));
}

#[tokio::test]
async fn test_no_system_instructions() {
    let analyzer = SystemProgramAnalyzer::new();
    let tx = create_system_transaction();
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["has_sol_transfer"], json!(false));
    assert_eq!(result["sol_transfer_count"], json!(0));
    assert_eq!(result["total_sol_transferred"], json!(0));
    assert_eq!(result["creates_accounts"], json!(false));
    assert_eq!(result["assigns_program_ownership"], json!(false));
    assert_eq!(result["uses_durable_nonce"], json!(false));
    assert_eq!(result["high_rent_spam"], json!(false));
    assert_eq!(result["large_sol_transfer"], json!(false));
}

#[tokio::test]
async fn test_single_sol_transfer() {
    let analyzer = SystemProgramAnalyzer::new();
    let mut tx = create_system_transaction();
    
    // Add recipient
    let recipient = Pubkey::new_unique();
    tx.message.account_keys.push(recipient);
    
    // Add transfer instruction (0.5 SOL = 500M lamports)
    let transfer_ix = create_transfer_instruction(500_000_000);
    tx.message.instructions.push(transfer_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["has_sol_transfer"], json!(true));
    assert_eq!(result["sol_transfer_count"], json!(1));
    assert_eq!(result["total_sol_transferred"], json!(500_000_000));
    assert_eq!(result["max_sol_transfer"], json!(500_000_000));
    assert_eq!(result["large_sol_transfer"], json!(false)); // < 1 SOL
    
    let recipients = result["sol_recipients"].as_array().unwrap();
    assert_eq!(recipients.len(), 1);
    assert_eq!(recipients[0], json!(recipient.to_string()));
}

#[tokio::test]
async fn test_multiple_sol_transfers() {
    let analyzer = SystemProgramAnalyzer::new();
    let mut tx = create_system_transaction();
    
    // Add 3 recipients
    let recipient1 = Pubkey::new_unique();
    let recipient2 = Pubkey::new_unique();
    let recipient3 = Pubkey::new_unique();
    tx.message.account_keys.push(recipient1);
    tx.message.account_keys.push(recipient2);
    tx.message.account_keys.push(recipient3);
    
    // Add 3 transfer instructions
    let amounts = [100_000_000, 500_000_000, 200_000_000]; // 0.1, 0.5, 0.2 SOL
    for (i, amount) in amounts.iter().enumerate() {
        let mut transfer_ix = create_transfer_instruction(*amount);
        transfer_ix.accounts[1] = (i + 2) as u8; // recipient index
        tx.message.instructions.push(transfer_ix);
    }
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["has_sol_transfer"], json!(true));
    assert_eq!(result["sol_transfer_count"], json!(3));
    assert_eq!(result["total_sol_transferred"], json!(800_000_000));
    assert_eq!(result["max_sol_transfer"], json!(500_000_000));
    
    let recipients = result["sol_recipients"].as_array().unwrap();
    assert_eq!(recipients.len(), 3);
}

#[tokio::test]
async fn test_large_sol_transfer() {
    let analyzer = SystemProgramAnalyzer::new();
    let mut tx = create_system_transaction();
    
    let recipient = Pubkey::new_unique();
    tx.message.account_keys.push(recipient);
    
    // Transfer 10 SOL = 10B lamports
    let transfer_ix = create_transfer_instruction(10_000_000_000);
    tx.message.instructions.push(transfer_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["large_sol_transfer"], json!(true)); // > 1 SOL
    assert_eq!(result["max_sol_transfer"], json!(10_000_000_000u64));
}

#[tokio::test]
async fn test_create_account() {
    let analyzer = SystemProgramAnalyzer::new();
    let mut tx = create_system_transaction();
    
    let new_account = Pubkey::new_unique();
    tx.message.account_keys.push(new_account);
    
    // Create account with 1 SOL rent and 165 bytes space
    let create_ix = create_create_account_instruction(1_000_000_000, 165);
    tx.message.instructions.push(create_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["creates_accounts"], json!(true));
    assert_eq!(result["account_creation_count"], json!(1));
    assert_eq!(result["total_rent_required"], json!(1_000_000_000));
    assert_eq!(result["high_rent_spam"], json!(false)); // < 10 accounts
}

#[tokio::test]
async fn test_multiple_account_creations() {
    let analyzer = SystemProgramAnalyzer::new();
    let mut tx = create_system_transaction();
    
    // Add 5 new accounts
    for _ in 0..5 {
        tx.message.account_keys.push(Pubkey::new_unique());
    }
    
    // Create 5 accounts
    for i in 0..5 {
        let mut create_ix = create_create_account_instruction(100_000_000, 165);
        create_ix.accounts[1] = (i + 2) as u8; // new account index
        tx.message.instructions.push(create_ix);
    }
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["creates_accounts"], json!(true));
    assert_eq!(result["account_creation_count"], json!(5));
    assert_eq!(result["total_rent_required"], json!(500_000_000));
    assert_eq!(result["high_rent_spam"], json!(false)); // < 10
}

#[tokio::test]
async fn test_high_rent_spam_detection() {
    let analyzer = SystemProgramAnalyzer::new();
    let mut tx = create_system_transaction();
    
    // Add 15 new accounts
    for _ in 0..15 {
        tx.message.account_keys.push(Pubkey::new_unique());
    }
    
    // Create 15 accounts (> 10 = spam)
    for i in 0..15 {
        let mut create_ix = create_create_account_instruction(10_000_000, 165);
        create_ix.accounts[1] = (i + 2) as u8;
        tx.message.instructions.push(create_ix);
    }
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["account_creation_count"], json!(15));
    assert_eq!(result["high_rent_spam"], json!(true)); // > 10 accounts
    assert_eq!(result["total_rent_required"], json!(150_000_000));
}

#[tokio::test]
async fn test_create_account_with_seed() {
    let analyzer = SystemProgramAnalyzer::new();
    let mut tx = create_system_transaction();
    
    let new_account = Pubkey::new_unique();
    tx.message.account_keys.push(new_account);
    
    // CreateAccountWithSeed instruction
    let mut data = vec![];
    data.extend_from_slice(&CREATE_ACCOUNT_WITH_SEED.to_le_bytes());
    data.extend_from_slice(&[0u8; 32]); // base pubkey
    data.extend_from_slice(&[4u8, 0, 0, 0]); // seed length (u32)
    data.extend_from_slice(b"seed"); // seed string
    data.extend_from_slice(&1_000_000_000u64.to_le_bytes()); // lamports
    data.extend_from_slice(&165u64.to_le_bytes()); // space
    data.extend_from_slice(&[0u8; 32]); // owner
    
    let create_seed_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 2],
        data,
    };
    tx.message.instructions.push(create_seed_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["creates_accounts"], json!(true));
    assert_eq!(result["account_creation_count"], json!(1));
}

#[tokio::test]
async fn test_assign_instruction() {
    let analyzer = SystemProgramAnalyzer::new();
    let mut tx = create_system_transaction();
    
    let target_program = Pubkey::new_unique();
    tx.message.account_keys.push(target_program);
    
    // Assign instruction
    let mut data = vec![];
    data.extend_from_slice(&ASSIGN.to_le_bytes());
    data.extend_from_slice(&target_program.to_bytes());
    
    let assign_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0],
        data,
    };
    tx.message.instructions.push(assign_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["assigns_program_ownership"], json!(true));
    assert_eq!(result["assign_count"], json!(1));
}

#[tokio::test]
async fn test_multiple_assign_instructions() {
    let analyzer = SystemProgramAnalyzer::new();
    let mut tx = create_system_transaction();
    
    // Add 3 assign instructions
    for _ in 0..3 {
        let target_program = Pubkey::new_unique();
        tx.message.account_keys.push(target_program);
        
        let mut data = vec![];
        data.extend_from_slice(&ASSIGN.to_le_bytes());
        data.extend_from_slice(&target_program.to_bytes());
        
        let assign_ix = CompiledInstruction {
            program_id_index: 1,
            accounts: vec![0],
            data,
        };
        tx.message.instructions.push(assign_ix);
    }
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["assigns_program_ownership"], json!(true));
    assert_eq!(result["assign_count"], json!(3));
}

#[tokio::test]
async fn test_durable_nonce_detection() {
    let analyzer = SystemProgramAnalyzer::new();
    let mut tx = create_system_transaction();
    
    // AdvanceNonceAccount instruction
    let nonce_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 2], // nonce_account, recent_blockhashes_sysvar
        data: ADVANCE_NONCE_ACCOUNT.to_le_bytes().to_vec(),
    };
    tx.message.instructions.push(nonce_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["uses_durable_nonce"], json!(true));
}

#[tokio::test]
async fn test_allocate_instruction() {
    let analyzer = SystemProgramAnalyzer::new();
    let mut tx = create_system_transaction();
    
    // Allocate instruction
    let mut data = vec![];
    data.extend_from_slice(&ALLOCATE.to_le_bytes());
    data.extend_from_slice(&165u64.to_le_bytes()); // space
    
    let allocate_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0],
        data,
    };
    tx.message.instructions.push(allocate_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["allocate_count"], json!(1));
}

#[tokio::test]
async fn test_allocate_with_seed_instruction() {
    let analyzer = SystemProgramAnalyzer::new();
    let mut tx = create_system_transaction();
    
    // AllocateWithSeed instruction
    let mut data = vec![];
    data.extend_from_slice(&ALLOCATE_WITH_SEED.to_le_bytes());
    data.extend_from_slice(&[0u8; 32]); // base pubkey
    data.extend_from_slice(&[4u8, 0, 0, 0]); // seed length
    data.extend_from_slice(b"seed"); // seed
    data.extend_from_slice(&165u64.to_le_bytes()); // space
    data.extend_from_slice(&[0u8; 32]); // owner
    
    let allocate_seed_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0],
        data,
    };
    tx.message.instructions.push(allocate_seed_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["allocate_count"], json!(1));
}

#[tokio::test]
async fn test_unknown_system_instruction() {
    let analyzer = SystemProgramAnalyzer::new();
    let mut tx = create_system_transaction();
    
    // Unknown discriminator (999)
    let unknown_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![],
        data: 999u32.to_le_bytes().to_vec(),
    };
    tx.message.instructions.push(unknown_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    // Should not crash, should handle gracefully
    assert_eq!(result["has_sol_transfer"], json!(false));
    assert_eq!(result["creates_accounts"], json!(false));
}

#[tokio::test]
async fn test_empty_instruction_data() {
    let analyzer = SystemProgramAnalyzer::new();
    let mut tx = create_system_transaction();
    
    // System instruction with empty data
    let empty_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![],
        data: vec![],
    };
    tx.message.instructions.push(empty_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    // Should not crash
    assert_eq!(result["has_sol_transfer"], json!(false));
}

#[tokio::test]
async fn test_short_instruction_data() {
    let analyzer = SystemProgramAnalyzer::new();
    let mut tx = create_system_transaction();
    
    // Transfer with incomplete data (only 2 bytes instead of 12)
    let short_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 2],
        data: vec![TRANSFER as u8, 0], // Incomplete
    };
    tx.message.instructions.push(short_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    // Should not crash, should handle gracefully
    // Won't detect as transfer because discriminator parse fails
    assert_eq!(result["has_sol_transfer"], json!(false));
}

#[tokio::test]
async fn test_non_system_program_ignored() {
    let analyzer = SystemProgramAnalyzer::new();
    let mut tx = create_system_transaction();
    
    // Add a different program
    let other_program = Pubkey::new_unique();
    tx.message.account_keys.push(other_program);
    
    // Add instruction that looks like transfer but from different program
    let fake_transfer = CompiledInstruction {
        program_id_index: 2, // other_program, not system
        accounts: vec![0, 1],
        data: create_transfer_instruction(1_000_000_000).data,
    };
    tx.message.instructions.push(fake_transfer);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    // Should NOT detect transfer (not from system program)
    assert_eq!(result["has_sol_transfer"], json!(false));
}

#[tokio::test]
async fn test_complex_system_transaction() {
    let analyzer = SystemProgramAnalyzer::new();
    let mut tx = create_system_transaction();
    
    // Add multiple accounts
    let recipient1 = Pubkey::new_unique();
    let recipient2 = Pubkey::new_unique();
    let new_account1 = Pubkey::new_unique();
    let new_account2 = Pubkey::new_unique();
    tx.message.account_keys.push(recipient1);
    tx.message.account_keys.push(recipient2);
    tx.message.account_keys.push(new_account1);
    tx.message.account_keys.push(new_account2);
    
    // Add 2 transfers
    let mut transfer1 = create_transfer_instruction(500_000_000);
    transfer1.accounts[1] = 2;
    tx.message.instructions.push(transfer1);
    
    let mut transfer2 = create_transfer_instruction(2_000_000_000);
    transfer2.accounts[1] = 3;
    tx.message.instructions.push(transfer2);
    
    // Add 2 account creations
    let mut create1 = create_create_account_instruction(100_000_000, 165);
    create1.accounts[1] = 4;
    tx.message.instructions.push(create1);
    
    let mut create2 = create_create_account_instruction(200_000_000, 165);
    create2.accounts[1] = 5;
    tx.message.instructions.push(create2);
    
    // Add assign
    let assign_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0],
        data: ASSIGN.to_le_bytes().to_vec(),
    };
    tx.message.instructions.push(assign_ix);
    
    // Add nonce
    let nonce_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 2],
        data: ADVANCE_NONCE_ACCOUNT.to_le_bytes().to_vec(),
    };
    tx.message.instructions.push(nonce_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["has_sol_transfer"], json!(true));
    assert_eq!(result["sol_transfer_count"], json!(2));
    assert_eq!(result["total_sol_transferred"], json!(2_500_000_000u64));
    assert_eq!(result["max_sol_transfer"], json!(2_000_000_000u64));
    assert_eq!(result["large_sol_transfer"], json!(true)); // 2 SOL > 1 SOL
    
    assert_eq!(result["creates_accounts"], json!(true));
    assert_eq!(result["account_creation_count"], json!(2));
    assert_eq!(result["total_rent_required"], json!(300_000_000));
    
    assert_eq!(result["assigns_program_ownership"], json!(true));
    assert_eq!(result["assign_count"], json!(1));
    
    assert_eq!(result["uses_durable_nonce"], json!(true));
}

#[tokio::test]
async fn test_estimated_latency() {
    let analyzer = SystemProgramAnalyzer::new();
    assert_eq!(analyzer.estimated_latency_ms(), 1);
}

#[tokio::test]
async fn test_default_constructor() {
    let analyzer = SystemProgramAnalyzer::default();
    assert_eq!(analyzer.name(), "system");
}

#[tokio::test]
async fn test_sol_recipients_list() {
    let analyzer = SystemProgramAnalyzer::new();
    let mut tx = create_system_transaction();
    
    let recipient1 = Pubkey::new_unique();
    let recipient2 = Pubkey::new_unique();
    tx.message.account_keys.push(recipient1);
    tx.message.account_keys.push(recipient2);
    
    // Transfer to recipient1
    let mut transfer1 = create_transfer_instruction(100_000_000);
    transfer1.accounts[1] = 2;
    tx.message.instructions.push(transfer1);
    
    // Transfer to recipient2
    let mut transfer2 = create_transfer_instruction(200_000_000);
    transfer2.accounts[1] = 3;
    tx.message.instructions.push(transfer2);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    let recipients = result["sol_recipients"].as_array().unwrap();
    assert_eq!(recipients.len(), 2);
    assert!(recipients.contains(&json!(recipient1.to_string())));
    assert!(recipients.contains(&json!(recipient2.to_string())));
}

#[tokio::test]
async fn test_zero_amount_transfer() {
    let analyzer = SystemProgramAnalyzer::new();
    let mut tx = create_system_transaction();
    
    let recipient = Pubkey::new_unique();
    tx.message.account_keys.push(recipient);
    
    // Transfer 0 lamports (edge case)
    let transfer_ix = create_transfer_instruction(0);
    tx.message.instructions.push(transfer_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["has_sol_transfer"], json!(true));
    assert_eq!(result["sol_transfer_count"], json!(1));
    assert_eq!(result["total_sol_transferred"], json!(0));
    assert_eq!(result["large_sol_transfer"], json!(false));
}

#[tokio::test]
async fn test_max_u64_transfer() {
    let analyzer = SystemProgramAnalyzer::new();
    let mut tx = create_system_transaction();
    
    let recipient = Pubkey::new_unique();
    tx.message.account_keys.push(recipient);
    
    // Transfer max u64 (impossible but test edge case)
    let transfer_ix = create_transfer_instruction(u64::MAX);
    tx.message.instructions.push(transfer_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["has_sol_transfer"], json!(true));
    assert_eq!(result["max_sol_transfer"], json!(u64::MAX));
    assert_eq!(result["large_sol_transfer"], json!(true));
}

#[tokio::test]
async fn test_mixed_system_and_token_instructions() {
    let analyzer = SystemProgramAnalyzer::new();
    let mut tx = create_system_transaction();
    
    // Add token program
    let token_program = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        .parse::<Pubkey>()
        .unwrap();
    tx.message.account_keys.push(token_program);
    
    let recipient = Pubkey::new_unique();
    tx.message.account_keys.push(recipient);
    
    // Add system transfer
    let mut transfer_ix = create_transfer_instruction(500_000_000);
    transfer_ix.accounts[1] = 3;
    tx.message.instructions.push(transfer_ix);
    
    // Add token instruction (should be ignored by system analyzer)
    let token_ix = CompiledInstruction {
        program_id_index: 2, // token program
        accounts: vec![0, 1],
        data: vec![3, 0, 0, 0, 0, 0, 0, 0, 100], // token transfer
    };
    tx.message.instructions.push(token_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    // Should only count system transfer, not token transfer
    assert_eq!(result["has_sol_transfer"], json!(true));
    assert_eq!(result["sol_transfer_count"], json!(1));
    assert_eq!(result["total_sol_transferred"], json!(500_000_000));
}

#[tokio::test]
async fn test_invalid_account_index() {
    let analyzer = SystemProgramAnalyzer::new();
    let mut tx = create_system_transaction();
    
    // Transfer with invalid recipient index
    let mut transfer_ix = create_transfer_instruction(100_000_000);
    transfer_ix.accounts[1] = 99; // Invalid index (out of bounds)
    tx.message.instructions.push(transfer_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    // Should handle gracefully - transfer detected but no recipient added
    assert_eq!(result["has_sol_transfer"], json!(true));
    assert_eq!(result["sol_transfer_count"], json!(1));
    
    let recipients = result["sol_recipients"].as_array().unwrap();
    assert_eq!(recipients.len(), 0); // No recipient because index was invalid
}
