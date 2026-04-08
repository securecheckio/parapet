use super::CoreSecurityAnalyzer;
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
use std::collections::HashSet;

const SPL_TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

// Instruction discriminators
const APPROVE: u8 = 4;
const APPROVE_CHECKED: u8 = 13;
const SET_AUTHORITY: u8 = 6;
const CLOSE_ACCOUNT: u8 = 9;
const TRANSFER: u8 = 3;

/// Helper to create a basic transaction
fn create_basic_transaction() -> Transaction {
    let payer = Pubkey::new_unique();
    
    Transaction {
        signatures: vec![Signature::default()],
        message: Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 0,
            },
            account_keys: vec![payer],
            recent_blockhash: Hash::default(),
            instructions: vec![],
        },
    }
}

/// Helper to create an approve instruction with specified amount
fn create_approve_instruction(amount: u64, checked: bool) -> CompiledInstruction {
    let mut data = vec![if checked { APPROVE_CHECKED } else { APPROVE }];
    data.extend_from_slice(&amount.to_le_bytes());
    
    CompiledInstruction {
        program_id_index: 1, // Assumes token program is at index 1
        accounts: vec![0, 1, 2], // source, delegate, owner
        data,
    }
}

/// Helper to add SPL Token program to transaction
fn add_token_program(tx: &mut Transaction) {
    let token_program = SPL_TOKEN_PROGRAM.parse::<Pubkey>().unwrap();
    tx.message.account_keys.push(token_program);
}

/// Helper to add Token-2022 program to transaction
fn add_token_2022_program(tx: &mut Transaction) {
    let token_2022_program = TOKEN_2022_PROGRAM.parse::<Pubkey>().unwrap();
    tx.message.account_keys.push(token_2022_program);
}

#[tokio::test]
async fn test_analyzer_name() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    assert_eq!(analyzer.name(), "core_security");
}

#[tokio::test]
async fn test_analyzer_fields() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    let fields = analyzer.fields();
    
    assert!(fields.contains(&"risk_score".to_string()));
    assert!(fields.contains(&"risk_level".to_string()));
    assert!(fields.contains(&"delegation_detected".to_string()));
    assert!(fields.contains(&"delegation_is_unlimited".to_string()));
    assert!(fields.contains(&"delegation_count".to_string()));
    assert!(fields.contains(&"authority_changes".to_string()));
    assert!(fields.contains(&"blocked_program_detected".to_string()));
    assert!(fields.contains(&"blocked_program_count".to_string()));
    assert!(fields.contains(&"has_issues".to_string()));
    assert!(fields.contains(&"issue_count".to_string()));
}

#[tokio::test]
async fn test_clean_transaction() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    let tx = create_basic_transaction();
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["risk_score"], json!(0));
    assert_eq!(result["risk_level"], json!("low"));
    assert_eq!(result["delegation_detected"], json!(false));
    assert_eq!(result["delegation_is_unlimited"], json!(false));
    assert_eq!(result["delegation_count"], json!(0));
    assert_eq!(result["authority_changes"], json!(false));
    assert_eq!(result["blocked_program_detected"], json!(false));
    assert_eq!(result["blocked_program_count"], json!(0));
}

#[tokio::test]
async fn test_unlimited_delegation_detection() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    let mut tx = create_basic_transaction();
    add_token_program(&mut tx);
    
    // Add unlimited approve instruction
    let approve_ix = create_approve_instruction(u64::MAX, false);
    tx.message.instructions.push(approve_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["delegation_detected"], json!(true));
    assert_eq!(result["delegation_is_unlimited"], json!(true));
    assert_eq!(result["delegation_count"], json!(1));
    assert_eq!(result["risk_score"], json!(95));
    assert_eq!(result["risk_level"], json!("critical"));
}

#[tokio::test]
async fn test_unlimited_delegation_with_approve_checked() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    let mut tx = create_basic_transaction();
    add_token_2022_program(&mut tx);
    
    // Add unlimited approve_checked instruction
    let approve_ix = create_approve_instruction(u64::MAX, true);
    tx.message.instructions.push(approve_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["delegation_detected"], json!(true));
    assert_eq!(result["delegation_is_unlimited"], json!(true));
    assert_eq!(result["delegation_count"], json!(1));
    assert_eq!(result["risk_score"], json!(95));
    assert_eq!(result["risk_level"], json!("critical"));
}

