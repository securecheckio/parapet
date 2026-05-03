use super::TokenInstructionAnalyzer;
use crate::rules::analyzer::TransactionAnalyzer;
use serde_json::json;
use solana_sdk::{
    hash::Hash,
    message::compiled_instruction::CompiledInstruction,
    message::{Message, MessageHeader},
    pubkey::Pubkey,
    signature::Signature,
    transaction::Transaction,
};
use std::str::FromStr;

const SPL_TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

// SPL Token instruction discriminators
const INITIALIZE_MINT: u8 = 0;
const INITIALIZE_ACCOUNT: u8 = 1;
const INITIALIZE_MULTISIG: u8 = 2;
const TRANSFER: u8 = 3;
const APPROVE: u8 = 4;
const REVOKE: u8 = 5;
const SET_AUTHORITY: u8 = 6;
const MINT_TO: u8 = 7;
const BURN: u8 = 8;
const CLOSE_ACCOUNT: u8 = 9;
const FREEZE_ACCOUNT: u8 = 10;
const THAW_ACCOUNT: u8 = 11;
const TRANSFER_CHECKED: u8 = 12;
const APPROVE_CHECKED: u8 = 13;
const MINT_TO_CHECKED: u8 = 14;
const BURN_CHECKED: u8 = 15;

/// Helper to create a basic transaction with token program
fn create_token_transaction() -> Transaction {
    let payer = Pubkey::new_unique();
    let token_program = Pubkey::from_str(SPL_TOKEN_PROGRAM).unwrap();

    Transaction {
        signatures: vec![Signature::default()],
        message: Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec![payer, token_program],
            recent_blockhash: Hash::default(),
            instructions: vec![],
        },
    }
}

/// Helper to create a Transfer instruction
fn create_transfer_instruction(amount: u64, is_signer: bool) -> CompiledInstruction {
    let mut data = vec![TRANSFER];
    data.extend_from_slice(&amount.to_le_bytes());

    CompiledInstruction {
        program_id_index: 1,                                 // token program
        accounts: vec![0, 2, if is_signer { 0 } else { 3 }], // source, dest, owner
        data,
    }
}

/// Helper to create an Approve instruction
fn create_approve_instruction(amount: u64, is_signer: bool) -> CompiledInstruction {
    let mut data = vec![APPROVE];
    data.extend_from_slice(&amount.to_le_bytes());

    CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 2, if is_signer { 0 } else { 3 }], // source, delegate, owner
        data,
    }
}

#[tokio::test]
async fn test_analyzer_name() {
    let analyzer = TokenInstructionAnalyzer::new();
    assert_eq!(analyzer.name(), "token_instructions");
}

#[tokio::test]
async fn test_analyzer_fields() {
    let analyzer = TokenInstructionAnalyzer::new();
    let fields = analyzer.fields();

    assert!(fields.contains(&"has_transfer".to_string()));
    assert!(fields.contains(&"has_approve".to_string()));
    assert!(fields.contains(&"has_revoke".to_string()));
    assert!(fields.contains(&"has_mint".to_string()));
    assert!(fields.contains(&"has_burn".to_string()));
    assert!(fields.contains(&"unlimited_approve_count".to_string()));
    assert!(fields.contains(&"transfer_recipients".to_string()));
}

#[tokio::test]
async fn test_no_token_instructions() {
    let analyzer = TokenInstructionAnalyzer::new();
    let tx = create_token_transaction();

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["has_transfer"], json!(false));
    assert_eq!(result["has_approve"], json!(false));
    assert_eq!(result["transfer_count"], json!(0));
    assert_eq!(result["approve_count"], json!(0));
}

#[tokio::test]
async fn test_single_transfer() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    // Add destination account
    let dest = Pubkey::new_unique();
    tx.message.account_keys.push(dest);

    // Add transfer instruction (100 tokens)
    let transfer_ix = create_transfer_instruction(100, true);
    tx.message.instructions.push(transfer_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["has_transfer"], json!(true));
    assert_eq!(result["transfer_count"], json!(1));
    assert_eq!(result["total_transfer_amount"], json!(100));
    assert_eq!(result["max_transfer_amount"], json!(100));

    let recipients = result["transfer_recipients"].as_array().unwrap();
    assert_eq!(recipients.len(), 1);
    assert_eq!(recipients[0], json!(dest.to_string()));
}

