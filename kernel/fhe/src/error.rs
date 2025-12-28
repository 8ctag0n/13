//! Error types for FHE operations

use thiserror::Error;

/// Errors that can occur during FHE operations
#[derive(Error, Debug)]
pub enum FheError {
    /// Failed to generate FHE keys
    #[error("Key generation failed: {0}")]
    KeyGenerationError(String),

    /// Failed to serialize/deserialize keys
    #[error("Key serialization error: {0}")]
    KeySerializationError(String),

    /// Failed to encrypt data
    #[error("Encryption failed: {0}")]
    EncryptionError(String),

    /// Failed to decrypt data
    #[error("Decryption failed: {0}")]
    DecryptionError(String),

    /// Failed to deserialize ciphertext
    #[error("Ciphertext deserialization failed: {0}")]
    CiphertextError(String),

    /// Computation error during FHE operation
    #[error("FHE computation failed: {0}")]
    ComputationError(String),

    /// Invalid input for operation
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Empty input slice
    #[error("Empty input: operation requires at least one element")]
    EmptyInput,
}

impl From<bincode::Error> for FheError {
    fn from(err: bincode::Error) -> Self {
        FheError::CiphertextError(err.to_string())
    }
}
