/// FHE (Fully Homomorphic Encryption) computation engine for prover node
///
/// This module provides FHE computation capabilities using TFHE-rs (Concrete),
/// allowing provers to perform computations on encrypted data without decryption.
///
/// Architecture:
/// - Client generates keys and encrypts data
/// - Prover receives encrypted data and server key
/// - Prover performs homomorphic operations (add, multiply, etc.)
/// - Result remains encrypted and is returned to client
/// - Client decrypts final result with their private key
use anyhow::{anyhow, Context, Result};
use sha3::{Digest, Sha3_256};
use tfhe::{generate_keys, set_server_key, ClientKey, ConfigBuilder, FheUint8, ServerKey};

/// FHE computation engine for prover node
///
/// Wraps TFHE-rs functionality and provides a clean API for:
/// - Homomorphic arithmetic operations
/// - Result hashing for consensus
/// - Serialization/deserialization for on-chain data
pub struct FheEngine {
    server_key: ServerKey,
}

impl FheEngine {
    /// Initialize engine with server key
    ///
    /// The server key allows performing homomorphic operations without
    /// access to the client's private key.
    ///
    /// # Arguments
    /// * `server_key` - Public key for homomorphic computations
    ///
    /// # Example
    /// ```ignore
    /// let (_, server_key) = generate_fhe_keys().unwrap();
    /// let engine = FheEngine::new(server_key);
    /// ```
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

    /// Perform FHE addition operation
    ///
    /// Adds a constant to an encrypted value without decryption.
    ///
    /// # Arguments
    /// * `encrypted_input` - Serialized encrypted FheUint8
    /// * `constant` - Plain constant to add
    ///
    /// # Returns
    /// Serialized encrypted result
    ///
    /// # Example
    /// ```ignore
    /// let encrypted_42 = bincode::serialize(&FheUint8::encrypt(42u8, &client_key)).unwrap();
    /// let result = engine.compute_add(&encrypted_42, 10).unwrap();
    /// // result is encrypted(52)
    /// ```
    pub fn compute_add(&self, encrypted_input: &[u8], constant: u8) -> Result<Vec<u8>> {
        // Deserialize input ciphertext
        let ciphertext: FheUint8 = bincode::deserialize(encrypted_input)
            .map_err(|e| anyhow!("Failed to deserialize input ciphertext: {}", e))?;

        // Perform FHE addition
        // This happens entirely on encrypted data
        let result = ciphertext + constant;

        // Serialize result for transmission
        let result_bytes = bincode::serialize(&result)
            .map_err(|e| anyhow!("Failed to serialize result: {}", e))?;

        Ok(result_bytes)
    }

    /// Perform FHE multiplication operation
    ///
    /// Multiplies an encrypted value by a constant without decryption.
    ///
    /// # Arguments
    /// * `encrypted_input` - Serialized encrypted FheUint8
    /// * `constant` - Plain constant to multiply by
    ///
    /// # Returns
    /// Serialized encrypted result
    pub fn compute_multiply(&self, encrypted_input: &[u8], constant: u8) -> Result<Vec<u8>> {
        let ciphertext: FheUint8 = bincode::deserialize(encrypted_input)
            .context("Failed to deserialize input for multiplication")?;

        let result = ciphertext * constant;

        let result_bytes =
            bincode::serialize(&result).context("Failed to serialize multiplication result")?;

        Ok(result_bytes)
    }

    /// Perform FHE subtraction operation
    ///
    /// Subtracts a constant from an encrypted value without decryption.
    ///
    /// # Arguments
    /// * `encrypted_input` - Serialized encrypted FheUint8
    /// * `constant` - Plain constant to subtract
    ///
    /// # Returns
    /// Serialized encrypted result
    pub fn compute_subtract(&self, encrypted_input: &[u8], constant: u8) -> Result<Vec<u8>> {
        let ciphertext: FheUint8 = bincode::deserialize(encrypted_input)
            .context("Failed to deserialize input for subtraction")?;

        let result = ciphertext - constant;

        let result_bytes =
            bincode::serialize(&result).context("Failed to serialize subtraction result")?;

        Ok(result_bytes)
    }

