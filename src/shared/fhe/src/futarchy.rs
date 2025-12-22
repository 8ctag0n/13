//! Futarchy-specific FHE operations for bet amounts (u64)
//!
//! Provides encryption/decryption of bet amounts using FheUint64.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use tfhe::prelude::*;
use tfhe::{ClientKey, FheUint64};

/// Encrypt a bet amount (u64) for futarchy markets
///
/// # Arguments
/// * `amount` - Bet amount in lamports
/// * `client_key` - FHE client key for encryption
///
/// # Returns
/// Serialized ciphertext (typically ~500KB)
pub fn encrypt_bet_amount(amount: u64, client_key: &ClientKey) -> Result<Vec<u8>> {
    let encrypted = FheUint64::try_encrypt(amount, client_key)
        .map_err(|e| anyhow::anyhow!("Encryption failed: {:?}", e))?;

    bincode::serialize(&encrypted)
        .context("Failed to serialize encrypted bet amount")
}

/// Decrypt a bet amount (u64) from ciphertext
///
/// # Arguments
/// * `encrypted_bytes` - Serialized FheUint64 ciphertext
/// * `client_key` - FHE client key for decryption
pub fn decrypt_bet_amount(encrypted_bytes: &[u8], client_key: &ClientKey) -> Result<u64> {
    let ciphertext: FheUint64 = bincode::deserialize(encrypted_bytes)
        .context("Failed to deserialize bet ciphertext")?;

    Ok(ciphertext.decrypt(client_key))
}

/// Hash a ciphertext using SHA256
///
/// The hash is used as on-chain reference while the full ciphertext
/// is stored off-chain (server/IPFS).
///
/// # Returns
/// 32-byte SHA256 hash
pub fn hash_ciphertext(ciphertext: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(ciphertext);
    hasher.finalize().into()
}

/// Hash ciphertext and return as hex string
pub fn hash_ciphertext_hex(ciphertext: &[u8]) -> String {
    hex::encode(hash_ciphertext(ciphertext))
}

/// Encrypted bet with both ciphertext and hash
#[derive(Debug, Clone)]
pub struct EncryptedBet {
    /// Full ciphertext - store off-chain
    pub ciphertext: Vec<u8>,
    /// SHA256 hash - store on-chain as reference
    pub hash: [u8; 32],
}

impl EncryptedBet {
    /// Create from ciphertext bytes
    pub fn from_ciphertext(ciphertext: Vec<u8>) -> Self {
        let hash = hash_ciphertext(&ciphertext);
        Self { ciphertext, hash }
    }

    /// Hash as hex string (for APIs)
    pub fn hash_hex(&self) -> String {
        hex::encode(self.hash)
    }

    /// Ciphertext size in bytes
    pub fn size(&self) -> usize {
        self.ciphertext.len()
    }
}

/// Encrypt bet amount and return with hash
///
/// Typical flow:
/// 1. Client encrypts with this function
/// 2. Sends `ciphertext` to app server
/// 3. Sends `hash` on-chain as reference
/// 4. Provers fetch ciphertext from server using hash
pub fn encrypt_bet_with_hash(amount: u64, client_key: &ClientKey) -> Result<EncryptedBet> {
    let ciphertext = encrypt_bet_amount(amount, client_key)?;
    Ok(EncryptedBet::from_ciphertext(ciphertext))
}

/// Verify that a ciphertext matches an expected hash
pub fn verify_ciphertext_hash(ciphertext: &[u8], expected_hash: &[u8; 32]) -> bool {
    hash_ciphertext(ciphertext) == *expected_hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::generate_keys;

    #[test]
    fn test_encrypt_decrypt_u64() {
        let (client_key, _) = generate_keys().unwrap();

        let amounts = vec![0u64, 1, 100, 1_000_000, 1_000_000_000, u64::MAX >> 16];

        for amount in amounts {
            let encrypted = encrypt_bet_amount(amount, &client_key).unwrap();
            let decrypted = decrypt_bet_amount(&encrypted, &client_key).unwrap();
            assert_eq!(amount, decrypted, "Mismatch for amount {}", amount);
        }
    }

    #[test]
    fn test_hash_consistency() {
        let (client_key, _) = generate_keys().unwrap();
        let encrypted = encrypt_bet_amount(12345, &client_key).unwrap();

        let hash1 = hash_ciphertext(&encrypted);
        let hash2 = hash_ciphertext(&encrypted);

        assert_eq!(hash1, hash2, "Hash should be deterministic");
    }

    #[test]
    fn test_encrypt_with_hash() {
        let (client_key, _) = generate_keys().unwrap();
        let amount = 999_999u64;

        let encrypted_bet = encrypt_bet_with_hash(amount, &client_key).unwrap();

        // Verify hash matches ciphertext
        assert!(verify_ciphertext_hash(&encrypted_bet.ciphertext, &encrypted_bet.hash));

        // Verify can decrypt
        let decrypted = decrypt_bet_amount(&encrypted_bet.ciphertext, &client_key).unwrap();
        assert_eq!(amount, decrypted);
    }

    #[test]
    fn test_randomized_encryption() {
        let (client_key, _) = generate_keys().unwrap();
        let amount = 777u64;

        // Same amount encrypted twice should produce different ciphertexts
        let ct1 = encrypt_bet_amount(amount, &client_key).unwrap();
        let ct2 = encrypt_bet_amount(amount, &client_key).unwrap();

        assert_ne!(ct1, ct2, "Encryption should be randomized");

        // But both should decrypt to same value
        let d1 = decrypt_bet_amount(&ct1, &client_key).unwrap();
        let d2 = decrypt_bet_amount(&ct2, &client_key).unwrap();
        assert_eq!(d1, d2);
        assert_eq!(amount, d1);
    }

    #[test]
    fn test_hash_hex() {
        let (client_key, _) = generate_keys().unwrap();
        let encrypted_bet = encrypt_bet_with_hash(100, &client_key).unwrap();

        let hash_hex = encrypted_bet.hash_hex();
        assert_eq!(hash_hex.len(), 64, "SHA256 hex should be 64 chars");
    }
}
