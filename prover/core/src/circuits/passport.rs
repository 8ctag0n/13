//! Passport Circuit - Age and identity verification
//!
//! Implements Threshold and RangeCheck operations for zk-passport

use anyhow::{Context, Result};
use zyberlink_fhe::prelude::{FheOrd, FheTryTrivialEncrypt, IfThenElse};
use zyberlink_fhe::{FheBool, FheUint8};

pub struct PassportCircuit;

impl PassportCircuit {
    /// Check if encrypted value meets threshold condition
    ///
    /// # Arguments
    /// * `encrypted_value` - Serialized FheUint8 ciphertext to check
    /// * `threshold` - Clear threshold value
    /// * `greater_or_equal` - If true, check value >= threshold; if false, check value < threshold
    ///
    /// # Returns
    /// * `Result<Vec<u8>>` - Serialized FheUint8 (1 if condition met, 0 otherwise)
    ///
    /// # Example
    /// ```ignore
    /// let age = encrypt(25);
    /// let result = PassportCircuit::compute_threshold(&age, 18, true)?;
    /// // decrypt(result) = true (25 >= 18)
    /// ```
    pub fn compute_threshold(
        encrypted_value: &[u8],
        threshold: u8,
        greater_or_equal: bool,
    ) -> Result<Vec<u8>> {
        // Deserialize the encrypted value
        let value_ct: FheUint8 = bincode::deserialize(encrypted_value)
            .context("Failed to deserialize encrypted value")?;

        // Create encrypted threshold (trivial encryption - no key needed)
        let threshold_ct = FheUint8::try_encrypt_trivial(threshold)
            .context("Failed to encrypt threshold value")?;

        // Perform comparison using FHE
        let result_bool: FheBool = if greater_or_equal {
            value_ct.ge(&threshold_ct) // >=
        } else {
            value_ct.lt(&threshold_ct) // <
        };

        // Convert FheBool to FheUint8 (1 if true, 0 if false)
        let one = FheUint8::try_encrypt_trivial(1u8)
            .context("Failed to encrypt trivial 1")?;
        let zero = FheUint8::try_encrypt_trivial(0u8)
            .context("Failed to encrypt trivial 0")?;
        let result_uint8 = result_bool.if_then_else(&one, &zero);

        // Serialize the uint8 result
        bincode::serialize(&result_uint8).context("Failed to serialize threshold result")
    }

    /// Verify encrypted value is within valid range [min, max]
    ///
    /// # Arguments
    /// * `encrypted_value` - Serialized FheUint8 ciphertext
    /// * `min` - Minimum allowed value (inclusive)
    /// * `max` - Maximum allowed value (inclusive)
    ///
    /// # Returns
    /// * `Result<Vec<u8>>` - Serialized FheUint8 (1 if min <= value <= max, 0 otherwise)
    ///
    /// # Example
    /// ```ignore
    /// let age = encrypt(25);
    /// let result = PassportCircuit::compute_range_check(&age, 18, 65)?;
    /// // decrypt(result) = true (18 <= 25 <= 65)
    /// ```
    pub fn compute_range_check(encrypted_value: &[u8], min: u8, max: u8) -> Result<Vec<u8>> {
        if min > max {
            anyhow::bail!("Invalid range: min ({}) > max ({})", min, max);
        }

        // Deserialize the encrypted value
        let value_ct: FheUint8 = bincode::deserialize(encrypted_value)
            .context("Failed to deserialize encrypted value")?;

        // Create encrypted min and max
        let min_ct = FheUint8::try_encrypt_trivial(min).context("Failed to encrypt min value")?;
        let max_ct = FheUint8::try_encrypt_trivial(max).context("Failed to encrypt max value")?;

        // Check: value >= min AND value <= max
        let ge_min: FheBool = value_ct.ge(&min_ct);
        let le_max: FheBool = value_ct.le(&max_ct);

        // Combine with AND
        let in_range_bool: FheBool = ge_min & le_max;

        // Convert FheBool to FheUint8 (1 if true, 0 if false)
        let one = FheUint8::try_encrypt_trivial(1u8)
            .context("Failed to encrypt trivial 1")?;
        let zero = FheUint8::try_encrypt_trivial(0u8)
            .context("Failed to encrypt trivial 0")?;
        let result_uint8 = in_range_bool.if_then_else(&one, &zero);

        bincode::serialize(&result_uint8).context("Failed to serialize range check result")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zyberlink_fhe::prelude::*;
    use zyberlink_fhe::tfhe_reexport::set_server_key;
    use zyberlink_fhe::{generate_keys, ClientKey, ServerKey};

    fn generate_test_keys() -> (ClientKey, ServerKey) {
        generate_keys().unwrap()
    }

    // THRESHOLD TESTS

    #[test]
    fn test_threshold_age_verification_adult() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        // User age: 25
        let age = FheUint8::try_encrypt(25u8, &client_key).unwrap();
        let age_bytes = bincode::serialize(&age).unwrap();

        // Check age >= 18
        let result_bytes = PassportCircuit::compute_threshold(&age_bytes, 18, true).unwrap();

        let result_uint8: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let is_adult: u8 = result_uint8.decrypt(&client_key);

        assert_eq!(is_adult, 1, "Age 25 should be >= 18 (expecting 1)");
    }

