//! Client-side FHE helpers for encryption and decryption

use anyhow::{Context, Result};
use tfhe::prelude::*;
use tfhe::{ClientKey, FheUint8};

/// Encrypt a single u8 value
///
/// # Example
/// ```ignore
/// let (client_key, _) = generate_keys()?;
/// let encrypted = encrypt_value(42, &client_key)?;
/// ```
pub fn encrypt_value(value: u8, client_key: &ClientKey) -> Result<Vec<u8>> {
    let encrypted = FheUint8::try_encrypt(value, client_key)
        .map_err(|e| anyhow::anyhow!("Encryption failed: {:?}", e))?;

    bincode::serialize(&encrypted)
        .context("Failed to serialize encrypted value")
}

/// Encrypt multiple u8 values
///
/// Returns a vector of serialized ciphertexts, one per input value.
///
/// # Example
/// ```ignore
/// let (client_key, _) = generate_keys()?;
/// let encrypted = encrypt_values(&[10, 20, 30], &client_key)?;
/// // encrypted.len() == 3
/// ```
pub fn encrypt_values(values: &[u8], client_key: &ClientKey) -> Result<Vec<Vec<u8>>> {
    values
        .iter()
        .enumerate()
        .map(|(i, &value)| {
            encrypt_value(value, client_key)
                .with_context(|| format!("Failed to encrypt value at index {}", i))
        })
        .collect()
}

/// Decrypt a single encrypted result
///
/// # Example
/// ```ignore
/// let result = engine.compute_sum(&encrypted_refs)?;
/// let sum: u8 = decrypt_result(&result, &client_key)?;
/// ```
pub fn decrypt_result(encrypted_bytes: &[u8], client_key: &ClientKey) -> Result<u8> {
    let ciphertext: FheUint8 = bincode::deserialize(encrypted_bytes)
        .context("Failed to deserialize ciphertext")?;

    Ok(ciphertext.decrypt(client_key))
}

/// Decrypt multiple encrypted results
///
/// # Example
/// ```ignore
/// let results = decrypt_results(&encrypted_list, &client_key)?;
/// ```
pub fn decrypt_results(encrypted_list: &[Vec<u8>], client_key: &ClientKey) -> Result<Vec<u8>> {
    encrypted_list
        .iter()
        .enumerate()
        .map(|(i, encrypted)| {
            decrypt_result(encrypted, client_key)
                .with_context(|| format!("Failed to decrypt value at index {}", i))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::generate_keys;

    #[test]
    fn test_encrypt_decrypt_single() {
        let (client_key, _) = generate_keys().unwrap();

        let value = 42u8;
        let encrypted = encrypt_value(value, &client_key).unwrap();
        let decrypted = decrypt_result(&encrypted, &client_key).unwrap();

        assert_eq!(value, decrypted);
    }

    #[test]
    fn test_encrypt_decrypt_multiple() {
        let (client_key, _) = generate_keys().unwrap();

        let values = vec![10u8, 20, 30, 40, 50];
        let encrypted = encrypt_values(&values, &client_key).unwrap();

        assert_eq!(encrypted.len(), values.len());

        for (i, enc) in encrypted.iter().enumerate() {
            let decrypted = decrypt_result(enc, &client_key).unwrap();
            assert_eq!(values[i], decrypted);
        }
    }

    #[test]
    fn test_encrypt_empty_slice() {
        let (client_key, _) = generate_keys().unwrap();

        let values: Vec<u8> = vec![];
        let encrypted = encrypt_values(&values, &client_key).unwrap();

        assert!(encrypted.is_empty());
    }

    #[test]
    fn test_decrypt_results() {
        let (client_key, _) = generate_keys().unwrap();

        let values = vec![5u8, 10, 15];
        let encrypted = encrypt_values(&values, &client_key).unwrap();
        let decrypted = decrypt_results(&encrypted, &client_key).unwrap();

        assert_eq!(values, decrypted);
    }
}
