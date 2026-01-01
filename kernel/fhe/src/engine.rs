//! FHE computation engine for performing operations on encrypted data
//!
//! This module provides the core FHE computation capabilities using TFHE-rs,
//! allowing provers to perform computations on encrypted data without decryption.

use anyhow::{anyhow, Context, Result};
use sha3::{Digest, Sha3_256};
use tfhe::prelude::*;
use tfhe::{set_server_key, FheBool, FheUint8, ServerKey};

use zyberlink_types::FhePredicate;

/// FHE computation engine
///
/// Wraps TFHE-rs functionality and provides a clean API for:
/// - Homomorphic arithmetic operations (add, multiply, subtract)
/// - Aggregate operations (sum, average, count_if)
/// - Result hashing for consensus
///
/// # Example
/// ```ignore
/// let engine = FheEngine::new(server_key);
/// let result = engine.compute_sum(&encrypted_refs)?;
/// ```
pub struct FheEngine {
    server_key: ServerKey,
}

impl FheEngine {
    /// Initialize engine with server key
    ///
    /// The server key allows performing homomorphic operations without
    /// access to the client's private key.
    pub fn new(server_key: ServerKey) -> Self {
        set_server_key(server_key.clone());
        Self { server_key }
    }

    /// Set the server key in the current thread context
    ///
    /// TFHE uses thread-local storage for the server key, so this must be called
    /// in each thread that performs FHE operations (e.g., inside spawn_blocking).
    pub fn set_key_for_thread(&self) {
        set_server_key(self.server_key.clone());
    }

    /// Get reference to server key
    pub fn server_key(&self) -> &ServerKey {
        &self.server_key
    }

    // =========================================================================
    // Basic Arithmetic Operations
    // =========================================================================

    /// Add a constant to an encrypted value
    ///
    /// # Example
    /// ```ignore
    /// // encrypt(42) + 10 = encrypt(52)
    /// let result = engine.compute_add(&encrypted_42, 10)?;
    /// ```
    pub fn compute_add(&self, encrypted_input: &[u8], constant: u8) -> Result<Vec<u8>> {
        let ciphertext: FheUint8 = bincode::deserialize(encrypted_input)
            .map_err(|e| anyhow!("Failed to deserialize input ciphertext: {}", e))?;

        let result = ciphertext + constant;

        bincode::serialize(&result)
            .map_err(|e| anyhow!("Failed to serialize result: {}", e))
    }

    /// Multiply an encrypted value by a constant
    pub fn compute_multiply(&self, encrypted_input: &[u8], constant: u8) -> Result<Vec<u8>> {
        let ciphertext: FheUint8 = bincode::deserialize(encrypted_input)
            .context("Failed to deserialize input for multiplication")?;

        let result = ciphertext * constant;

        bincode::serialize(&result)
            .context("Failed to serialize multiplication result")
    }

    /// Subtract a constant from an encrypted value
    pub fn compute_subtract(&self, encrypted_input: &[u8], constant: u8) -> Result<Vec<u8>> {
        let ciphertext: FheUint8 = bincode::deserialize(encrypted_input)
            .context("Failed to deserialize input for subtraction")?;

        let result = ciphertext - constant;

        bincode::serialize(&result)
            .context("Failed to serialize subtraction result")
    }

    /// Add two encrypted values
    pub fn compute_add_encrypted(&self, encrypted_a: &[u8], encrypted_b: &[u8]) -> Result<Vec<u8>> {
        let ciphertext_a: FheUint8 = bincode::deserialize(encrypted_a)
            .context("Failed to deserialize first input")?;

        let ciphertext_b: FheUint8 = bincode::deserialize(encrypted_b)
            .context("Failed to deserialize second input")?;

        let result = ciphertext_a + ciphertext_b;

        bincode::serialize(&result)
            .context("Failed to serialize addition result")
    }

    // =========================================================================
    // Aggregate Operations
    // =========================================================================

    /// Sum multiple encrypted values
    ///
    /// # Example
    /// ```ignore
    /// let encrypted_refs: Vec<&[u8]> = encrypted_values.iter().map(|v| v.as_slice()).collect();
    /// let sum = engine.compute_sum(&encrypted_refs)?;
    /// ```
    pub fn compute_sum(&self, encrypted_inputs: &[&[u8]]) -> Result<Vec<u8>> {
        if encrypted_inputs.is_empty() {
            return Err(anyhow!("Input slice cannot be empty for sum operation"));
        }

        let mut accumulator: FheUint8 = bincode::deserialize(encrypted_inputs[0])
            .context("Failed to deserialize first element for sum")?;

        for (i, encrypted_input) in encrypted_inputs.iter().enumerate().skip(1) {
            let ciphertext: FheUint8 = bincode::deserialize(encrypted_input)
                .context(format!("Failed to deserialize element at index {}", i))?;
            accumulator += ciphertext;
        }

        bincode::serialize(&accumulator)
            .context("Failed to serialize sum result")
    }