#[tokio::test]
async fn test_multiple_transfers() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    // Add 3 destinations
    for _ in 0..3 {
        tx.message.account_keys.push(Pubkey::new_unique());
    }

    // Add 3 transfers
    let amounts = [100, 500, 200];
    for (i, amount) in amounts.iter().enumerate() {
        let mut transfer_ix = create_transfer_instruction(*amount, true);
        transfer_ix.accounts[1] = (i + 2) as u8; // destination index
        tx.message.instructions.push(transfer_ix);
    }

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["transfer_count"], json!(3));
    assert_eq!(result["total_transfer_amount"], json!(800));
    assert_eq!(result["max_transfer_amount"], json!(500));

    let recipients = result["transfer_recipients"].as_array().unwrap();
    assert_eq!(recipients.len(), 3);
}

#[tokio::test]
async fn test_transfer_checked() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    let dest = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    tx.message.account_keys.push(dest);
    tx.message.account_keys.push(mint);

    // TransferChecked instruction
    let mut data = vec![TRANSFER_CHECKED];
    data.extend_from_slice(&250u64.to_le_bytes()); // amount
    data.push(6); // decimals

    let transfer_checked_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 2, 3, 0], // source, dest, mint, owner
        data,
    };
    tx.message.instructions.push(transfer_checked_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["has_transfer"], json!(true));
    assert_eq!(result["transfer_count"], json!(1));
    assert_eq!(result["transfer_checked_count"], json!(1));
    assert_eq!(result["uses_transfer_checked"], json!(true));
    assert_eq!(result["total_transfer_amount"], json!(250));

    let mints = result["mints_involved"].as_array().unwrap();
    assert_eq!(mints.len(), 1);
    assert_eq!(mints[0], json!(mint.to_string()));
}

#[tokio::test]
async fn test_unlimited_approve() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    let delegate = Pubkey::new_unique();
    tx.message.account_keys.push(delegate);

    // Approve with u64::MAX (unlimited)
    let approve_ix = create_approve_instruction(u64::MAX, true);
    tx.message.instructions.push(approve_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["has_approve"], json!(true));
    assert_eq!(result["approve_count"], json!(1));
    assert_eq!(result["unlimited_approve_count"], json!(1));

    let delegates = result["delegate_addresses"].as_array().unwrap();
    assert_eq!(delegates.len(), 1);
    assert_eq!(delegates[0], json!(delegate.to_string()));
}

#[tokio::test]
async fn test_limited_approve() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    let delegate = Pubkey::new_unique();
    tx.message.account_keys.push(delegate);

    // Approve with limited amount
    let approve_ix = create_approve_instruction(1000, true);
    tx.message.instructions.push(approve_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["has_approve"], json!(true));
    assert_eq!(result["approve_count"], json!(1));
    assert_eq!(result["unlimited_approve_count"], json!(0));
}

#[tokio::test]
async fn test_approve_checked() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    let delegate = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    tx.message.account_keys.push(delegate);
    tx.message.account_keys.push(mint);

    // ApproveChecked instruction
    let mut data = vec![APPROVE_CHECKED];
    data.extend_from_slice(&500u64.to_le_bytes());
    data.push(6); // decimals

    let approve_checked_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 2, 3, 0], // source, delegate, mint, owner
        data,
    };
    tx.message.instructions.push(approve_checked_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["has_approve"], json!(true));
    assert_eq!(result["approve_count"], json!(1));

    let mints = result["mints_involved"].as_array().unwrap();
    assert_eq!(mints.len(), 1);
}

#[tokio::test]
async fn test_revoke() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    // Revoke instruction
    let revoke_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 0], // source, owner
        data: vec![REVOKE],
    };
    tx.message.instructions.push(revoke_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["has_revoke"], json!(true));
    assert_eq!(result["revoke_count"], json!(1));
}

#[tokio::test]
async fn test_net_delegation_change() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    let delegate = Pubkey::new_unique();
    tx.message.account_keys.push(delegate);

    // 3 approves
    for _ in 0..3 {
        let approve_ix = create_approve_instruction(100, true);
        tx.message.instructions.push(approve_ix);
    }

    // 1 revoke
    let revoke_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 0],
        data: vec![REVOKE],
    };
    tx.message.instructions.push(revoke_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["approve_count"], json!(3));
    assert_eq!(result["revoke_count"], json!(1));
    assert_eq!(result["net_delegation_change"], json!(2)); // 3 - 1 = 2
}

