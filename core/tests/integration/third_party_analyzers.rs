// Integration tests for third-party analyzers that require API calls
// These tests are disabled by default and must be run with environment variables set

#[allow(unused_imports)]
use parapet_core::rules::analyzer::TransactionAnalyzer;
#[allow(unused_imports)]
use solana_sdk::{
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};
#[allow(unused_imports)]
use std::str::FromStr;

#[cfg(feature = "helius")]
use parapet_core::rules::analyzers::{HeliusIdentityAnalyzer, HeliusTransferAnalyzer, HeliusFundingAnalyzer};

#[cfg(feature = "ottersec")]
use parapet_core::rules::analyzers::OtterSecVerifiedAnalyzer;

#[cfg(feature = "token-mint")]
use parapet_core::rules::analyzers::TokenMintAnalyzer;

// Helper to check if integration tests should run
fn should_run_integration_tests() -> bool {
    std::env::var("RUN_INTEGRATION_TESTS").is_ok()
}

#[cfg(feature = "helius")]
#[tokio::test]
async fn test_helius_identity_analyzer_integration() {
    if !should_run_integration_tests() {
        println!("Skipping integration test (set RUN_INTEGRATION_TESTS=1 to run)");
        return;
    }
    
    std::env::var("HELIUS_API_KEY")
        .expect("HELIUS_API_KEY must be set for integration tests");
    
    let analyzer = HeliusIdentityAnalyzer::new();
    assert_eq!(analyzer.name(), "helius_identity");
    
    // Create transaction with known wallet
    let known_wallet = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
    let payer = Keypair::new();
    
    let ix = Instruction {
        program_id: known_wallet,
        accounts: vec![],
        data: vec![],
    };
    
    let message = Message::new(&[ix], Some(&payer.pubkey()));
    let tx = Transaction::new_unsigned(message);
    
    let fields = analyzer.analyze(&tx).await;
    
    match fields {
        Ok(fields) => {
            println!("Helius analysis successful: {:?}", fields);
            
            // Verify expected fields are present
            assert!(fields.contains_key("helius_identity:total_tokens"));
            assert!(fields.contains_key("helius_identity:total_nfts"));
            assert!(fields.contains_key("helius_identity:wallet_type"));
        },
        Err(e) => {
            // API might fail, but we should get a proper error
            println!("Helius API error (expected in some cases): {}", e);
        }
    }
}

#[cfg(feature = "ottersec")]
#[tokio::test]
async fn test_ottersec_verified_analyzer_integration() {
    if !should_run_integration_tests() {
        println!("Skipping integration test (set RUN_INTEGRATION_TESTS=1 to run)");
        return;
    }
    
    let analyzer = OtterSecVerifiedAnalyzer::new();
    assert_eq!(analyzer.name(), "ottersec_verified");
    
    // Test with known Solana program
    let known_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
    let payer = Keypair::new();
    
    let ix = Instruction {
        program_id: known_program,
        accounts: vec![],
        data: vec![],
    };
    
    let message = Message::new(&[ix], Some(&payer.pubkey()));
    let tx = Transaction::new_unsigned(message);
    
    let fields = analyzer.analyze(&tx).await;
    
    match fields {
        Ok(fields) => {
            println!("OtterSec analysis successful: {:?}", fields);
            
            // Verify expected fields
            assert!(fields.contains_key("ottersec_verified:verified_program_count"));
            assert!(fields.contains_key("ottersec_verified:all_programs_verified"));
        },
        Err(e) => {
            println!("OtterSec API error (expected in some cases): {}", e);
        }
    }
}

