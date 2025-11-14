/// FHE Test Utilities for E2E Testing
///
/// Provides helper functions for FHE job testing including:
/// - FHE key generation
/// - Encryption/decryption
/// - Result hashing
/// - Test configurations

use anyhow::Result;
use sha3::{Digest, Sha3_256};
use tfhe::{generate_keys, ClientKey, ConfigBuilder, FheUint8, ServerKey};

/// Generate FHE keypair for testing
///
/// Returns (ClientKey, ServerKey) tuple
/// Note: Key generation is slow (~1 second), consider caching in tests
pub fn setup_fhe_keys() -> Result<(ClientKey, ServerKey)> {
    let config = ConfigBuilder::default().build();
    Ok(generate_keys(config))
}

/// Encrypt u8 value with client key
///
/// # Arguments
/// * `value` - Plaintext value to encrypt
/// * `client_key` - Client's private key
///
/// # Returns
/// Serialized encrypted FheUint8
pub fn encrypt_value(value: u8, client_key: &ClientKey) -> Result<Vec<u8>> {
    use tfhe::prelude::*;
    let encrypted = FheUint8::try_encrypt(value, client_key)?;
    Ok(bincode::serialize(&encrypted)?)
}

/// Decrypt result with client key
///
/// # Arguments
/// * `encrypted_bytes` - Serialized encrypted FheUint8
/// * `client_key` - Client's private key
///
/// # Returns
/// Decrypted plaintext value
pub fn decrypt_value(encrypted_bytes: &[u8], client_key: &ClientKey) -> Result<u8> {
    use tfhe::prelude::*;
    let encrypted: FheUint8 = bincode::deserialize(encrypted_bytes)?;
    Ok(encrypted.decrypt(client_key))
}

/// Perform FHE addition (prover-side simulation)
///
/// # Arguments
/// * `encrypted_input` - Serialized encrypted input
/// * `constant` - Plaintext constant to add
/// * `server_key` - Server's public key
///
/// # Returns
/// Serialized encrypted result
pub fn compute_fhe_add(
    encrypted_input: &[u8],
    constant: u8,
    server_key: &ServerKey,
) -> Result<Vec<u8>> {
    use tfhe::{prelude::*, set_server_key};

    set_server_key(server_key.clone());

    let input: FheUint8 = bincode::deserialize(encrypted_input)?;
    let result = input + constant;

    Ok(bincode::serialize(&result)?)
}

/// Perform FHE multiplication (prover-side simulation)
pub fn compute_fhe_multiply(
    encrypted_input: &[u8],
    constant: u8,
    server_key: &ServerKey,
) -> Result<Vec<u8>> {
    use tfhe::{prelude::*, set_server_key};

    set_server_key(server_key.clone());

    let input: FheUint8 = bincode::deserialize(encrypted_input)?;
    let result = input * constant;

    Ok(bincode::serialize(&result)?)
}

/// Perform FHE subtraction (prover-side simulation)
pub fn compute_fhe_subtract(
    encrypted_input: &[u8],
    constant: u8,
    server_key: &ServerKey,
) -> Result<Vec<u8>> {
    use tfhe::{prelude::*, set_server_key};

    set_server_key(server_key.clone());

    let input: FheUint8 = bincode::deserialize(encrypted_input)?;
    let result = input - constant;

    Ok(bincode::serialize(&result)?)
}

/// Hash encrypted result for consensus
///
/// Multiple provers computing the same operation should produce identical hashes.
///
/// # Arguments
/// * `result_bytes` - Serialized encrypted result
///
/// # Returns
/// SHA3-256 hash (32 bytes)
pub fn hash_fhe_result(result_bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha3_256::new();
    hasher.update(result_bytes);
    let hash = hasher.finalize();

    let mut output = [0u8; 32];
    output.copy_from_slice(&hash);
    output
}

/// Create test FHE job configuration (3 provers, 2-of-3 consensus)
pub fn default_fhe_config() -> cypherlink_types::FheConsensusConfig {
    use cypherlink_types::{FheConsensusConfig, FheOperation};

    FheConsensusConfig {
        required_provers: 3,
        consensus_threshold: 2,
        submission_timeout_secs: 300,
        operation: FheOperation::Add(10),
    }
}

