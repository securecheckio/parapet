use base64::Engine as _;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};

pub fn test_wallet() -> (String, SigningKey) {
    // Deterministic keypair for reproducible test vectors.
    let seed = Sha256::digest(b"parapet-api-core-test-wallet");
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&seed[..32]);
    let signing_key = SigningKey::from_bytes(&bytes);
    let wallet = bs58::encode(signing_key.verifying_key().to_bytes()).into_string();
    (wallet, signing_key)
}

pub fn sign_message_base64(signing_key: &SigningKey, message: &str) -> String {
    let sig = signing_key.sign(message.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(sig.to_bytes())
}

pub fn nonce_message(wallet: &str, nonce: &str, timestamp: u64) -> String {
    format!("wallet={wallet};nonce={nonce};timestamp={timestamp}")
}