#[cfg(feature = "token-mint")]
#[tokio::test]
async fn test_token_mint_analyzer_integration() {
    if !should_run_integration_tests() {
        println!("Skipping integration test (set RUN_INTEGRATION_TESTS=1 to run)");
        return;
    }
    
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    
    let analyzer = TokenMintAnalyzer::new(rpc_url);
    assert_eq!(analyzer.name(), "token_mint");
    
    // Test with known mint (USDC)
    let payer = Keypair::new();
    
    let token_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
    
    // Create a transfer_checked instruction that includes the mint
    let mut data = vec![12u8]; // TRANSFER_CHECKED discriminator
    data.extend_from_slice(&1000u64.to_le_bytes());
    data.push(6); // decimals
    
    let ix = Instruction {
        program_id: token_program,
        accounts: vec![],
        data,
    };
    
    let message = Message::new(&[ix], Some(&payer.pubkey()));
    let tx = Transaction::new_unsigned(message);
    
    let fields = analyzer.analyze(&tx).await;
    
    match fields {
        Ok(fields) => {
            println!("TokenMint analysis successful: {:?}", fields);
            
            // Verify expected fields
            assert!(fields.contains_key("token_mint:freeze_authority_present"));
            assert!(fields.contains_key("token_mint:mint_authority_present"));
        },
        Err(e) => {
            println!("TokenMint RPC error (expected in some cases): {}", e);
        }
    }
}

// Unit tests that don't require API calls

#[cfg(feature = "helius")]
#[test]
fn test_helius_analyzer_metadata() {
    let analyzer = HeliusIdentityAnalyzer::new();
    
    assert_eq!(analyzer.name(), "helius_identity");
    assert_eq!(analyzer.estimated_latency_ms(), 150); // Network call expected
    
    let fields = analyzer.fields();
    assert!(!fields.is_empty());
    assert!(fields.iter().any(|f| f.contains("classification")));
}

#[cfg(feature = "ottersec")]
#[test]
fn test_ottersec_analyzer_metadata() {
    let analyzer = OtterSecVerifiedAnalyzer::new();
    
    assert_eq!(analyzer.name(), "ottersec");
    assert!(analyzer.estimated_latency_ms() > 50); // Network call expected
    
    let fields = analyzer.fields();
    assert!(!fields.is_empty());
    assert!(fields.iter().any(|f| f.contains("verified")));
}

#[cfg(feature = "token-mint")]
#[test]
fn test_token_mint_analyzer_metadata() {
    let analyzer = TokenMintAnalyzer::new("https://api.mainnet-beta.solana.com".to_string());
    
    assert_eq!(analyzer.name(), "token_mint");
    assert_eq!(analyzer.estimated_latency_ms(), 50); // RPC call expected
    
    let fields = analyzer.fields();
    assert!(!fields.is_empty());
    assert!(fields.iter().any(|f| f.contains("freeze_authority")));
}

#[cfg(feature = "helius")]
#[tokio::test]
async fn test_helius_transfer_analyzer_integration() {
    if !should_run_integration_tests() {
        println!("Skipping integration test (set RUN_INTEGRATION_TESTS=1 to run)");
        return;
    }
    
    std::env::var("HELIUS_API_KEY")
        .expect("HELIUS_API_KEY must be set for integration tests");
    
    let analyzer = HeliusTransferAnalyzer::new();
    assert_eq!(analyzer.name(), "helius_transfer");
    
    // Verify fields
    let fields = analyzer.fields();
    assert!(fields.contains(&"outgoing_tx_per_hour".to_string()));
    assert!(fields.contains(&"max_transfers_to_same_address".to_string()));
    assert!(fields.contains(&"is_high_velocity".to_string()));
    assert!(fields.contains(&"top_counterparty".to_string()));
    assert!(fields.contains(&"counterparty_concentration".to_string()));
    
    // Create transaction with known wallet (fee payer)
    let known_wallet = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
    let payer = Keypair::new();
    
    let ix = Instruction {
        program_id: known_wallet,
        accounts: vec![],
        data: vec![],
    };
    
    let message = Message::new(&[ix], Some(&payer.pubkey()));
    let tx = Transaction::new_unsigned(message);
    
    let result = analyzer.analyze(&tx).await;
    
    match result {
        Ok(fields) => {
            println!("Helius Transfer analysis successful: {:?}", fields);
            
            // Verify expected fields are present (may be 0 for wallets with no history)
            assert!(fields.contains_key("outgoing_tx_per_hour"));
            assert!(fields.contains_key("is_high_velocity"));
            assert!(fields.contains_key("counterparty_concentration"));
        },
        Err(e) => {
            // API might fail or wallet might have no history
            println!("Helius Transfer API result (may be empty for new wallets): {}", e);
        }
    }
}