#[tokio::test]
async fn test_mint_to() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    // MintTo instruction
    let mut data = vec![MINT_TO];
    data.extend_from_slice(&1000u64.to_le_bytes());

    let mint_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 2, 0], // mint, destination, authority
        data,
    };
    tx.message.instructions.push(mint_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["has_mint"], json!(true));
    assert_eq!(result["mint_count"], json!(1));
    assert_eq!(result["total_mint_amount"], json!(1000));
    assert_eq!(result["modifies_supply"], json!(true));
}

#[tokio::test]
async fn test_mint_to_checked() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    // MintToChecked instruction
    let mut data = vec![MINT_TO_CHECKED];
    data.extend_from_slice(&2000u64.to_le_bytes());
    data.push(6); // decimals

    let mint_checked_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 2, 0], // mint, destination, authority
        data,
    };
    tx.message.instructions.push(mint_checked_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["has_mint"], json!(true));
    assert_eq!(result["mint_count"], json!(1));
    assert_eq!(result["total_mint_amount"], json!(2000));
}

#[tokio::test]
async fn test_burn() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    // Burn instruction
    let mut data = vec![BURN];
    data.extend_from_slice(&500u64.to_le_bytes());

    let burn_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 2, 0], // account, mint, owner
        data,
    };
    tx.message.instructions.push(burn_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["has_burn"], json!(true));
    assert_eq!(result["burn_count"], json!(1));
    assert_eq!(result["total_burn_amount"], json!(500));
    assert_eq!(result["modifies_supply"], json!(true));
}

#[tokio::test]
async fn test_burn_checked() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    // BurnChecked instruction
    let mut data = vec![BURN_CHECKED];
    data.extend_from_slice(&750u64.to_le_bytes());
    data.push(6); // decimals

    let burn_checked_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 2, 0], // account, mint, owner
        data,
    };
    tx.message.instructions.push(burn_checked_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["has_burn"], json!(true));
    assert_eq!(result["burn_count"], json!(1));
    assert_eq!(result["total_burn_amount"], json!(750));
}

#[tokio::test]
async fn test_freeze_account() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    // FreezeAccount instruction
    let freeze_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 2, 0], // account, mint, authority
        data: vec![FREEZE_ACCOUNT],
    };
    tx.message.instructions.push(freeze_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["has_freeze"], json!(true));
    assert_eq!(result["freeze_count"], json!(1));
    assert_eq!(result["modifies_permissions"], json!(true));
}

#[tokio::test]
async fn test_thaw_account() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    // ThawAccount instruction
    let thaw_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 2, 0], // account, mint, authority
        data: vec![THAW_ACCOUNT],
    };
    tx.message.instructions.push(thaw_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["has_thaw"], json!(true));
    assert_eq!(result["thaw_count"], json!(1));
}

#[tokio::test]
async fn test_close_account() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    // CloseAccount instruction
    let close_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 2, 0], // account, destination, owner
        data: vec![CLOSE_ACCOUNT],
    };
    tx.message.instructions.push(close_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["has_close_account"], json!(true));
    assert_eq!(result["close_count"], json!(1));
    assert_eq!(result["account_management_detected"], json!(true));
}

#[tokio::test]
async fn test_set_authority() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    // SetAuthority instruction
    let mut data = vec![SET_AUTHORITY];
    data.push(0); // authority type
    data.push(1); // new authority option (Some)
    data.extend_from_slice(&[0u8; 32]); // new authority pubkey

    let set_auth_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 0], // account, current_authority
        data,
    };
    tx.message.instructions.push(set_auth_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["has_set_authority"], json!(true));
    assert_eq!(result["set_authority_count"], json!(1));
    assert_eq!(result["modifies_permissions"], json!(true));
}

#[tokio::test]
async fn test_initialize_mint() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    // InitializeMint instruction
    let mut data = vec![INITIALIZE_MINT];
    data.push(6); // decimals
    data.extend_from_slice(&[0u8; 32]); // mint authority
    data.push(1); // freeze authority option
    data.extend_from_slice(&[0u8; 32]); // freeze authority

    let init_mint_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 2], // mint, rent_sysvar
        data,
    };
    tx.message.instructions.push(init_mint_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["has_initialize_mint"], json!(true));
}