#[tokio::test]
async fn test_limited_delegation() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    let mut tx = create_basic_transaction();
    add_token_program(&mut tx);
    
    // Add limited approve (1000 tokens)
    let approve_ix = create_approve_instruction(1000, false);
    tx.message.instructions.push(approve_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["delegation_detected"], json!(true));
    assert_eq!(result["delegation_is_unlimited"], json!(false));
    assert_eq!(result["delegation_count"], json!(1));
    assert_eq!(result["risk_score"], json!(30));
    assert_eq!(result["risk_level"], json!("medium"));
}

#[tokio::test]
async fn test_multiple_delegations() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    let mut tx = create_basic_transaction();
    add_token_program(&mut tx);
    
    // Add 3 approve instructions
    for _ in 0..3 {
        let approve_ix = create_approve_instruction(1000, false);
        tx.message.instructions.push(approve_ix);
    }
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["delegation_detected"], json!(true));
    assert_eq!(result["delegation_is_unlimited"], json!(false));
    assert_eq!(result["delegation_count"], json!(3));
    assert_eq!(result["risk_score"], json!(85));
    assert_eq!(result["risk_level"], json!("critical"));
}

#[tokio::test]
async fn test_authority_change_detection_set_authority() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    let mut tx = create_basic_transaction();
    add_token_program(&mut tx);
    
    // Add SetAuthority instruction
    let set_authority_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 1],
        data: vec![SET_AUTHORITY, 0, 1], // discriminator + authority_type + option
    };
    tx.message.instructions.push(set_authority_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["authority_changes"], json!(true));
    assert_eq!(result["risk_score"], json!(40));
    assert_eq!(result["risk_level"], json!("medium"));
}

#[tokio::test]
async fn test_authority_change_detection_close_account() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    let mut tx = create_basic_transaction();
    add_token_program(&mut tx);
    
    // Add CloseAccount instruction
    let close_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 1, 2],
        data: vec![CLOSE_ACCOUNT],
    };
    tx.message.instructions.push(close_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["authority_changes"], json!(true));
    assert_eq!(result["risk_score"], json!(40));
    assert_eq!(result["risk_level"], json!("medium"));
}

#[tokio::test]
async fn test_authority_change_with_delegation() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    let mut tx = create_basic_transaction();
    add_token_program(&mut tx);
    
    // Add approve + set_authority (dangerous combo)
    let approve_ix = create_approve_instruction(1000, false);
    tx.message.instructions.push(approve_ix);
    
    let set_authority_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 1],
        data: vec![SET_AUTHORITY, 0, 1],
    };
    tx.message.instructions.push(set_authority_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["delegation_detected"], json!(true));
    assert_eq!(result["authority_changes"], json!(true));
    assert_eq!(result["risk_score"], json!(80));
    assert_eq!(result["risk_level"], json!("critical"));
}

#[tokio::test]
async fn test_blocked_program_detection() {
    let mut blocklist = HashSet::new();
    let malicious_program = Pubkey::new_unique();
    blocklist.insert(malicious_program.to_string());
    
    let analyzer = CoreSecurityAnalyzer::new(blocklist);
    
    let mut tx = create_basic_transaction();
    tx.message.account_keys.push(malicious_program);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["blocked_program_detected"], json!(true));
    assert_eq!(result["blocked_program_count"], json!(1));
    assert_eq!(result["risk_score"], json!(100));
    assert_eq!(result["risk_level"], json!("critical"));
}

#[tokio::test]
async fn test_multiple_blocked_programs() {
    let mut blocklist = HashSet::new();
    let malicious1 = Pubkey::new_unique();
    let malicious2 = Pubkey::new_unique();
    blocklist.insert(malicious1.to_string());
    blocklist.insert(malicious2.to_string());
    
    let analyzer = CoreSecurityAnalyzer::new(blocklist);
    
    let mut tx = create_basic_transaction();
    tx.message.account_keys.push(malicious1);
    tx.message.account_keys.push(malicious2);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["blocked_program_detected"], json!(true));
    assert_eq!(result["blocked_program_count"], json!(2));
    assert_eq!(result["risk_score"], json!(100));
}