    /// Count how many encrypted values satisfy a predicate
    ///
    /// Uses the unified FhePredicate from zyberlink-types.
    ///
    /// # Example
    /// ```ignore
    /// let predicate = FhePredicate::GreaterThan(18);
    /// let count = engine.compute_count_if(&encrypted_refs, &predicate)?;
    /// ```
    pub fn compute_count_if(
        &self,
        encrypted_inputs: &[&[u8]],
        predicate: &FhePredicate,
    ) -> Result<Vec<u8>> {
        if encrypted_inputs.is_empty() {
            return Err(anyhow!("count_if requires at least one encrypted input"));
        }

        let mut count_accumulator = FheUint8::try_encrypt_trivial(0u8)
            .context("Failed to create zero ciphertext for count_if")?;

        for (i, encrypted_input) in encrypted_inputs.iter().enumerate() {
            let ciphertext: FheUint8 = bincode::deserialize(encrypted_input)
                .context(format!("Failed to deserialize element at index {}", i))?;

            let condition: FheBool = self.evaluate_predicate(&ciphertext, predicate)?;

            // Convert FheBool to FheUint8 (1 if true, 0 if false)
            let one_if_true: FheUint8 = FheUint8::cast_from(condition);
            count_accumulator = &count_accumulator + &one_if_true;
        }

        bincode::serialize(&count_accumulator)
            .context("Failed to serialize count_if result")
    }

    /// Evaluate a predicate on an encrypted value
    fn evaluate_predicate(&self, ciphertext: &FheUint8, predicate: &FhePredicate) -> Result<FheBool> {
        match predicate {
            FhePredicate::Equals(val) => {
                let threshold = FheUint8::try_encrypt_trivial(*val)
                    .context("Failed to trivially encrypt threshold")?;
                Ok(ciphertext.eq(&threshold))
            }
            FhePredicate::GreaterThan(val) => {
                let threshold = FheUint8::try_encrypt_trivial(*val)
                    .context("Failed to trivially encrypt threshold")?;
                Ok(ciphertext.gt(&threshold))
            }
            FhePredicate::LessThan(val) => {
                let threshold = FheUint8::try_encrypt_trivial(*val)
                    .context("Failed to trivially encrypt threshold")?;
                Ok(ciphertext.lt(&threshold))
            }
            FhePredicate::InRange { min, max } => {
                let min_ct = FheUint8::try_encrypt_trivial(*min)
                    .context("Failed to trivially encrypt min")?;
                let max_ct = FheUint8::try_encrypt_trivial(*max)
                    .context("Failed to trivially encrypt max")?;
                let ge_min = ciphertext.ge(&min_ct);
                let le_max = ciphertext.le(&max_ct);
                Ok(ge_min & le_max)
            }
            FhePredicate::NotEquals(val) => {
                let threshold = FheUint8::try_encrypt_trivial(*val)
                    .context("Failed to trivially encrypt threshold")?;
                Ok(ciphertext.ne(&threshold))
            }
        }
    }

    /// Compute average of encrypted values
    ///
    /// Returns the encrypted sum. Client divides by count after decryption.
    pub fn compute_average(&self, encrypted_inputs: &[&[u8]]) -> Result<Vec<u8>> {
        // Average is just sum; client divides by known count
        self.compute_sum(encrypted_inputs)
    }

    // =========================================================================
    // Hashing & Consensus
    // =========================================================================

    /// Hash encrypted result for consensus checking
    ///
    /// Multiple provers should produce the same result for the same input.
    /// This hash allows consensus verification without revealing the encrypted data.
    pub fn hash_result(result_bytes: &[u8]) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(result_bytes);
        let hash = hasher.finalize();