/// Create custom FHE configuration
pub fn custom_fhe_config(
    required_provers: u8,
    consensus_threshold: u8,
    operation: cypherlink_types::FheOperation,
) -> cypherlink_types::FheConsensusConfig {
    use cypherlink_types::FheConsensusConfig;

    FheConsensusConfig {
        required_provers,
        consensus_threshold,
        submission_timeout_secs: 300,
        operation,
    }
}

/// Simulate wrong computation (for testing consensus failure)
///
/// Returns a different result than correct computation
pub fn compute_fhe_wrong(
    encrypted_input: &[u8],
    server_key: &ServerKey,
) -> Result<Vec<u8>> {
    use tfhe::{prelude::*, set_server_key};

    set_server_key(server_key.clone());

    let input: FheUint8 = bincode::deserialize(encrypted_input)?;
    // Intentionally wrong: multiply by 2 instead of adding
    let result = input * 2u8;

    Ok(bincode::serialize(&result)?)
}

/// Serialize server key for transmission
pub fn serialize_server_key(server_key: &ServerKey) -> Result<Vec<u8>> {
    Ok(bincode::serialize(server_key)?)
}

/// Deserialize server key
pub fn deserialize_server_key(bytes: &[u8]) -> Result<ServerKey> {
    Ok(bincode::deserialize(bytes)?)
}

/// Serialize client key for storage
pub fn serialize_client_key(client_key: &ClientKey) -> Result<Vec<u8>> {
    Ok(bincode::serialize(client_key)?)
}

/// Deserialize client key
pub fn deserialize_client_key(bytes: &[u8]) -> Result<ClientKey> {
    Ok(bincode::deserialize(bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let (client_key, _) = setup_fhe_keys().unwrap();

        let value = 42u8;
        let encrypted = encrypt_value(value, &client_key).unwrap();
        let decrypted = decrypt_value(&encrypted, &client_key).unwrap();

        assert_eq!(decrypted, value);
    }

    #[test]
    fn test_fhe_add_computation() {
        let (client_key, server_key) = setup_fhe_keys().unwrap();

        let value = 100u8;
        let constant = 50u8;

        let encrypted = encrypt_value(value, &client_key).unwrap();
        let result = compute_fhe_add(&encrypted, constant, &server_key).unwrap();
        let decrypted = decrypt_value(&result, &client_key).unwrap();

        assert_eq!(decrypted, value + constant);
    }

    #[test]
    fn test_fhe_multiply_computation() {
        let (client_key, server_key) = setup_fhe_keys().unwrap();

        let value = 7u8;
        let constant = 6u8;

        let encrypted = encrypt_value(value, &client_key).unwrap();
        let result = compute_fhe_multiply(&encrypted, constant, &server_key).unwrap();
        let decrypted = decrypt_value(&result, &client_key).unwrap();

        assert_eq!(decrypted, value * constant);
    }

    #[test]
    fn test_result_hashing_deterministic() {
        let data = vec![1, 2, 3, 4, 5];
        let hash1 = hash_fhe_result(&data);
        let hash2 = hash_fhe_result(&data);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_different_results_different_hashes() {
        let data1 = vec![1, 2, 3];
        let data2 = vec![1, 2, 4];

        let hash1 = hash_fhe_result(&data1);
        let hash2 = hash_fhe_result(&data2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_key_serialization() {
        let (client_key, server_key) = setup_fhe_keys().unwrap();

        let client_bytes = serialize_client_key(&client_key).unwrap();
        let server_bytes = serialize_server_key(&server_key).unwrap();

        let client_restored = deserialize_client_key(&client_bytes).unwrap();
        let server_restored = deserialize_server_key(&server_bytes).unwrap();

        // Verify keys work after serialization
        let value = 123u8;
        let encrypted = encrypt_value(value, &client_restored).unwrap();
        let result = compute_fhe_add(&encrypted, 1, &server_restored).unwrap();
        let decrypted = decrypt_value(&result, &client_restored).unwrap();

        assert_eq!(decrypted, 124);
    }
}