#[tokio::test]
async fn test_initialize_account() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    // InitializeAccount instruction
    let init_account_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 2, 3, 4], // account, mint, owner, rent_sysvar
        data: vec![INITIALIZE_ACCOUNT],
    };
    tx.message.instructions.push(init_account_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["has_initialize_account"], json!(true));
    assert_eq!(result["account_management_detected"], json!(true));
}

#[tokio::test]
async fn test_initialize_multisig() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    // InitializeMultisig instruction
    let mut data = vec![INITIALIZE_MULTISIG];
    data.push(2); // m (required signers)

    let init_multisig_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 2, 3, 4], // multisig, rent_sysvar, signer1, signer2
        data,
    };
    tx.message.instructions.push(init_multisig_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    // No specific field for multisig, but should not crash
    assert_eq!(result["has_transfer"], json!(false));
}

#[tokio::test]
async fn test_unknown_instruction() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    // Unknown discriminator (99)
    let unknown_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![],
        data: vec![99],
    };
    tx.message.instructions.push(unknown_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    // Should handle gracefully
    assert_eq!(result["has_transfer"], json!(false));
}

#[tokio::test]
async fn test_non_owned_transfer() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    let dest = Pubkey::new_unique();
    let non_signer_owner = Pubkey::new_unique();
    tx.message.account_keys.push(dest);
    tx.message.account_keys.push(non_signer_owner);

    // Transfer with non-signer owner
    let transfer_ix = create_transfer_instruction(100, false);
    tx.message.instructions.push(transfer_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["transfers_from_non_owned"], json!(true));
    assert_eq!(result["non_owned_operations_count"], json!(1));
}

#[tokio::test]
async fn test_non_owned_approve() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    let delegate = Pubkey::new_unique();
    let non_signer_owner = Pubkey::new_unique();
    tx.message.account_keys.push(delegate);
    tx.message.account_keys.push(non_signer_owner);

    // Approve with non-signer owner
    let approve_ix = create_approve_instruction(100, false);
    tx.message.instructions.push(approve_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["approves_non_owned_tokens"], json!(true));
    assert_eq!(result["non_owned_operations_count"], json!(1));
}

#[tokio::test]
async fn test_non_owned_close() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    let dest = Pubkey::new_unique();
    let non_signer_owner = Pubkey::new_unique();
    tx.message.account_keys.push(dest);
    tx.message.account_keys.push(non_signer_owner);

    // CloseAccount with non-signer owner
    let close_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 2, 3], // account, destination, non-signer owner
        data: vec![CLOSE_ACCOUNT],
    };
    tx.message.instructions.push(close_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["closes_non_owned_account"], json!(true));
    assert_eq!(result["non_owned_operations_count"], json!(1));
}

#[tokio::test]
async fn test_non_owned_authority_change() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    let non_signer_authority = Pubkey::new_unique();
    tx.message.account_keys.push(non_signer_authority);

    // SetAuthority with non-signer current authority
    let mut data = vec![SET_AUTHORITY];
    data.push(0);
    data.push(0); // None

    let set_auth_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 2], // account, non-signer authority
        data,
    };
    tx.message.instructions.push(set_auth_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["modifies_non_owned_authority"], json!(true));
    assert_eq!(result["non_owned_operations_count"], json!(1));
}

#[tokio::test]
async fn test_dangerous_operation_combo_freeze_approve() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    let delegate = Pubkey::new_unique();
    tx.message.account_keys.push(delegate);

    // Freeze + Approve combo
    let freeze_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 2, 0],
        data: vec![FREEZE_ACCOUNT],
    };
    tx.message.instructions.push(freeze_ix);

    let approve_ix = create_approve_instruction(100, true);
    tx.message.instructions.push(approve_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["dangerous_operation_combo"], json!(true));
}