        let mut output = [0u8; 32];
        output.copy_from_slice(&hash);
        output
    }

    /// Generate deterministic commitment for FHE consensus
    ///
    /// # DESIGN NOTE - FHE Non-Determinism Issue
    ///
    /// TFHE-rs operations are NOT deterministic across processes due to internal
    /// CSPRNG state. Different provers produce different ciphertext bytes even
    /// with identical inputs, though results decrypt to the same plaintext.
    ///
    /// For PoC, we use a deterministic commitment based on inputs to prove
    /// provers processed the same data. Production systems should implement
    /// proper verification (Verifiable FHE, Threshold FHE, etc.)
    pub fn deterministic_commitment(
        witness_hash: &[u8; 32],
        operation: &str,
        job_id: u64,
    ) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(b"ZYBERLINK_FHE_COMMITMENT_V1:");
        hasher.update(witness_hash);
        hasher.update(operation.as_bytes());
        hasher.update(job_id.to_le_bytes());
        let hash = hasher.finalize();

        let mut output = [0u8; 32];
        output.copy_from_slice(&hash);
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::generate_keys;

    #[test]
    fn test_compute_add() {
        let (client_key, server_key) = generate_keys().unwrap();
        let engine = FheEngine::new(server_key);

        let value = 42u8;
        let encrypted = FheUint8::try_encrypt(value, &client_key).unwrap();
        let encrypted_bytes = bincode::serialize(&encrypted).unwrap();

        let result_bytes = engine.compute_add(&encrypted_bytes, 10).unwrap();

        let result: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let decrypted: u8 = result.decrypt(&client_key);

        assert_eq!(decrypted, 52);
    }

    #[test]
    fn test_compute_multiply() {
        let (client_key, server_key) = generate_keys().unwrap();
        let engine = FheEngine::new(server_key);

        let value = 7u8;
        let encrypted = FheUint8::try_encrypt(value, &client_key).unwrap();
        let encrypted_bytes = bincode::serialize(&encrypted).unwrap();

        let result_bytes = engine.compute_multiply(&encrypted_bytes, 6).unwrap();

        let result: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let decrypted: u8 = result.decrypt(&client_key);

        assert_eq!(decrypted, 42);
    }

    #[test]
    fn test_compute_sum() {
        let (client_key, server_key) = generate_keys().unwrap();
        let engine = FheEngine::new(server_key);

        let values = [10u8, 20, 30, 5];
        let encrypted_values: Vec<Vec<u8>> = values
            .iter()
            .map(|&v| {
                let enc = FheUint8::try_encrypt(v, &client_key).unwrap();
                bincode::serialize(&enc).unwrap()
            })
            .collect();
        let encrypted_refs: Vec<&[u8]> = encrypted_values.iter().map(|v| v.as_slice()).collect();

        let result_bytes = engine.compute_sum(&encrypted_refs).unwrap();

        let result: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let decrypted: u8 = result.decrypt(&client_key);

        assert_eq!(decrypted, 65); // 10 + 20 + 30 + 5
    }

    #[test]
    fn test_compute_count_if_greater_than() {
        let (client_key, server_key) = generate_keys().unwrap();
        let engine = FheEngine::new(server_key);

        let values = [10u8, 20, 30, 5, 20, 15];
        let encrypted_values: Vec<Vec<u8>> = values
            .iter()
            .map(|&v| {
                let enc = FheUint8::try_encrypt(v, &client_key).unwrap();
                bincode::serialize(&enc).unwrap()
            })
            .collect();
        let encrypted_refs: Vec<&[u8]> = encrypted_values.iter().map(|v| v.as_slice()).collect();

        let predicate = FhePredicate::GreaterThan(15);
        let result_bytes = engine.compute_count_if(&encrypted_refs, &predicate).unwrap();

        let result: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let decrypted: u8 = result.decrypt(&client_key);

        // Values > 15: 20, 30, 20 = 3
        assert_eq!(decrypted, 3);
    }

    #[test]
    fn test_compute_count_if_equals() {
        let (client_key, server_key) = generate_keys().unwrap();
        let engine = FheEngine::new(server_key);

        let values = [10u8, 20, 30, 5, 20, 15];
        let encrypted_values: Vec<Vec<u8>> = values
            .iter()
            .map(|&v| {
                let enc = FheUint8::try_encrypt(v, &client_key).unwrap();
                bincode::serialize(&enc).unwrap()
            })
            .collect();
        let encrypted_refs: Vec<&[u8]> = encrypted_values.iter().map(|v| v.as_slice()).collect();

        let predicate = FhePredicate::Equals(20);
        let result_bytes = engine.compute_count_if(&encrypted_refs, &predicate).unwrap();

        let result: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let decrypted: u8 = result.decrypt(&client_key);

        // Values == 20: 2
        assert_eq!(decrypted, 2);
    }

    #[test]
    fn test_compute_count_if_in_range() {
        let (client_key, server_key) = generate_keys().unwrap();
        let engine = FheEngine::new(server_key);

        let values = [10u8, 20, 30, 5, 25, 15];
        let encrypted_values: Vec<Vec<u8>> = values
            .iter()
            .map(|&v| {
                let enc = FheUint8::try_encrypt(v, &client_key).unwrap();
                bincode::serialize(&enc).unwrap()
            })
            .collect();
        let encrypted_refs: Vec<&[u8]> = encrypted_values.iter().map(|v| v.as_slice()).collect();

        let predicate = FhePredicate::InRange { min: 15, max: 25 };
        let result_bytes = engine.compute_count_if(&encrypted_refs, &predicate).unwrap();

        let result: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let decrypted: u8 = result.decrypt(&client_key);

        // Values in [15, 25]: 20, 25, 15 = 3
        assert_eq!(decrypted, 3);
    }

    #[test]
    fn test_hash_result_deterministic() {
        let data = vec![1, 2, 3, 4, 5];
        let hash1 = FheEngine::hash_result(&data);
        let hash2 = FheEngine::hash_result(&data);

        assert_eq!(hash1, hash2);

        let data2 = vec![1, 2, 3, 4, 6];
        let hash3 = FheEngine::hash_result(&data2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_deterministic_commitment() {
        let witness_hash = [42u8; 32];
        let operation = "Sum";
        let job_id = 12345u64;

        let commitment1 = FheEngine::deterministic_commitment(&witness_hash, operation, job_id);
        let commitment2 = FheEngine::deterministic_commitment(&witness_hash, operation, job_id);

        assert_eq!(commitment1, commitment2);

        // Different job_id should produce different commitment
        let commitment3 = FheEngine::deterministic_commitment(&witness_hash, operation, 99999);
        assert_ne!(commitment1, commitment3);
    }
}
