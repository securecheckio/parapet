use anyhow::Result;
use parapet_api::state::{AppState, Config};
use redis::AsyncCommands;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction as SolanaTransaction,
};

/// Test setup helper
async fn setup_test_state() -> Result<AppState> {
    let config = Config {
        server_host: "127.0.0.1".to_string(),
        server_port: 3001,
        worker_threads: None,
        max_concurrent_scans: 2,
        scans_per_hour_per_key: 10,
        redis_url: "redis://localhost:6379".to_string(),
        solana_rpc_url: "https://api.devnet.solana.com".to_string(),
        solana_network: "devnet".to_string(),
        authorized_wallets: vec!["test_wallet".to_string()],
        nonce_ttl_seconds: 300,
        mcp_api_keys: vec!["test_key".to_string()],
    };

    AppState::new(config).await
}

#[tokio::test]
async fn test_nonce_generation() -> Result<()> {
    let state = match setup_test_state().await {
        Ok(s) => s,
        Err(_) => {
            println!("⚠️  Skipping test: Redis not available");
            return Ok(());
        }
    };

    let Some(ref redis_conn) = state.redis.as_ref() else {
        println!("⚠️  Skipping test: Redis not available");
        return Ok(());
    };
    let mut redis = redis_conn.clone();

    // Generate nonce
    let wallet = "test_wallet";
    let nonce_key = format!("nonce:{}:test_nonce", wallet);

    // Store test nonce
    redis.set_ex::<_, _, ()>(&nonce_key, "1", 300).await?;

    // Verify nonce exists
    let exists: bool = redis.exists(&nonce_key).await?;
    assert!(exists, "Nonce should exist in Redis");

    // Clean up
    redis.del::<_, ()>(&nonce_key).await?;

    Ok(())
}

#[tokio::test]
async fn test_escalation_lifecycle() -> Result<()> {
    let state = match setup_test_state().await {
        Ok(s) => s,
        Err(_) => {
            println!("⚠️  Skipping test: Redis not available");
            return Ok(());
        }
    };

    let Some(ref redis_conn) = state.redis.as_ref() else {
        println!("⚠️  Skipping test: Redis not available");
        return Ok(());
    };
    let mut redis = redis_conn.clone();

    // Create test escalation
    let escalation_id = "esc_test_123";
    let escalation = serde_json::json!({
        "escalation_id": escalation_id,
        "canonical_hash": "test_hash",
        "requester_wallet": "requester_wallet",
        "approver_wallet": "approver_wallet",
        "risk_score": 75,
        "warnings": ["Test warning"],
        "decoded_instructions": [],
        "suggested_rules": [],
        "status": "pending",
        "created_at": 1234567890u64,
        "expires_at": 1234567890u64 + 300,
    });

    // Store escalation
    let escalation_key = format!("escalation:pending:{}", escalation_id);
    redis
        .set_ex::<_, _, ()>(&escalation_key, serde_json::to_string(&escalation)?, 300)
        .await?;

    // Verify escalation exists
    let stored: String = redis.get(&escalation_key).await?;
    let parsed: serde_json::Value = serde_json::from_str(&stored)?;
    assert_eq!(parsed["escalation_id"], escalation_id);
    assert_eq!(parsed["status"], "pending");

    // Update status to approved
    let mut escalation_obj: serde_json::Value = serde_json::from_str(&stored)?;
    escalation_obj["status"] = serde_json::json!("approved");
    redis
        .set_ex::<_, _, ()>(
            &escalation_key,
            serde_json::to_string(&escalation_obj)?,
            300,
        )
        .await?;

    // Verify update
    let updated: String = redis.get(&escalation_key).await?;
    let parsed_updated: serde_json::Value = serde_json::from_str(&updated)?;
    assert_eq!(parsed_updated["status"], "approved");

    // Clean up
    redis.del::<_, ()>(&escalation_key).await?;

    Ok(())
}

