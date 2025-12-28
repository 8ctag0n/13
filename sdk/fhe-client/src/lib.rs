pub mod client;
pub mod keys;
pub mod serialization;

pub use client::{EncryptedBet, FutarchyFheClient};
pub use keys::{ClientKeyData, ServerKeyData};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FheError {
    #[error("Failed to serialize data: {0}")]
    SerializationError(String),

    #[error("Failed to deserialize data: {0}")]
    DeserializationError(String),

    #[error("Encryption failed: {0}")]
    EncryptionError(String),

    #[error("Decryption failed: {0}")]
    DecryptionError(String),

    #[error("Key generation failed: {0}")]
    KeyGenerationError(String),

    #[error("Invalid ciphertext")]
    InvalidCiphertext,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_encryption_decryption() {
        let client = FutarchyFheClient::new().expect("Failed to create client");

        let original_amount: u64 = 1000;
        let encrypted = client.encrypt_bet_amount(original_amount)
            .expect("Failed to encrypt");

        let decrypted = client.decrypt_pool(&encrypted)
            .expect("Failed to decrypt");

        assert_eq!(original_amount, decrypted);
    }

    #[test]
    fn test_different_ciphertexts_same_value() {
        let client = FutarchyFheClient::new().expect("Failed to create client");

        let amount: u64 = 500;
        let encrypted1 = client.encrypt_bet_amount(amount)
            .expect("Failed to encrypt first time");
        let encrypted2 = client.encrypt_bet_amount(amount)
            .expect("Failed to encrypt second time");

        // Los ciphertexts deben ser diferentes debido a randomness
        assert_ne!(encrypted1, encrypted2);

        // Pero ambos deben desencriptar al mismo valor
        let decrypted1 = client.decrypt_pool(&encrypted1).expect("Failed to decrypt 1");
        let decrypted2 = client.decrypt_pool(&encrypted2).expect("Failed to decrypt 2");
        assert_eq!(amount, decrypted1);
        assert_eq!(amount, decrypted2);
    }

    #[test]
    fn test_key_serialization_roundtrip() {
        let client = FutarchyFheClient::new().expect("Failed to create client");

        let client_key_bytes = client.get_client_key_bytes();
        let server_key_bytes = client.get_server_key_bytes();

        // Recrear client desde keys serializadas
        let restored_client = FutarchyFheClient::from_keys(&client_key_bytes, &server_key_bytes)
            .expect("Failed to restore from keys");

        // Verificar que funciona
        let amount: u64 = 750;
        let encrypted = restored_client.encrypt_bet_amount(amount)
            .expect("Failed to encrypt with restored client");
        let decrypted = client.decrypt_pool(&encrypted)
            .expect("Failed to decrypt with original client");

        assert_eq!(amount, decrypted);
    }

    #[test]
    fn test_encryption_of_zero() {
        let client = FutarchyFheClient::new().expect("Failed to create client");

        let encrypted = client.encrypt_bet_amount(0)
            .expect("Failed to encrypt zero");
        let decrypted = client.decrypt_pool(&encrypted)
            .expect("Failed to decrypt");

        assert_eq!(0, decrypted);
    }

    #[test]
    fn test_encryption_of_max_value() {
        let client = FutarchyFheClient::new().expect("Failed to create client");

        // Usar un valor grande pero razonable para u64
        let max_amount: u64 = u64::MAX >> 8; // Reducido para compatibilidad
        let encrypted = client.encrypt_bet_amount(max_amount)
            .expect("Failed to encrypt max value");
        let decrypted = client.decrypt_pool(&encrypted)
            .expect("Failed to decrypt");

        assert_eq!(max_amount, decrypted);
    }

    #[test]
    fn test_ciphertext_size() {
        let client = FutarchyFheClient::new().expect("Failed to create client");

        let encrypted = client.encrypt_bet_amount(1000)
            .expect("Failed to encrypt");

        // TFHE ciphertexts are large (~500KB for FheUint64 with default params)
        // For production, consider:
        // - Using smaller integer types (FheUint16, FheUint32)
        // - Off-chain storage (IPFS/Arweave) with on-chain hash
        // - TFHE packing/compression features
        println!("Ciphertext size: {} bytes ({:.2} KB)", encrypted.len(), encrypted.len() as f64 / 1024.0);
        assert!(encrypted.len() > 100_000, "Ciphertext unexpectedly small");
        assert!(encrypted.len() < 1_000_000, "Ciphertext unexpectedly large");
    }

    #[test]
    fn test_encrypt_bet_with_hash() {
        let client = FutarchyFheClient::new().expect("Failed to create client");

        let amount: u64 = 1000;
        let encrypted_bet = client.encrypt_bet_with_hash(amount)
            .expect("Failed to encrypt with hash");

        // Hash debe ser 32 bytes
        assert_eq!(encrypted_bet.hash.len(), 32);

        // Hash hex debe ser 64 caracteres
        assert_eq!(encrypted_bet.hash_hex().len(), 64);

        // Verificar que el hash corresponde al ciphertext
        assert!(FutarchyFheClient::verify_ciphertext_hash(
            &encrypted_bet.ciphertext,
            &encrypted_bet.hash
        ));

        // Desencriptar debe dar el valor original
        let decrypted = client.decrypt_pool(&encrypted_bet.ciphertext)
            .expect("Failed to decrypt");
        assert_eq!(amount, decrypted);

        println!("Ciphertext size: {} bytes", encrypted_bet.ciphertext_size());
        println!("Hash: {}", encrypted_bet.hash_hex());
    }

    #[test]
    fn test_hash_verification_fails_for_wrong_ciphertext() {
        let client = FutarchyFheClient::new().expect("Failed to create client");

        let bet1 = client.encrypt_bet_with_hash(100).expect("Failed to encrypt bet1");
        let bet2 = client.encrypt_bet_with_hash(200).expect("Failed to encrypt bet2");

        // Hash de bet1 no debe verificar con ciphertext de bet2
        assert!(!FutarchyFheClient::verify_ciphertext_hash(
            &bet2.ciphertext,
            &bet1.hash
        ));
    }
}
