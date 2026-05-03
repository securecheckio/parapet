use parapet_core::rules::analyzers::*;
use parapet_core::rules::types::RuleDefinition;
use parapet_core::rules::{AnalyzerRegistry, RuleEngine};
use solana_sdk::{
    message::Message, pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction,
};
use solana_sdk_ids::system_program;
use solana_system_interface::instruction as system_instruction;
use std::sync::Arc;

fn create_test_registry() -> AnalyzerRegistry {
    let mut registry = AnalyzerRegistry::new();
    registry.register(Arc::new(BasicAnalyzer::new()));
    registry.register(Arc::new(TokenInstructionAnalyzer::new()));
    registry.register(Arc::new(SystemProgramAnalyzer::new()));
    registry
}

/// Test AI agent velocity limiting
#[tokio::test]
async fn test_ai_agent_velocity_limit() {
    // Load AI agent protection rules
    let rules_json = include_str!("../tests/fixtures/rules/presets/ai-agent-protection.json");
    let rules: Vec<RuleDefinition> = serde_json::from_str(rules_json).unwrap();

    let registry = create_test_registry();
    let mut engine = RuleEngine::new(registry).with_flowstate(None);
    engine.load_rules(rules).unwrap();

    let agent_keypair = Keypair::new();
    let recipient = Pubkey::new_unique();

    // Send 9 transactions - should all pass
    for i in 0..9 {
        let ix = system_instruction::transfer(&agent_keypair.pubkey(), &recipient, 1000);
        let message = Message::new(&[ix], Some(&agent_keypair.pubkey()));
        let mut tx = Transaction::new_unsigned(message);
        tx.sign(&[&agent_keypair], solana_sdk::hash::Hash::default());

        let decision = engine.evaluate(&tx).await.unwrap();
        assert_ne!(
            decision.action,
            parapet_core::rules::types::RuleAction::Block,
            "Transaction {} should not be blocked",
            i + 1
        );
    }

    // 10th transaction should be blocked (velocity limit)
    let ix = system_instruction::transfer(&agent_keypair.pubkey(), &recipient, 1000);
    let message = Message::new(&[ix], Some(&agent_keypair.pubkey()));
    let mut tx = Transaction::new_unsigned(message);
    tx.sign(&[&agent_keypair], solana_sdk::hash::Hash::default());

    let decision = engine.evaluate(&tx).await.unwrap();
    assert_eq!(
        decision.action,
        parapet_core::rules::types::RuleAction::Block
    );
    assert!(decision.message.contains("velocity"));
}

/// Test AI agent account creation spam detection
#[tokio::test]
async fn test_ai_agent_account_spam() {
    let rules_json = include_str!("../tests/fixtures/rules/presets/ai-agent-protection.json");
    let rules: Vec<RuleDefinition> = serde_json::from_str(rules_json).unwrap();

    let registry = create_test_registry();
    let mut engine = RuleEngine::new(registry).with_flowstate(None);
    engine.load_rules(rules).unwrap();

    let agent_keypair = Keypair::new();

    // Create 4 accounts - should all pass
    for i in 0..4 {
        let new_account = Keypair::new();
        let ix = system_instruction::create_account(
            &agent_keypair.pubkey(),
            &new_account.pubkey(),
            1_000_000,
            0,
            &system_program::id(),
        );
        let message = Message::new(&[ix], Some(&agent_keypair.pubkey()));
        let mut tx = Transaction::new_unsigned(message);
        tx.sign(
            &[&agent_keypair, &new_account],
            solana_sdk::hash::Hash::default(),
        );

        let decision = engine.evaluate(&tx).await.unwrap();
        assert_ne!(
            decision.action,
            parapet_core::rules::types::RuleAction::Block,
            "Account creation {} should not be blocked",
            i + 1
        );
    }

    // 5th account creation should be blocked (spam detection)
    let new_account = Keypair::new();
    let ix = system_instruction::create_account(
        &agent_keypair.pubkey(),
        &new_account.pubkey(),
        1_000_000,
        0,
        &system_program::id(),
    );
    let message = Message::new(&[ix], Some(&agent_keypair.pubkey()));
    let mut tx = Transaction::new_unsigned(message);
    tx.sign(
        &[&agent_keypair, &new_account],
        solana_sdk::hash::Hash::default(),
    );

    let decision = engine.evaluate(&tx).await.unwrap();
    assert_eq!(
        decision.action,
        parapet_core::rules::types::RuleAction::Block
    );
    assert!(decision.message.contains("spam") || decision.message.contains("spike"));
}