#[tokio::test]
async fn test_dangerous_operation_combo_mint_authority() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    // Mint + SetAuthority combo
    let mut mint_data = vec![MINT_TO];
    mint_data.extend_from_slice(&1000u64.to_le_bytes());

    let mint_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 2, 0],
        data: mint_data,
    };
    tx.message.instructions.push(mint_ix);

    let mut auth_data = vec![SET_AUTHORITY];
    auth_data.push(0);
    auth_data.push(0);

    let set_auth_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 0],
        data: auth_data,
    };
    tx.message.instructions.push(set_auth_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["dangerous_operation_combo"], json!(true));
}

#[tokio::test]
async fn test_token_2022_program() {
    let analyzer = TokenInstructionAnalyzer::new();
    let payer = Pubkey::new_unique();
    let token_2022 = Pubkey::from_str(TOKEN_2022_PROGRAM).unwrap();
    let dest = Pubkey::new_unique();

    let mut tx = Transaction {
        signatures: vec![Signature::default()],
        message: Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec![payer, token_2022, dest],
            recent_blockhash: Hash::default(),
            instructions: vec![],
        },
    };

    // Transfer using Token-2022
    let transfer_ix = create_transfer_instruction(200, true);
    tx.message.instructions.push(transfer_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["has_transfer"], json!(true));
    assert_eq!(result["transfer_count"], json!(1));
    assert_eq!(result["total_transfer_amount"], json!(200));
}

#[tokio::test]
async fn test_complex_transaction() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    // Add accounts
    for _ in 0..5 {
        tx.message.account_keys.push(Pubkey::new_unique());
    }

    // 2 transfers
    let mut transfer1 = create_transfer_instruction(100, true);
    transfer1.accounts[1] = 2;
    tx.message.instructions.push(transfer1);

    let mut transfer2 = create_transfer_instruction(200, true);
    transfer2.accounts[1] = 3;
    tx.message.instructions.push(transfer2);

    // 1 approve
    let mut approve_ix = create_approve_instruction(500, true);
    approve_ix.accounts[1] = 4;
    tx.message.instructions.push(approve_ix);

    // 1 burn
    let mut burn_data = vec![BURN];
    burn_data.extend_from_slice(&50u64.to_le_bytes());
    let burn_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 5, 0],
        data: burn_data,
    };
    tx.message.instructions.push(burn_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    assert_eq!(result["transfer_count"], json!(2));
    assert_eq!(result["total_transfer_amount"], json!(300));
    assert_eq!(result["approve_count"], json!(1));
    assert_eq!(result["burn_count"], json!(1));
    assert_eq!(result["modifies_supply"], json!(true));
    assert_eq!(result["modifies_permissions"], json!(true));
}

#[tokio::test]
async fn test_estimated_latency() {
    let analyzer = TokenInstructionAnalyzer::new();
    assert_eq!(analyzer.estimated_latency_ms(), 1);
}

#[tokio::test]
async fn test_default_constructor() {
    let analyzer = TokenInstructionAnalyzer;
    assert_eq!(analyzer.name(), "token_instructions");
}

#[tokio::test]
async fn test_empty_instruction_data() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    // Token instruction with empty data
    let empty_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![],
        data: vec![],
    };
    tx.message.instructions.push(empty_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    // Should not crash
    assert_eq!(result["has_transfer"], json!(false));
}

#[tokio::test]
async fn test_short_instruction_data() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    // Transfer with incomplete amount data
    let short_ix = CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0, 2, 0],
        data: vec![TRANSFER, 0, 0], // Only 3 bytes, needs 9
    };
    tx.message.instructions.push(short_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    // Should detect transfer but not parse amount
    assert_eq!(result["has_transfer"], json!(true));
    assert_eq!(result["total_transfer_amount"], json!(0)); // Amount parse failed
}

#[tokio::test]
async fn test_invalid_account_index() {
    let analyzer = TokenInstructionAnalyzer::new();
    let mut tx = create_token_transaction();

    // Transfer with invalid destination index
    let mut transfer_ix = create_transfer_instruction(100, true);
    transfer_ix.accounts[1] = 99; // Out of bounds
    tx.message.instructions.push(transfer_ix);

    let result = analyzer.analyze(&tx).await.unwrap();

    // Should detect transfer but not add recipient
    assert_eq!(result["has_transfer"], json!(true));
    let recipients = result["transfer_recipients"].as_array().unwrap();
    assert_eq!(recipients.len(), 0); // No recipient because index was invalid
}