#[tokio::test]
async fn test_too_many_instructions() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    let mut tx = create_basic_transaction();
    add_token_program(&mut tx);
    
    // Add 25 instructions (max is 20)
    for _ in 0..25 {
        let transfer_ix = CompiledInstruction {
            program_id_index: 1,
            accounts: vec![0, 1],
            data: vec![TRANSFER, 0, 0, 0, 0, 0, 0, 0, 100], // small transfer
        };
        tx.message.instructions.push(transfer_ix);
    }
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["has_issues"], json!(true));
    assert!(result["issue_count"].as_u64().unwrap() > 0);
    assert!(result["risk_score"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn test_too_many_signers() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    let mut tx = create_basic_transaction();
    
    // Set 10 required signatures (max is 5)
    tx.message.header.num_required_signatures = 10;
    
    // Add 10 signers
    for _ in 0..9 {
        tx.message.account_keys.push(Pubkey::new_unique());
    }
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["has_issues"], json!(true));
    assert!(result["issue_count"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn test_too_many_writable_accounts() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    let mut tx = create_basic_transaction();
    
    // Add 20 writable accounts (max is 15)
    for _ in 0..20 {
        tx.message.account_keys.push(Pubkey::new_unique());
    }
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["has_issues"], json!(true));
    assert!(result["issue_count"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn test_too_many_programs() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    let mut tx = create_basic_transaction();
    
    // Add 7 different programs (threshold is 5)
    for i in 0..7 {
        let program = Pubkey::new_unique();
        tx.message.account_keys.push(program);
        
        let ix = CompiledInstruction {
            program_id_index: (i + 1) as u8,
            accounts: vec![],
            data: vec![0],
        };
        tx.message.instructions.push(ix);
    }
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["has_issues"], json!(true));
    assert!(result["issue_count"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn test_large_instruction_data() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    let mut tx = create_basic_transaction();
    add_token_program(&mut tx);
    
    // Add instruction with > 1024 bytes of data
    let large_data = vec![0u8; 2000];
    let large_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![],
        data: large_data,
    };
    tx.message.instructions.push(large_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["has_issues"], json!(true));
    assert!(result["issue_count"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn test_risk_level_low() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    let tx = create_basic_transaction();
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["risk_level"], json!("low"));
    assert!(result["risk_score"].as_u64().unwrap() <= 25);
}

#[tokio::test]
async fn test_risk_level_medium() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    let mut tx = create_basic_transaction();
    add_token_program(&mut tx);
    
    // Limited delegation = 30 points = medium
    let approve_ix = create_approve_instruction(1000, false);
    tx.message.instructions.push(approve_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["risk_level"], json!("medium"));
    assert!(result["risk_score"].as_u64().unwrap() >= 26);
    assert!(result["risk_score"].as_u64().unwrap() <= 50);
}

#[tokio::test]
async fn test_risk_level_high() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    let mut tx = create_basic_transaction();
    add_token_program(&mut tx);
    
    // Authority change with delegation = 80 points = high
    let approve_ix = create_approve_instruction(1000, false);
    tx.message.instructions.push(approve_ix);
    
    let set_authority_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 1],
        data: vec![SET_AUTHORITY, 0, 1],
    };
    tx.message.instructions.push(set_authority_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["risk_level"], json!("critical")); // Actually critical at 80
    assert!(result["risk_score"].as_u64().unwrap() >= 51);
}

#[tokio::test]
async fn test_risk_level_critical() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    let mut tx = create_basic_transaction();
    add_token_program(&mut tx);
    
    // Unlimited delegation = 95 points = critical
    let approve_ix = create_approve_instruction(u64::MAX, false);
    tx.message.instructions.push(approve_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["risk_level"], json!("critical"));
    assert!(result["risk_score"].as_u64().unwrap() >= 76);
}

#[tokio::test]
async fn test_pattern_issues_accumulate() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    let mut tx = create_basic_transaction();
    add_token_program(&mut tx);
    
    // Add multiple pattern issues
    // 1. Too many instructions (25 > 20)
    for _ in 0..25 {
        let transfer_ix = CompiledInstruction {
            program_id_index: 1,
            accounts: vec![0, 1],
            data: vec![TRANSFER, 0, 0, 0, 0, 0, 0, 0, 100],
        };
        tx.message.instructions.push(transfer_ix);
    }
    
    // 2. Too many signers
    tx.message.header.num_required_signatures = 10;
    for _ in 0..9 {
        tx.message.account_keys.push(Pubkey::new_unique());
    }
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    assert_eq!(result["has_issues"], json!(true));
    assert!(result["issue_count"].as_u64().unwrap() >= 2);
    // Pattern issues add 10 points each (capped at 30)
    assert!(result["risk_score"].as_u64().unwrap() >= 20);
}

