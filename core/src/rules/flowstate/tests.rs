use super::state::FlowStateManager;
use solana_sdk::pubkey::Pubkey;
use std::time::Duration;

#[test]
fn test_transaction_count_tracking() {
    let mut manager = FlowStateManager::new(None);
    let wallet = Pubkey::new_unique();

    // Simulate 5 transactions
    for _ in 0..5 {
        manager.increment(&wallet, "transaction_count", Some(Duration::from_secs(600)));
    }

    assert_eq!(manager.get_counter(&wallet, "transaction_count"), 5);
}

#[test]
fn test_transaction_velocity_limit() {
    let mut manager = FlowStateManager::new(None);
    let wallet = Pubkey::new_unique();

    // Simulate 10 transactions (at velocity limit)
    for _ in 0..10 {
        manager.increment(&wallet, "transaction_count", Some(Duration::from_secs(600)));
    }

    let count = manager.get_counter(&wallet, "transaction_count");
    assert_eq!(count, 10);
    assert!(count >= 10, "Should trigger velocity limit");
}

#[test]
fn test_account_creation_spam_detection() {
    let mut manager = FlowStateManager::new(None);
    let wallet = Pubkey::new_unique();

    // Simulate 5 account creations (at spam threshold)
    for _ in 0..5 {
        manager.increment(
            &wallet,
            "account_creation_count",
            Some(Duration::from_secs(300)),
        );
    }

    let count = manager.get_counter(&wallet, "account_creation_count");
    assert_eq!(count, 5);
    assert!(count >= 5, "Should trigger spam detection");
}

#[test]
fn test_blocked_transaction_count() {
    let mut manager = FlowStateManager::new(None);
    let wallet = Pubkey::new_unique();

    // Simulate 3 blocked transactions
    for _ in 0..3 {
        manager.increment(
            &wallet,
            "blocked_transaction_count",
            Some(Duration::from_secs(3600)),
        );
    }

    let count = manager.get_counter(&wallet, "blocked_transaction_count");
    assert_eq!(count, 3);
    assert!(count >= 3, "Should trigger repeated block alert");
}

#[test]
fn test_counter_expiration_after_ttl() {
    let mut manager = FlowStateManager::new(None);
    let wallet = Pubkey::new_unique();

    // Set counter with very short TTL
    manager.increment(
        &wallet,
        "transaction_count",
        Some(Duration::from_millis(10)),
    );
    assert_eq!(manager.get_counter(&wallet, "transaction_count"), 1);

    // Wait for expiration
    std::thread::sleep(Duration::from_millis(20));

    // Counter should be expired (returns 0)
    assert_eq!(manager.get_counter(&wallet, "transaction_count"), 0);
}

#[test]
fn test_multiple_wallets_independent_counters() {
    let mut manager = FlowStateManager::new(None);
    let wallet1 = Pubkey::new_unique();
    let wallet2 = Pubkey::new_unique();

    // Wallet 1: 3 transactions
    for _ in 0..3 {
        manager.increment(
            &wallet1,
            "transaction_count",
            Some(Duration::from_secs(600)),
        );
    }

    // Wallet 2: 7 transactions
    for _ in 0..7 {
        manager.increment(
            &wallet2,
            "transaction_count",
            Some(Duration::from_secs(600)),
        );
    }

    assert_eq!(manager.get_counter(&wallet1, "transaction_count"), 3);
    assert_eq!(manager.get_counter(&wallet2, "transaction_count"), 7);
}

#[test]
fn test_nonce_tracking() {
    let mut manager = FlowStateManager::new(None);
    let wallet = Pubkey::new_unique();

    // Track nonce usage
    manager.set(
        &wallet,
        "nonce_used_recently",
        Some(Duration::from_secs(86400)),
    );

    assert!(manager.is_set(&wallet, "nonce_used_recently"));
}

