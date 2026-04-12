use anyhow::{anyhow, Result};
use ed25519_dalek::{Signature, Verifier, VerifyingKey, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH};
use rand::Rng;

pub fn generate_api_key() -> String {
    let mut rng = rand::thread_rng();
    // Generate 48 random bytes (384 bits of entropy)
    // Base58 encoding will produce ~65 character string
    let bytes: Vec<u8> = (0..48).map(|_| rng.gen()).collect();
    format!("sc_{}", bs58::encode(bytes).into_string())
}

pub fn hash_api_key(key: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn verify_wallet_signature(
    wallet_address: &str,
    message: &str,
    signature_b58: &str,
) -> Result<()> {
    // Decode wallet address (public key)
    let pubkey_bytes = bs58::decode(wallet_address)
        .into_vec()
        .map_err(|e| anyhow!("Invalid wallet address: {}", e))?;

    if pubkey_bytes.len() != PUBLIC_KEY_LENGTH {
        return Err(anyhow!("Invalid public key length"));
    }

    // Decode signature
    let sig_bytes = bs58::decode(signature_b58)
        .into_vec()
        .map_err(|e| anyhow!("Invalid signature encoding: {}", e))?;

    if sig_bytes.len() != SIGNATURE_LENGTH {
        return Err(anyhow!("Invalid signature length"));
    }

    // Create verifying key
    let verifying_key = VerifyingKey::from_bytes(
        &pubkey_bytes
            .try_into()
            .map_err(|_| anyhow!("Invalid public key"))?,
    )
    .map_err(|e| anyhow!("Invalid public key: {}", e))?;

    // Create signature
    let signature = Signature::from_bytes(
        &sig_bytes
            .try_into()
            .map_err(|_| anyhow!("Invalid signature"))?,
    );

    // Verify signature
    verifying_key
        .verify(message.as_bytes(), &signature)
        .map_err(|e| anyhow!("Signature verification failed: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_api_key() {
        let key = generate_api_key();
        assert!(key.starts_with("sc_"));
        // Should be at least 60 characters (sc_ + 48 bytes base58 encoded)
        assert!(key.len() >= 60, "Key length {} is too short", key.len());
        // Verify uniqueness - generate 10 keys and ensure all different
        let keys: std::collections::HashSet<String> = (0..10).map(|_| generate_api_key()).collect();
        assert_eq!(keys.len(), 10, "Generated keys should be unique");
    }

    #[test]
    fn test_hash_api_key() {
        let key = "sc_test123";
        let hash1 = hash_api_key(key);
        let hash2 = hash_api_key(key);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA256 hex
    }
}
