//! # ZyberLink FHE Engine
//!
//! Fully Homomorphic Encryption computation engine for ZyberLink.
//!
//! This crate provides:
//! - FHE computation on encrypted data (sum, count_if, average, etc.)
//! - Key generation and management
//! - Encryption/decryption helpers
//! - Result hashing for consensus
//!
//! ## Architecture
//!
//! ```text
//! Client                    Prover
//! ------                    ------
//! 1. Generate keys
//! 2. Encrypt data ------>   3. Receive encrypted data
//!                           4. Compute on ciphertext
//!                           5. Return encrypted result
//! 6. Decrypt result <------
//! ```
//!
//! ## Example
//!
//! ```ignore
//! use zyberlink_fhe::{FheEngine, generate_keys, encrypt_values, decrypt_result};
//!
//! // Generate keys
//! let (client_key, server_key) = generate_keys()?;
//!
//! // Client encrypts data
//! let encrypted = encrypt_values(&[10, 20, 30], &client_key)?;
//!
//! // Prover computes on encrypted data
//! let engine = FheEngine::new(server_key);
//! let encrypted_refs: Vec<&[u8]> = encrypted.iter().map(|v| v.as_slice()).collect();
//! let result = engine.compute_sum(&encrypted_refs)?;
//!
//! // Client decrypts result
//! let sum: u8 = decrypt_result(&result, &client_key)?;
//! assert_eq!(sum, 60);
//! ```

mod engine;
mod keys;
mod client;
mod error;
pub mod futarchy;

// Re-export main types
pub use engine::FheEngine;
pub use keys::{
    generate_keys,
    serialize_server_key,
    deserialize_server_key,
    serialize_client_key,
    deserialize_client_key,
};
pub use client::{
    encrypt_value,
    encrypt_values,
    decrypt_result,
};
pub use error::FheError;

// Re-export types from zyberlink-types for convenience
pub use zyberlink_types::{
    FhePredicate,
    FheOperation,
    FheConsensusConfig,
    FheJobResult,
    OperationCostConfig,
    HistogramBin,
};

// Re-export tfhe types that users might need
pub use tfhe::{ClientKey, ServerKey, FheUint8, FheUint16, FheUint32, FheBool};
pub use tfhe::prelude;

/// Re-export tfhe module for advanced users who need direct access
pub mod tfhe_reexport {
    pub use tfhe::*;
}

/// Re-export sha3 for hashing (used in tests and consensus)
pub mod sha3_reexport {
    pub use sha3::*;
}
