use anyhow::Result;
use ring::{digest, hmac};

/// Cryptographic utility functions
pub struct CryptoUtils;

impl CryptoUtils {
    /// Calculate SHA-256 hash of data
    pub fn sha256(data: &[u8]) -> String {
        let digest = digest::digest(&digest::SHA256, data);
        hex::encode(digest.as_ref())
    }
    
    /// Calculate HMAC-SHA256
    pub fn hmac_sha256(key: &[u8], data: &[u8]) -> Result<String> {
        let key = hmac::Key::new(hmac::HMAC_SHA256, key);
        let signature = hmac::sign(&key, data);
        Ok(hex::encode(signature.as_ref()))
    }
    
    /// Generate random bytes
    pub fn random_bytes(len: usize) -> Vec<u8> {
        use ring::rand::{SystemRandom, SecureRandom};
        
        let rng = SystemRandom::new();
        let mut bytes = vec![0u8; len];
        rng.fill(&mut bytes).expect("Failed to generate random bytes");
        bytes
    }
}