#[tokio::test]
async fn test_dynamic_rule_creation() -> Result<()> {
    let state = match setup_test_state().await {
        Ok(s) => s,
        Err(_) => {
            println!("⚠️  Skipping test: Redis not available");
            return Ok(());
        }
    };

    let Some(ref redis_conn) = state.redis.as_ref() else {
        println!("⚠️  Skipping test: Redis not available");
        return Ok(());
    };
    let mut redis = redis_conn.clone();

    // Create test rule
    let rule_id = "rule_test_123";
    let rule = serde_json::json!({
        "id": rule_id,
        "name": "Test Rule",
        "action": "pass",
        "conditions": {
            "canonical_transaction_hash": "test_hash"
        },
        "priority": 100,
        "metadata": {}
    });

    // Store rule
    let rule_key = format!("dynamic_rule:{}", rule_id);
    redis
        .set_ex::<_, _, ()>(&rule_key, serde_json::to_string(&rule)?, 3600)
        .await?;

    // Publish update event
    let channel = "dynamic_rules:updated";
    redis.publish::<_, _, ()>(channel, rule_id).await?;

    // Verify rule exists
    let stored: String = redis.get(&rule_key).await?;
    let parsed: serde_json::Value = serde_json::from_str(&stored)?;
    assert_eq!(parsed["id"], rule_id);
    assert_eq!(parsed["action"], "pass");

    // Clean up
    redis.del::<_, ()>(&rule_key).await?;

    Ok(())
}

#[tokio::test]
async fn test_pending_transaction_storage() -> Result<()> {
    let state = match setup_test_state().await {
        Ok(s) => s,
        Err(_) => {
            println!("⚠️  Skipping test: Redis not available");
            return Ok(());
        }
    };

    let Some(ref redis_conn) = state.redis.as_ref() else {
        println!("⚠️  Skipping test: Redis not available");
        return Ok(());
    };
    let mut redis = redis_conn.clone();

    // Create test transaction
    let escalation_id = "esc_test_tx_123";
    let tx_bytes = vec![1, 2, 3, 4, 5]; // Mock transaction bytes

    // Store transaction for fast-path
    let tx_key = format!("pending_tx:{}", escalation_id);
    redis.set_ex::<_, _, ()>(&tx_key, &tx_bytes, 50).await?;

    // Verify transaction exists
    let stored_bytes: Vec<u8> = redis.get(&tx_key).await?;
    assert_eq!(stored_bytes, tx_bytes);

    // Verify TTL is set
    let ttl: isize = redis.ttl(&tx_key).await?;
    assert!(
        ttl > 0 && ttl <= 50,
        "TTL should be between 1 and 50 seconds"
    );

    // Clean up
    redis.del::<_, ()>(&tx_key).await?;

    Ok(())
}

#[tokio::test]
async fn test_approver_pending_set() -> Result<()> {
    let state = match setup_test_state().await {
        Ok(s) => s,
        Err(_) => {
            println!("⚠️  Skipping test: Redis not available");
            return Ok(());
        }
    };

    let Some(ref redis_conn) = state.redis.as_ref() else {
        println!("⚠️  Skipping test: Redis not available");
        return Ok(());
    };
    let mut redis = redis_conn.clone();

    let approver_wallet = "test_approver";
    let escalation_ids = vec!["esc_1", "esc_2", "esc_3"];

    // Add escalations to pending set
    let approver_key = format!("escalation:pending:approver:{}", approver_wallet);
    for id in &escalation_ids {
        redis.sadd::<_, _, ()>(&approver_key, id).await?;
    }
    redis.expire::<_, ()>(&approver_key, 300).await?;

    // Verify all escalations are in set
    let members: Vec<String> = redis.smembers(&approver_key).await?;
    assert_eq!(members.len(), escalation_ids.len());

    for id in &escalation_ids {
        assert!(members.contains(&id.to_string()));
    }

    // Remove one escalation
    redis.srem::<_, _, ()>(&approver_key, "esc_2").await?;

    // Verify removal
    let updated_members: Vec<String> = redis.smembers(&approver_key).await?;
    assert_eq!(updated_members.len(), 2);
    assert!(!updated_members.contains(&"esc_2".to_string()));

    // Clean up
    redis.del::<_, ()>(&approver_key).await?;

    Ok(())
}

