//! FHE key generation and serialization utilities

use anyhow::{Context, Result};
use tfhe::{generate_keys as tfhe_generate_keys, ClientKey, ConfigBuilder, ServerKey};

/// Generate a new FHE keypair (client key + server key)
///
/// The client key is used for encryption/decryption and must remain private.
/// The server key is used for computation and can be shared with provers.
///
/// # Performance Note
/// Key generation is slow (~1 second), so cache keys in production.
///
/// # Example
/// ```ignore
/// let (client_key, server_key) = generate_keys()?;
/// // Keep client_key private
/// // Share server_key with provers
/// ```
pub fn generate_keys() -> Result<(ClientKey, ServerKey)> {
    let config = ConfigBuilder::default().build();
    Ok(tfhe_generate_keys(config))
}

/// Serialize server key to bytes for storage/transmission
///
/// # Warning
/// Server keys are large (~50-100MB), consider compression for network transfer.
pub fn serialize_server_key(server_key: &ServerKey) -> Result<Vec<u8>> {
    bincode::serialize(server_key).context("Failed to serialize server key")
}

/// Deserialize server key from bytes
pub fn deserialize_server_key(bytes: &[u8]) -> Result<ServerKey> {
    bincode::deserialize(bytes).context("Failed to deserialize server key")
}

/// Serialize client key to bytes for storage
///
/// # Security Warning
/// Client keys are secret! Store securely (encrypted, secure enclave, etc.)
pub fn serialize_client_key(client_key: &ClientKey) -> Result<Vec<u8>> {
    bincode::serialize(client_key).context("Failed to serialize client key")
}

/// Deserialize client key from bytes
pub fn deserialize_client_key(bytes: &[u8]) -> Result<ClientKey> {
    bincode::deserialize(bytes).context("Failed to deserialize client key")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tfhe::prelude::*;
    use tfhe::FheUint8;

    #[test]
    fn test_key_generation() {
        let result = generate_keys();
        assert!(result.is_ok());

        let (client_key, _server_key) = result.unwrap();

        // Verify keys work by encrypting/decrypting
        let value = 42u8;
        let encrypted = FheUint8::try_encrypt(value, &client_key).unwrap();
        let decrypted: u8 = encrypted.decrypt(&client_key);
        assert_eq!(value, decrypted);
    }

    #[test]
    fn test_server_key_serialization_roundtrip() {
        let (_client_key, server_key) = generate_keys().unwrap();

        let bytes = serialize_server_key(&server_key).unwrap();
        let restored = deserialize_server_key(&bytes).unwrap();

        // Keys should be functionally equivalent (can't directly compare)
        // We verify by checking serialization produces same bytes
        let bytes2 = serialize_server_key(&restored).unwrap();
        assert_eq!(bytes, bytes2);
    }

    #[test]
    fn test_client_key_serialization_roundtrip() {
        let (client_key, _server_key) = generate_keys().unwrap();

        let bytes = serialize_client_key(&client_key).unwrap();
        let restored = deserialize_client_key(&bytes).unwrap();

        // Verify restored key works
        let value = 123u8;
        let encrypted = FheUint8::try_encrypt(value, &restored).unwrap();
        let decrypted: u8 = encrypted.decrypt(&restored);
        assert_eq!(value, decrypted);
    }
}