/// Test enterprise lateral movement detection (cross-wallet)
#[tokio::test]
async fn test_enterprise_lateral_movement() {
    let rules_json = include_str!("../tests/fixtures/rules/presets/enterprise-cross-wallet.json");
    let rules: Vec<RuleDefinition> = serde_json::from_str(rules_json).unwrap();

    let registry = create_test_registry();
    let mut engine = RuleEngine::new(registry).with_flowstate(None);
    engine.load_rules(rules).unwrap();

    // Simulate 3 different wallets sending to same recipient
    let wallet1 = Keypair::new();
    let wallet2 = Keypair::new();
    let wallet3 = Keypair::new();
    let suspicious_recipient = Pubkey::new_unique();

    // Wallet 1 sends - should pass
    let ix1 = system_instruction::transfer(&wallet1.pubkey(), &suspicious_recipient, 1_000_000);
    let message1 = Message::new(&[ix1], Some(&wallet1.pubkey()));
    let mut tx1 = Transaction::new_unsigned(message1);
    tx1.sign(&[&wallet1], solana_sdk::hash::Hash::default());

    let decision1 = engine.evaluate(&tx1).await.unwrap();
    assert_ne!(
        decision1.action,
        parapet_core::rules::types::RuleAction::Block,
        "First transfer should not be blocked"
    );

    // Wallet 2 sends - should pass
    let ix2 = system_instruction::transfer(&wallet2.pubkey(), &suspicious_recipient, 1_000_000);
    let message2 = Message::new(&[ix2], Some(&wallet2.pubkey()));
    let mut tx2 = Transaction::new_unsigned(message2);
    tx2.sign(&[&wallet2], solana_sdk::hash::Hash::default());

    let decision2 = engine.evaluate(&tx2).await.unwrap();
    assert_ne!(
        decision2.action,
        parapet_core::rules::types::RuleAction::Block,
        "Second transfer should not be blocked"
    );

    // Wallet 3 sends - should be BLOCKED (lateral movement detected)
    let ix3 = system_instruction::transfer(&wallet3.pubkey(), &suspicious_recipient, 1_000_000);
    let message3 = Message::new(&[ix3], Some(&wallet3.pubkey()));
    let mut tx3 = Transaction::new_unsigned(message3);
    tx3.sign(&[&wallet3], solana_sdk::hash::Hash::default());

    let decision3 = engine.evaluate(&tx3).await.unwrap();
    assert_eq!(
        decision3.action,
        parapet_core::rules::types::RuleAction::Block,
        "Third transfer should be blocked (lateral movement)"
    );
    assert!(
        decision3.message.contains("Lateral movement") || decision3.message.contains("lateral")
    );
}

/// Test AI agent gradual exfiltration detection
#[tokio::test]
async fn test_ai_agent_gradual_exfiltration() {
    let rules_json = include_str!("../tests/fixtures/rules/presets/ai-agent-advanced.json");
    let rules: Vec<RuleDefinition> = serde_json::from_str(rules_json).unwrap();

    let registry = create_test_registry();
    let mut engine = RuleEngine::new(registry).with_flowstate(None);
    engine.load_rules(rules).unwrap();

    let agent_keypair = Keypair::new();
    let attacker_wallet = Pubkey::new_unique();

    // Send 3 transfers to same recipient - should all pass
    for i in 0..3 {
        let ix = system_instruction::transfer(&agent_keypair.pubkey(), &attacker_wallet, 100_000);
        let message = Message::new(&[ix], Some(&agent_keypair.pubkey()));
        let mut tx = Transaction::new_unsigned(message);
        tx.sign(&[&agent_keypair], solana_sdk::hash::Hash::default());

        let decision = engine.evaluate(&tx).await.unwrap();
        assert_ne!(
            decision.action,
            parapet_core::rules::types::RuleAction::Block,
            "Transfer {} should not be blocked",
            i + 1
        );
    }

    // 4th transfer to same recipient should be blocked (exfiltration)
    let ix = system_instruction::transfer(&agent_keypair.pubkey(), &attacker_wallet, 100_000);
    let message = Message::new(&[ix], Some(&agent_keypair.pubkey()));
    let mut tx = Transaction::new_unsigned(message);
    tx.sign(&[&agent_keypair], solana_sdk::hash::Hash::default());

    let decision = engine.evaluate(&tx).await.unwrap();
    assert_eq!(
        decision.action,
        parapet_core::rules::types::RuleAction::Block
    );
    assert!(
        decision.message.contains("exfiltration")
            || decision.message.contains("Repeated transfers")
    );
}

/// Test repeated block detection
#[tokio::test]
async fn test_repeated_block_detection() {
    let rules_json = include_str!("../tests/fixtures/rules/presets/ai-agent-protection.json");
    let rules: Vec<RuleDefinition> = serde_json::from_str(rules_json).unwrap();

    let registry = create_test_registry();
    let mut engine = RuleEngine::new(registry).with_flowstate(None);
    engine.load_rules(rules).unwrap();

    let agent_keypair = Keypair::new();
    let recipient = Pubkey::new_unique();

    // Trigger velocity limit by sending 10 transactions
    for _ in 0..10 {
        let ix = system_instruction::transfer(&agent_keypair.pubkey(), &recipient, 1000);
        let message = Message::new(&[ix], Some(&agent_keypair.pubkey()));
        let mut tx = Transaction::new_unsigned(message);
        tx.sign(&[&agent_keypair], solana_sdk::hash::Hash::default());

        let _ = engine.evaluate(&tx).await.unwrap();
    }

    // Next 2 transactions should be blocked (velocity limit)
    // This increments blocked_transaction_count to 2
    for _ in 0..2 {
        let ix = system_instruction::transfer(&agent_keypair.pubkey(), &recipient, 1000);
        let message = Message::new(&[ix], Some(&agent_keypair.pubkey()));
        let mut tx = Transaction::new_unsigned(message);
        tx.sign(&[&agent_keypair], solana_sdk::hash::Hash::default());

        let decision = engine.evaluate(&tx).await.unwrap();
        assert_eq!(
            decision.action,
            parapet_core::rules::types::RuleAction::Block
        );
    }

    // 3rd blocked transaction should trigger repeated block alert
    let ix = system_instruction::transfer(&agent_keypair.pubkey(), &recipient, 1000);
    let message = Message::new(&[ix], Some(&agent_keypair.pubkey()));
    let mut tx = Transaction::new_unsigned(message);
    tx.sign(&[&agent_keypair], solana_sdk::hash::Hash::default());

    let decision = engine.evaluate(&tx).await.unwrap();
    // Should still be blocked, but also trigger alert
    assert_eq!(
        decision.action,
        parapet_core::rules::types::RuleAction::Block
    );
}