    /// Perform FHE addition of two encrypted values
    ///
    /// Adds two encrypted values without decryption.
    ///
    /// # Arguments
    /// * `encrypted_a` - First serialized encrypted FheUint8
    /// * `encrypted_b` - Second serialized encrypted FheUint8
    ///
    /// # Returns
    /// Serialized encrypted result (a + b)
    pub fn compute_add_encrypted(&self, encrypted_a: &[u8], encrypted_b: &[u8]) -> Result<Vec<u8>> {
        let ciphertext_a: FheUint8 = bincode::deserialize(encrypted_a)
            .context("Failed to deserialize first input for encrypted addition")?;

        let ciphertext_b: FheUint8 = bincode::deserialize(encrypted_b)
            .context("Failed to deserialize second input for encrypted addition")?;

        let result = ciphertext_a + ciphertext_b;

        let result_bytes =
            bincode::serialize(&result).context("Failed to serialize encrypted addition result")?;

        Ok(result_bytes)
    }

    /// Hash encrypted result for consensus checking
    ///
    /// Multiple provers should produce the same result for the same input.
    /// This hash allows consensus verification without revealing the encrypted data.
    ///
    /// # Arguments
    /// * `result_bytes` - Serialized encrypted result
    ///
    /// # Returns
    /// SHA3-256 hash of the result (32 bytes)
    ///
    /// # Example
    /// ```ignore
    /// let hash = FheEngine::hash_result(&encrypted_result);
    /// // Submit hash to chain for consensus
    /// ```
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
    /// CSPRNG state used in bootstrapping operations. Different prover processes
    /// produce different ciphertext bytes even with identical inputs, though all
    /// results decrypt to the same plaintext value.
    ///
    /// This is a known limitation that requires one of these production solutions:
    /// 1. Verifiable FHE with ZK proofs (e.g., Zama Concrete, Fhenix)
    /// 2. Threshold FHE where provers share key fragments
    /// 3. Client-side verification by decrypting multiple results
    ///
    /// For this PoC, we use a deterministic commitment based on inputs:
    /// - All provers working on the same job produce identical commitment
    /// - This proves they processed the same data, not that computation is correct
    /// - Production systems MUST implement proper verification
    ///
    /// # Arguments
    /// * `witness_hash` - Hash of witness data (from blockchain job)
    /// * `operation` - FHE operation name
    /// * `job_id` - Job identifier
    ///
    /// # Returns
    /// SHA3-256 deterministic commitment (32 bytes)
    pub fn deterministic_commitment(
        witness_hash: &[u8; 32],
        operation: &str,
        job_id: u64,
    ) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(b"ZYBERLINK_FHE_COMMITMENT_V1:");
        hasher.update(witness_hash);
        hasher.update(operation.as_bytes());
        hasher.update(&job_id.to_le_bytes());
        let hash = hasher.finalize();

        let mut output = [0u8; 32];
        output.copy_from_slice(&hash);
        output
    }

    /// Get reference to server key
    pub fn server_key(&self) -> &ServerKey {
        &self.server_key
    }
}

/// Generate FHE keypair (client key + server key)
///
/// For testing and demo purposes. In production:
/// - Client generates their own keypair
/// - Only server key is shared with provers
/// - Client key remains private
///
/// # Returns
/// Tuple of (ClientKey, ServerKey)
///
/// # Performance Note
/// Key generation is slow (~1 second), so cache keys in production.
pub fn generate_fhe_keys() -> Result<(ClientKey, ServerKey)> {
    let config = ConfigBuilder::default().build();
    Ok(generate_keys(config))
}

/// Serialize server key to bytes for storage/transmission
///
/// Server keys are large (~50MB), so consider compression or
/// storing only the hash in job metadata.
pub fn serialize_server_key(server_key: &ServerKey) -> Result<Vec<u8>> {
    bincode::serialize(server_key).context("Failed to serialize server key")
}

/// Deserialize server key from bytes
pub fn deserialize_server_key(bytes: &[u8]) -> Result<ServerKey> {
    bincode::deserialize(bytes).context("Failed to deserialize server key")
}