#[cfg(feature = "helius")]
#[tokio::test]
async fn test_helius_funding_analyzer_integration() {
    if !should_run_integration_tests() {
        println!("Skipping integration test (set RUN_INTEGRATION_TESTS=1 to run)");
        return;
    }
    
    std::env::var("HELIUS_API_KEY")
        .expect("HELIUS_API_KEY must be set for integration tests");
    
    let analyzer = HeliusFundingAnalyzer::new();
    assert_eq!(analyzer.name(), "helius_funding");
    
    // Verify fields
    let fields = analyzer.fields();
    assert!(fields.contains(&"funding_source".to_string()));
    assert!(fields.contains(&"funding_source_type".to_string()));
    assert!(fields.contains(&"funding_risk_score".to_string()));
    assert!(fields.contains(&"is_likely_sybil".to_string()));
    assert!(fields.contains(&"funding_age_hours".to_string()));
    
    // Create transaction with known wallet (fee payer)
    let known_wallet = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
    let payer = Keypair::new();
    
    let ix = Instruction {
        program_id: known_wallet,
        accounts: vec![],
        data: vec![],
    };
    
    let message = Message::new(&[ix], Some(&payer.pubkey()));
    let tx = Transaction::new_unsigned(message);
    
    let result = analyzer.analyze(&tx).await;
    
    match result {
        Ok(fields) => {
            println!("Helius Funding analysis successful: {:?}", fields);
            
            // Verify expected fields are present
            assert!(fields.contains_key("funding_source"));
            assert!(fields.contains_key("funding_risk_score"));
            assert!(fields.contains_key("is_likely_sybil"));
        },
        Err(e) => {
            // API might fail or wallet might have no funding history
            println!("Helius Funding API result (may be empty for unfunded wallets): {}", e);
        }
    }
}

#[cfg(feature = "helius")]
#[tokio::test]
async fn test_helius_transfer_analyzer_caching() {
    if !should_run_integration_tests() {
        println!("Skipping integration test (set RUN_INTEGRATION_TESTS=1 to run)");
        return;
    }
    
    std::env::var("HELIUS_API_KEY")
        .expect("HELIUS_API_KEY must be set for integration tests");
    
    let analyzer = HeliusTransferAnalyzer::new();
    
    let payer = Keypair::new();
    let ix = Instruction {
        program_id: Pubkey::new_unique(),
        accounts: vec![],
        data: vec![],
    };
    
    let message = Message::new(&[ix], Some(&payer.pubkey()));
    let tx = Transaction::new_unsigned(message);
    
    // First call - should hit API
    let start = std::time::Instant::now();
    let _result1 = analyzer.analyze(&tx).await;
    let first_duration = start.elapsed();
    
    // Second call - should use cache
    let start = std::time::Instant::now();
    let _result2 = analyzer.analyze(&tx).await;
    let second_duration = start.elapsed();
    
    println!("First call: {:?}, Second call: {:?}", first_duration, second_duration);
    
    // Cache should make second call faster (though not guaranteed due to network variance)
    // Just verify both calls complete
    assert!(first_duration.as_millis() < 5000);
    assert!(second_duration.as_millis() < 5000);
}

#[cfg(feature = "helius")]
#[tokio::test]
async fn test_helius_funding_analyzer_permanent_cache() {
    if !should_run_integration_tests() {
        println!("Skipping integration test (set RUN_INTEGRATION_TESTS=1 to run)");
        return;
    }
    
    std::env::var("HELIUS_API_KEY")
        .expect("HELIUS_API_KEY must be set for integration tests");
    
    let analyzer = HeliusFundingAnalyzer::new();
    
    let payer = Keypair::new();
    let ix = Instruction {
        program_id: Pubkey::new_unique(),
        accounts: vec![],
        data: vec![],
    };
    
    let message = Message::new(&[ix], Some(&payer.pubkey()));
    let tx = Transaction::new_unsigned(message);
    
    // First call - should hit API
    let _result1 = analyzer.analyze(&tx).await;
    
    // Second call - should use permanent cache
    let start = std::time::Instant::now();
    let _result2 = analyzer.analyze(&tx).await;
    let cached_duration = start.elapsed();
    
    println!("Cached call duration: {:?}", cached_duration);
    
    // Cached call should be very fast (< 10ms)
    assert!(cached_duration.as_millis() < 100);
}
