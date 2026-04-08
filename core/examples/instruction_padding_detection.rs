/// Example: Instruction Padding Attack Detection
///
/// This example demonstrates how to use the InstructionPaddingAnalyzer to detect
/// suspicious padding in Solana transaction instructions.
///
/// Run with:
/// ```bash
/// cargo run --example instruction_padding_detection
/// ```

use parapet_core::rules::analyzer::TransactionAnalyzer;
use parapet_core::rules::analyzers::core::InstructionPaddingAnalyzer;
use solana_sdk::instruction::CompiledInstruction;
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;

const SPL_TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("=== Instruction Padding Attack Detection Examples ===\n");

    let analyzer = InstructionPaddingAnalyzer::new();

    // Example 1: Normal SPL Token Transfer (no padding)
    println!("Example 1: Normal SPL Token Transfer");
    println!("-------------------------------------");
    let normal_tx = create_normal_transfer();
    let result = analyzer.analyze(&normal_tx).await.unwrap();
    print_result(&result);
    println!();

    // Example 2: Token-2022 with Extensions (legitimate padding)
    println!("Example 2: Token-2022 with Extensions (Legitimate)");
    println!("---------------------------------------------------");
    let token2022_tx = create_token2022_with_extensions();
    let result = analyzer.analyze(&token2022_tx).await.unwrap();
    print_result(&result);
    println!();

    // Example 3: Malicious padding attack (excessive padding)
    println!("Example 3: Malicious Padding Attack (Excessive)");
    println!("-----------------------------------------------");
    let attack_tx = create_padding_attack();
    let result = analyzer.analyze(&attack_tx).await.unwrap();
    print_result(&result);
    println!();

    // Example 4: Obfuscation attack (repeated pattern)
    println!("Example 4: Obfuscation Attack (Repeated Pattern)");
    println!("------------------------------------------------");
    let obfuscation_tx = create_obfuscation_attack();
    let result = analyzer.analyze(&obfuscation_tx).await.unwrap();
    print_result(&result);
}

fn create_normal_transfer() -> Transaction {
    // Normal SPL Token Transfer: discriminator (1) + amount (8) = 9 bytes
    let mut data = vec![3]; // Transfer discriminator
    data.extend_from_slice(&1000u64.to_le_bytes()); // amount

    let token_program = SPL_TOKEN_PROGRAM.parse::<Pubkey>().unwrap();
    let ix = CompiledInstruction {
        program_id_index: 0,
        accounts: vec![0, 1, 2],
        data,
    };

    let message = Message {
        header: solana_sdk::message::MessageHeader::default(),
        account_keys: vec![token_program],
        recent_blockhash: solana_sdk::hash::Hash::default(),
        instructions: vec![ix],
    };

    Transaction {
        signatures: vec![],
        message,
    }
}

fn create_token2022_with_extensions() -> Transaction {
    // Token-2022 Transfer with TransferFee extension (~108 bytes)
    let mut data = vec![3]; // Transfer discriminator
    data.extend_from_slice(&1000u64.to_le_bytes()); // amount
    
    // Simulate TransferFee extension data
    data.extend_from_slice(&vec![1u8; 108]); // TransferFee extension

    let token_program = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
        .parse::<Pubkey>()
        .unwrap();
    let ix = CompiledInstruction {
        program_id_index: 0,
        accounts: vec![0, 1, 2],
        data,
    };

    let message = Message {
        header: solana_sdk::message::MessageHeader::default(),
        account_keys: vec![token_program],
        recent_blockhash: solana_sdk::hash::Hash::default(),
        instructions: vec![ix],
    };

    Transaction {
        signatures: vec![],
        message,
    }
}

fn create_padding_attack() -> Transaction {
    // Malicious SPL Token Transfer with excessive padding (> 512 bytes)
    let mut data = vec![3]; // Transfer discriminator
    data.extend_from_slice(&u64::MAX.to_le_bytes()); // unlimited approval amount
    data.extend_from_slice(&vec![0xAB; 1000]); // excessive padding to hide malicious intent

    let token_program = SPL_TOKEN_PROGRAM.parse::<Pubkey>().unwrap();
    let ix = CompiledInstruction {
        program_id_index: 0,
        accounts: vec![0, 1, 2],
        data,
    };

    let message = Message {
        header: solana_sdk::message::MessageHeader::default(),
        account_keys: vec![token_program],
        recent_blockhash: solana_sdk::hash::Hash::default(),
        instructions: vec![ix],
    };

    Transaction {
        signatures: vec![],
        message,
    }
}

fn create_obfuscation_attack() -> Transaction {
    // Malicious transaction with repeated null bytes (obfuscation pattern)
    let mut data = vec![4]; // Approve discriminator
    data.extend_from_slice(&u64::MAX.to_le_bytes()); // unlimited approval
    data.extend_from_slice(&vec![0x00; 600]); // repeated nulls - highly suspicious

    let token_program = SPL_TOKEN_PROGRAM.parse::<Pubkey>().unwrap();
    let ix = CompiledInstruction {
        program_id_index: 0,
        accounts: vec![0, 1, 2],
        data,
    };

    let message = Message {
        header: solana_sdk::message::MessageHeader::default(),
        account_keys: vec![token_program],
        recent_blockhash: solana_sdk::hash::Hash::default(),
        instructions: vec![ix],
    };

    Transaction {
        signatures: vec![],
        message,
    }
}

fn print_result(result: &std::collections::HashMap<String, serde_json::Value>) {
    use serde_json::Value;

    let has_suspicious = result
        .get("has_suspicious_padding")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if has_suspicious {
        println!("⚠️  SUSPICIOUS PADDING DETECTED!");
        println!(
            "   Suspicious instructions: {}",
            result
                .get("suspicious_instruction_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
        );
        println!(
            "   Max padding bytes: {}",
            result
                .get("max_padding_bytes")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
        );
        println!(
            "   Max padding ratio: {:.1}x",
            result
                .get("max_padding_ratio")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
        );
        println!(
            "   Has repeated pattern: {}",
            result
                .get("has_repeated_padding")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        );

        if let Some(Value::Array(suspicious)) = result.get("suspicious_instructions") {
            println!("\n   Details:");
            for (i, inst) in suspicious.iter().enumerate() {
                println!("   Instruction {}:", i + 1);
                if let Some(Value::String(itype)) = inst.get("instruction_type") {
                    println!("     Type: {}", itype);
                }
                if let Some(Value::Number(expected)) = inst.get("expected_size") {
                    println!("     Expected size: {}", expected);
                }
                if let Some(Value::Number(actual)) = inst.get("actual_size") {
                    println!("     Actual size: {}", actual);
                }
                if let Some(Value::String(reason)) = inst.get("reason") {
                    println!("     Reason: {}", reason);
                }
            }
        }
    } else {
        println!("✅ No suspicious padding detected");
        println!("   Transaction appears normal");
    }
}