/// Serialize client key to bytes for storage
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

    #[test]
    fn test_fhe_engine_add() {
        // Generate keys
        let (client_key, server_key) = generate_fhe_keys().unwrap();

        // Client encrypts input
        let value = 42u8;
        let encrypted = FheUint8::try_encrypt(value, &client_key).unwrap();
        let encrypted_bytes = bincode::serialize(&encrypted).unwrap();

        // Prover computes
        let engine = FheEngine::new(server_key);
        let result_bytes = engine.compute_add(&encrypted_bytes, 10).unwrap();

        // Client decrypts to verify
        let result_ciphertext: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let decrypted: u8 = result_ciphertext.decrypt(&client_key);

        assert_eq!(decrypted, 52);
    }

    #[test]
    fn test_fhe_engine_multiply() {
        let (client_key, server_key) = generate_fhe_keys().unwrap();

        let value = 7u8;
        let encrypted = FheUint8::try_encrypt(value, &client_key).unwrap();
        let encrypted_bytes = bincode::serialize(&encrypted).unwrap();

        let engine = FheEngine::new(server_key);
        let result_bytes = engine.compute_multiply(&encrypted_bytes, 6).unwrap();

        let result_ciphertext: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let decrypted: u8 = result_ciphertext.decrypt(&client_key);

        assert_eq!(decrypted, 42);
    }

    #[test]
    fn test_fhe_engine_subtract() {
        let (client_key, server_key) = generate_fhe_keys().unwrap();

        let value = 100u8;
        let encrypted = FheUint8::try_encrypt(value, &client_key).unwrap();
        let encrypted_bytes = bincode::serialize(&encrypted).unwrap();

        let engine = FheEngine::new(server_key);
        let result_bytes = engine.compute_subtract(&encrypted_bytes, 42).unwrap();

        let result_ciphertext: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let decrypted: u8 = result_ciphertext.decrypt(&client_key);

        assert_eq!(decrypted, 58);
    }

    #[test]
    fn test_fhe_engine_add_encrypted() {
        let (client_key, server_key) = generate_fhe_keys().unwrap();

        // Encrypt two values
        let value_a = 100u8;
        let value_b = 50u8;
        let encrypted_a = FheUint8::try_encrypt(value_a, &client_key).unwrap();
        let encrypted_b = FheUint8::try_encrypt(value_b, &client_key).unwrap();

        let bytes_a = bincode::serialize(&encrypted_a).unwrap();
        let bytes_b = bincode::serialize(&encrypted_b).unwrap();

        // Prover adds encrypted values
        let engine = FheEngine::new(server_key);
        let result_bytes = engine.compute_add_encrypted(&bytes_a, &bytes_b).unwrap();

        // Client decrypts
        let result_ciphertext: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let decrypted: u8 = result_ciphertext.decrypt(&client_key);

        assert_eq!(decrypted, 150);
    }

    #[test]
    fn test_result_hashing() {
        let data = vec![1, 2, 3, 4, 5];
        let hash1 = FheEngine::hash_result(&data);
        let hash2 = FheEngine::hash_result(&data);

        // Same input should produce same hash
        assert_eq!(hash1, hash2);

        // Different input should produce different hash
        let data2 = vec![1, 2, 3, 4, 6];
        let hash3 = FheEngine::hash_result(&data2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_key_serialization() {
        let (client_key, server_key) = generate_fhe_keys().unwrap();

        // Test server key serialization
        let server_bytes = serialize_server_key(&server_key).unwrap();
        let server_key_restored = deserialize_server_key(&server_bytes).unwrap();

        // Test client key serialization
        let client_bytes = serialize_client_key(&client_key).unwrap();
        let client_key_restored = deserialize_client_key(&client_bytes).unwrap();

        // Verify keys work after round-trip
        let value = 123u8;
        let encrypted = FheUint8::try_encrypt(value, &client_key_restored).unwrap();

        let engine = FheEngine::new(server_key_restored);
        let encrypted_bytes = bincode::serialize(&encrypted).unwrap();
        let result_bytes = engine.compute_add(&encrypted_bytes, 1).unwrap();

        let result: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let decrypted: u8 = result.decrypt(&client_key_restored);

        assert_eq!(decrypted, 124);
    }

    #[test]
    fn test_fhe_performance() {
        use std::time::Instant;

        let (client_key, server_key) = generate_fhe_keys().unwrap();
        let engine = FheEngine::new(server_key);

        // Encrypt
        let value = 42u8;
        let encrypted = FheUint8::try_encrypt(value, &client_key).unwrap();
        let encrypted_bytes = bincode::serialize(&encrypted).unwrap();

        // Benchmark computation
        let start = Instant::now();
        let result_bytes = engine.compute_add(&encrypted_bytes, 10).unwrap();
        let duration = start.elapsed();

        println!("FHE addition took: {:?}", duration);

        // Verify correctness
        let result: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let decrypted: u8 = result.decrypt(&client_key);
        assert_eq!(decrypted, 52);

        // Assert reasonable performance (< 500ms as per spec)
        assert!(
            duration.as_millis() < 500,
            "FHE computation too slow: {:?}",
            duration
        );
    }
}
