use anyhow::{anyhow, Result};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Verify a Solana wallet signature
pub fn verify_wallet_signature(wallet: &str, message: &str, signature: &str) -> Result<()> {
    // Parse wallet address
    let pubkey = Pubkey::from_str(wallet).map_err(|e| anyhow!("Invalid wallet address: {}", e))?;

    // Decode signature from base58
    let sig_bytes = bs58::decode(signature)
        .into_vec()
        .map_err(|e| anyhow!("Invalid signature encoding: {}", e))?;

    if sig_bytes.len() != 64 {
        return Err(anyhow!(
            "Invalid signature length: expected 64 bytes, got {}",
            sig_bytes.len()
        ));
    }

    let signature = Signature::from_bytes(&sig_bytes.try_into().unwrap());

    // Convert Pubkey to VerifyingKey
    let verifying_key_bytes: [u8; 32] = pubkey.to_bytes();
    let verifying_key = VerifyingKey::from_bytes(&verifying_key_bytes)
        .map_err(|e| anyhow!("Invalid public key: {}", e))?;

    // Verify signature
    verifying_key
        .verify(message.as_bytes(), &signature)
        .map_err(|e| anyhow!("Signature verification failed: {}", e))?;

    Ok(())
}

/// Check if a wallet is authorized
pub fn is_wallet_authorized(wallet: &str, authorized_wallets: &[String]) -> bool {
    if authorized_wallets.is_empty() {
        // If no wallets configured, allow all (insecure default for development)
        log::warn!("⚠️  No authorized wallets configured - allowing all wallets (INSECURE)");
        return true;
    }

    authorized_wallets.iter().any(|w| w == wallet)
}

/// Verify timestamp is recent (within 60 seconds)
pub fn verify_timestamp(timestamp: u64) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let age = now.saturating_sub(timestamp);

    if age > 60 {
        return Err(anyhow!("Signature expired: {} seconds old", age));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_verification() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Recent timestamp should pass
        assert!(verify_timestamp(now).is_ok());

        // Old timestamp should fail
        assert!(verify_timestamp(now - 120).is_err());
    }

    #[test]
    fn test_authorization() {
        let wallet = "9zQeWvJ7TB5bZx7rXDDPKKULJVmL6H4BxW5u4q4tz4cz";
        let authorized = vec![wallet.to_string()];

        assert!(is_wallet_authorized(wallet, &authorized));
        assert!(!is_wallet_authorized("different_wallet", &authorized));
    }
}