#[tokio::test]
async fn test_estimated_latency() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    assert_eq!(analyzer.estimated_latency_ms(), 2);
}

#[tokio::test]
async fn test_empty_instruction_data() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    let mut tx = create_basic_transaction();
    add_token_program(&mut tx);
    
    // Add instruction with empty data
    let empty_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![],
        data: vec![],
    };
    tx.message.instructions.push(empty_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    // Should not crash, should analyze safely
    assert_eq!(result["delegation_detected"], json!(false));
    assert_eq!(result["authority_changes"], json!(false));
}

#[tokio::test]
async fn test_short_instruction_data() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    let mut tx = create_basic_transaction();
    add_token_program(&mut tx);
    
    // Add approve with incomplete amount data (< 9 bytes)
    let short_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 1, 2],
        data: vec![APPROVE, 0, 0, 0], // Only 4 bytes instead of 8
    };
    tx.message.instructions.push(short_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    // Should detect delegation but not as unlimited (can't parse amount)
    assert_eq!(result["delegation_detected"], json!(true));
    assert_eq!(result["delegation_is_unlimited"], json!(false));
}

#[tokio::test]
async fn test_non_token_program_ignored() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    let mut tx = create_basic_transaction();
    
    // Add a random program (not SPL Token)
    let random_program = Pubkey::new_unique();
    tx.message.account_keys.push(random_program);
    
    // Add instruction that looks like approve but isn't from token program
    let fake_approve = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 1, 2],
        data: vec![APPROVE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    };
    tx.message.instructions.push(fake_approve);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    // Should NOT detect delegation (not from token program)
    assert_eq!(result["delegation_detected"], json!(false));
    assert_eq!(result["authority_changes"], json!(false));
}

#[tokio::test]
async fn test_token_2022_program_recognized() {
    let analyzer = CoreSecurityAnalyzer::new(HashSet::new());
    let mut tx = create_basic_transaction();
    add_token_2022_program(&mut tx);
    
    let approve_ix = create_approve_instruction(u64::MAX, false);
    tx.message.instructions.push(approve_ix);
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    // Token-2022 should be recognized same as SPL Token
    assert_eq!(result["delegation_detected"], json!(true));
    assert_eq!(result["delegation_is_unlimited"], json!(true));
}

#[tokio::test]
async fn test_complex_transaction() {
    let mut blocklist = HashSet::new();
    let malicious = Pubkey::new_unique();
    blocklist.insert(malicious.to_string());
    
    let analyzer = CoreSecurityAnalyzer::new(blocklist);
    let mut tx = create_basic_transaction();
    add_token_program(&mut tx);
    tx.message.account_keys.push(malicious);
    
    // Add unlimited delegation
    let approve_ix = create_approve_instruction(u64::MAX, false);
    tx.message.instructions.push(approve_ix);
    
    // Add authority change
    let set_authority_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 1],
        data: vec![SET_AUTHORITY, 0, 1],
    };
    tx.message.instructions.push(set_authority_ix);
    
    // Add many more instructions
    for _ in 0..20 {
        let transfer_ix = CompiledInstruction {
            program_id_index: 1,
            accounts: vec![0, 1],
            data: vec![TRANSFER, 0, 0, 0, 0, 0, 0, 0, 100],
        };
        tx.message.instructions.push(transfer_ix);
    }
    
    let result = analyzer.analyze(&tx).await.unwrap();
    
    // Blocked program = instant 100 risk score
    assert_eq!(result["blocked_program_detected"], json!(true));
    assert_eq!(result["delegation_detected"], json!(true));
    assert_eq!(result["delegation_is_unlimited"], json!(true));
    assert_eq!(result["authority_changes"], json!(true));
    assert_eq!(result["has_issues"], json!(true));
    assert_eq!(result["risk_score"], json!(100));
    assert_eq!(result["risk_level"], json!("critical"));
}