#[test]
fn test_pass_action_tracker_pattern() {
    let mut manager = FlowStateManager::new(None);
    let wallet = Pubkey::new_unique();

    // Simulate "pass" action tracker incrementing counter
    manager.increment(&wallet, "transaction_count", Some(Duration::from_secs(600)));
    manager.increment(&wallet, "transaction_count", Some(Duration::from_secs(600)));
    manager.increment(&wallet, "transaction_count", Some(Duration::from_secs(600)));

    // Simulate detection rule checking counter
    let count = manager.get_counter(&wallet, "transaction_count");
    assert_eq!(count, 3);

    // Detection rule would evaluate: count >= 10 ? false (allow)
    assert!(count < 10, "Should not trigger block yet");
}

#[test]
fn test_memory_limit_with_ai_agent() {
    let mut manager = FlowStateManager::new(Some(1)); // Max 1 wallet (AI agent)

    let wallet1 = Pubkey::new_unique();
    let wallet2 = Pubkey::new_unique();

    // Add first wallet
    manager.increment(
        &wallet1,
        "transaction_count",
        Some(Duration::from_secs(600)),
    );
    assert_eq!(manager.get_counter(&wallet1, "transaction_count"), 1);

    // Add second wallet - should evict first
    manager.increment(
        &wallet2,
        "transaction_count",
        Some(Duration::from_secs(600)),
    );
    assert_eq!(manager.get_counter(&wallet2, "transaction_count"), 1);
    assert_eq!(manager.get_counter(&wallet1, "transaction_count"), 0); // Evicted
}

#[test]
fn test_cleanup_removes_expired_flowstate() {
    let mut manager = FlowStateManager::new(None);
    let wallet = Pubkey::new_unique();

    // Set flowstate with short TTL
    manager.increment(&wallet, "short_lived", Some(Duration::from_millis(10)));
    assert_eq!(manager.get_counter(&wallet, "short_lived"), 1);

    // Wait for expiration
    std::thread::sleep(Duration::from_millis(20));

    // Flowstate should be expired (returns 0)
    assert_eq!(manager.get_counter(&wallet, "short_lived"), 0);
}

#[test]
fn test_ai_agent_scenario_runaway_behavior() {
    let mut manager = FlowStateManager::new(Some(1));
    let wallet = Pubkey::new_unique();

    // Simulate runaway AI agent sending 15 transactions rapidly
    for i in 1..=15 {
        manager.increment(&wallet, "transaction_count", Some(Duration::from_secs(600)));

        let count = manager.get_counter(&wallet, "transaction_count");

        if i < 10 {
            assert!(count < 10, "Should not trigger block at transaction {}", i);
        } else {
            assert!(count >= 10, "Should trigger block at transaction {}", i);
        }
    }
}

#[test]
fn test_ai_agent_scenario_account_spam() {
    let mut manager = FlowStateManager::new(Some(1));
    let wallet = Pubkey::new_unique();

    // Simulate AI agent creating accounts in a loop
    for i in 1..=7 {
        manager.increment(
            &wallet,
            "account_creation_count",
            Some(Duration::from_secs(300)),
        );

        let count = manager.get_counter(&wallet, "account_creation_count");

        if i < 5 {
            assert!(count < 5, "Should not trigger block at creation {}", i);
        } else {
            assert!(count >= 5, "Should trigger block at creation {}", i);
        }
    }
}

#[test]
fn test_repeated_block_detection() {
    let mut manager = FlowStateManager::new(Some(1));
    let wallet = Pubkey::new_unique();

    // Simulate rule engine incrementing on each block
    for i in 1..=4 {
        manager.increment(
            &wallet,
            "blocked_transaction_count",
            Some(Duration::from_secs(3600)),
        );

        let count = manager.get_counter(&wallet, "blocked_transaction_count");

        if i < 3 {
            assert!(count < 3, "Should not trigger alert at block {}", i);
        } else {
            assert!(count >= 3, "Should trigger alert at block {}", i);
        }
    }
}