    #[test]
    fn test_threshold_age_verification_minor() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        // User age: 15
        let age = FheUint8::try_encrypt(15u8, &client_key).unwrap();
        let age_bytes = bincode::serialize(&age).unwrap();

        // Check age >= 18
        let result_bytes = PassportCircuit::compute_threshold(&age_bytes, 18, true).unwrap();

        let result_uint8: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let is_adult: u8 = result_uint8.decrypt(&client_key);

        assert_eq!(is_adult, 0, "Age 15 should NOT be >= 18 (expecting 0)");
    }

    #[test]
    fn test_threshold_exact_value() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        // User age: exactly 18
        let age = FheUint8::try_encrypt(18u8, &client_key).unwrap();
        let age_bytes = bincode::serialize(&age).unwrap();

        // Check age >= 18
        let result_bytes = PassportCircuit::compute_threshold(&age_bytes, 18, true).unwrap();

        let result_uint8: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let is_adult: u8 = result_uint8.decrypt(&client_key);

        assert_eq!(is_adult, 1, "Age 18 should be >= 18 (boundary case, expecting 1)");
    }

    #[test]
    fn test_threshold_less_than_mode() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        let value = FheUint8::try_encrypt(10u8, &client_key).unwrap();
        let value_bytes = bincode::serialize(&value).unwrap();

        // Check value < 20 (should be true)
        let result_bytes = PassportCircuit::compute_threshold(&value_bytes, 20, false).unwrap();

        let result_uint8: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let is_less: u8 = result_uint8.decrypt(&client_key);

        assert_eq!(is_less, 1, "10 should be < 20 (expecting 1)");
    }

    #[test]
    fn test_threshold_with_zero() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        let value = FheUint8::try_encrypt(0u8, &client_key).unwrap();
        let value_bytes = bincode::serialize(&value).unwrap();

        // Check 0 >= 0 (should be true)
        let result_bytes = PassportCircuit::compute_threshold(&value_bytes, 0, true).unwrap();

        let result_uint8: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let result: u8 = result_uint8.decrypt(&client_key);

        assert_eq!(result, 1, "0 should be >= 0 (expecting 1)");
    }

    // RANGE CHECK TESTS

    #[test]
    fn test_range_check_within_range() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        let age = FheUint8::try_encrypt(25u8, &client_key).unwrap();
        let age_bytes = bincode::serialize(&age).unwrap();

        // Check 18 <= age <= 65
        let result_bytes = PassportCircuit::compute_range_check(&age_bytes, 18, 65).unwrap();

        let result_uint8: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let in_range: u8 = result_uint8.decrypt(&client_key);

        assert_eq!(in_range, 1, "25 should be in range [18, 65] (expecting 1)");
    }

    #[test]
    fn test_range_check_below_min() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        let age = FheUint8::try_encrypt(15u8, &client_key).unwrap();
        let age_bytes = bincode::serialize(&age).unwrap();

        // Check 18 <= age <= 65
        let result_bytes = PassportCircuit::compute_range_check(&age_bytes, 18, 65).unwrap();

        let result_uint8: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let in_range: u8 = result_uint8.decrypt(&client_key);

        assert_eq!(in_range, 0, "15 should NOT be in range [18, 65] (expecting 0)");
    }

    #[test]
    fn test_range_check_above_max() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        let age = FheUint8::try_encrypt(70u8, &client_key).unwrap();
        let age_bytes = bincode::serialize(&age).unwrap();

        // Check 18 <= age <= 65
        let result_bytes = PassportCircuit::compute_range_check(&age_bytes, 18, 65).unwrap();

        let result_uint8: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let in_range: u8 = result_uint8.decrypt(&client_key);

        assert_eq!(in_range, 0, "70 should NOT be in range [18, 65] (expecting 0)");
    }

    #[test]
    fn test_range_check_boundary_min() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        let age = FheUint8::try_encrypt(18u8, &client_key).unwrap();
        let age_bytes = bincode::serialize(&age).unwrap();

        let result_bytes = PassportCircuit::compute_range_check(&age_bytes, 18, 65).unwrap();

        let result_uint8: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let in_range: u8 = result_uint8.decrypt(&client_key);

        assert_eq!(in_range, 1, "18 should be in range [18, 65] (min boundary, expecting 1)");
    }

    #[test]
    fn test_range_check_boundary_max() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        let age = FheUint8::try_encrypt(65u8, &client_key).unwrap();
        let age_bytes = bincode::serialize(&age).unwrap();

        let result_bytes = PassportCircuit::compute_range_check(&age_bytes, 18, 65).unwrap();

        let result_uint8: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let in_range: u8 = result_uint8.decrypt(&client_key);

        assert_eq!(in_range, 1, "65 should be in range [18, 65] (max boundary, expecting 1)");
    }

    #[test]
    fn test_range_check_invalid_range_fails() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        let value = FheUint8::try_encrypt(50u8, &client_key).unwrap();
        let value_bytes = bincode::serialize(&value).unwrap();

        // min > max should fail
        let result = PassportCircuit::compute_range_check(&value_bytes, 100, 50);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid range"));
    }

    // PERFORMANCE BENCHMARKS

    #[test]
    #[ignore]
    fn test_threshold_performance() {
        use std::time::Instant;

        println!("Generating FHE keys (this is slow, ~30-40s)...");
        let key_gen_start = Instant::now();
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);
        println!("Key generation took: {:?}", key_gen_start.elapsed());

        println!("Encrypting test data...");
        let encrypt_start = Instant::now();
        let age = FheUint8::try_encrypt(25u8, &client_key).unwrap();
        let age_bytes = bincode::serialize(&age).unwrap();
        println!("Encryption took: {:?}", encrypt_start.elapsed());

        // MEASURE ONLY THE THRESHOLD OPERATION (not key gen or encryption)
        println!("Running Threshold operation...");
        let start = Instant::now();
        let result_bytes = PassportCircuit::compute_threshold(&age_bytes, 18, true).unwrap();
        let duration = start.elapsed();

        println!("Decrypting result...");
        let decrypt_start = Instant::now();
        let result_uint8: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let is_adult: u8 = result_uint8.decrypt(&client_key);
        println!("Decryption took: {:?}", decrypt_start.elapsed());

        assert_eq!(is_adult, 1);

        println!("\n=== PERFORMANCE RESULT ===");
        println!("Threshold operation (FHE comparison) took: {:?}", duration);
        println!("NOTE: FHE comparisons are computationally expensive");
        println!("Production optimizations: hardware acceleration, parameter tuning, caching");

        // FHE comparison operations are inherently slow (50-60s with default parameters)
        // This is expected behavior for encrypted comparisons
        // In production: use hardware acceleration (GPU/FPGA) and optimized parameters
        assert!(
            duration.as_secs() < 120,
            "Performance target: Threshold < 120s (FHE comparison baseline)"
        );
    }

    #[test]
    #[ignore]
    fn test_range_check_performance() {
        use std::time::Instant;

        println!("Generating FHE keys (this is slow, ~30-40s)...");
        let key_gen_start = Instant::now();
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);
        println!("Key generation took: {:?}", key_gen_start.elapsed());

        println!("Encrypting test data...");
        let encrypt_start = Instant::now();
        let age = FheUint8::try_encrypt(25u8, &client_key).unwrap();
        let age_bytes = bincode::serialize(&age).unwrap();
        println!("Encryption took: {:?}", encrypt_start.elapsed());

        // MEASURE ONLY THE RANGE CHECK OPERATION (not key gen or encryption)
        println!("Running RangeCheck operation...");
        let start = Instant::now();
        let result_bytes = PassportCircuit::compute_range_check(&age_bytes, 18, 65).unwrap();
        let duration = start.elapsed();

        println!("Decrypting result...");
        let decrypt_start = Instant::now();
        let result_uint8: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let in_range: u8 = result_uint8.decrypt(&client_key);
        println!("Decryption took: {:?}", decrypt_start.elapsed());

        assert_eq!(in_range, 1);

        println!("\n=== PERFORMANCE RESULT ===");
        println!(
            "RangeCheck operation (2 comparisons + AND) took: {:?}",
            duration
        );
        println!("NOTE: RangeCheck = 2 FHE comparisons + 1 AND gate (computationally expensive)");
        println!("Production optimizations: hardware acceleration, parameter tuning, caching");

        // RangeCheck does 2 comparisons + 1 AND, expect ~100-120s with default parameters
        // This is expected behavior for encrypted range verification
        // In production: use hardware acceleration (GPU/FPGA) and optimized parameters
        assert!(
            duration.as_secs() < 180,
            "Performance target: RangeCheck < 180s (FHE double comparison baseline)"
        );
    }
}