#[tokio::test]
async fn test_websocket_event_publish() -> Result<()> {
    let state = match setup_test_state().await {
        Ok(s) => s,
        Err(_) => {
            println!("⚠️  Skipping test: Redis not available");
            return Ok(());
        }
    };

    let Some(ref redis_conn) = state.redis.as_ref() else {
        println!("⚠️  Skipping test: Redis not available");
        return Ok(());
    };
    let mut redis = redis_conn.clone();

    let approver_wallet = "test_approver";
    let channel = format!("escalation:events:{}", approver_wallet);

    let event = serde_json::json!({
        "type": "escalation_created",
        "escalation": {
            "escalation_id": "esc_test",
            "status": "pending"
        }
    });

    // Publish event
    let subscribers: usize = redis
        .publish(&channel, serde_json::to_string(&event)?)
        .await?;

    // No WebSocket clients are connected in this test path.
    assert_eq!(subscribers, 0);

    Ok(())
}

#[cfg(test)]
mod canonical_hash_tests {
    use super::*;
    use parapet_core::rules::analyzers::core::CanonicalTransactionAnalyzer;
    use solana_sdk::{
        instruction::CompiledInstruction, message::Message, pubkey::Pubkey, signature::Keypair,
    };
    use solana_system_interface::instruction as system_instruction;

    #[test]
    fn test_canonical_hash_determinism() {
        let keypair1 = Keypair::new();
        let keypair2 = Keypair::new();

        // Create same transaction with different blockhashes
        let ix = system_instruction::transfer(&keypair1.pubkey(), &keypair2.pubkey(), 1000);

        let message1 = Message::new(&[ix.clone()], Some(&keypair1.pubkey()));
        let mut tx1 = SolanaTransaction::new_unsigned(message1);
        tx1.message.recent_blockhash = solana_sdk::hash::Hash::new_unique();
        tx1.sign(&[&keypair1], tx1.message.recent_blockhash);

        let message2 = Message::new(&[ix], Some(&keypair1.pubkey()));
        let mut tx2 = SolanaTransaction::new_unsigned(message2);
        tx2.message.recent_blockhash = solana_sdk::hash::Hash::new_unique(); // Different blockhash
        tx2.sign(&[&keypair1], tx2.message.recent_blockhash);

        // Compute canonical hashes
        let hash1 = CanonicalTransactionAnalyzer::compute_canonical_hash(&tx1).unwrap();
        let hash2 = CanonicalTransactionAnalyzer::compute_canonical_hash(&tx2).unwrap();

        // Hashes should be identical despite different blockhashes
        assert_eq!(
            hash1, hash2,
            "Canonical hashes should be identical for same transaction logic"
        );
    }

    #[test]
    fn test_canonical_hash_different_instructions() {
        let keypair1 = Keypair::new();
        let keypair2 = Keypair::new();
        let keypair3 = Keypair::new();

        // Create different transactions
        let ix1 = system_instruction::transfer(&keypair1.pubkey(), &keypair2.pubkey(), 1000);
        let ix2 = system_instruction::transfer(&keypair1.pubkey(), &keypair3.pubkey(), 1000); // Different recipient

        let message1 = Message::new(&[ix1], Some(&keypair1.pubkey()));
        let mut tx1 = SolanaTransaction::new_unsigned(message1);
        tx1.message.recent_blockhash = solana_sdk::hash::Hash::new_unique();
        tx1.sign(&[&keypair1], tx1.message.recent_blockhash);

        let message2 = Message::new(&[ix2], Some(&keypair1.pubkey()));
        let mut tx2 = SolanaTransaction::new_unsigned(message2);
        tx2.message.recent_blockhash = tx1.message.recent_blockhash; // Same blockhash
        tx2.sign(&[&keypair1], tx2.message.recent_blockhash);

        // Compute canonical hashes
        let hash1 = CanonicalTransactionAnalyzer::compute_canonical_hash(&tx1).unwrap();
        let hash2 = CanonicalTransactionAnalyzer::compute_canonical_hash(&tx2).unwrap();

        // Hashes should be different for different instructions
        assert_ne!(
            hash1, hash2,
            "Canonical hashes should differ for different transaction logic"
        );
    }
}
