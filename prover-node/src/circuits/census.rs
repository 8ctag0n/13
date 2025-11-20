//! Census Circuit - Privacy-preserving population statistics
//!
//! Implements Sum operation for network member counting

use anyhow::{Result, Context};
use tfhe::{FheUint8, FheUint16, FheUint32};
use tfhe::prelude::*;

pub struct CensusCircuit;

impl CensusCircuit {
    /// Sum multiple encrypted u8 values using FHE
    ///
    /// # Arguments
    /// * `encrypted_inputs` - Vector of serialized FheUint8 ciphertexts
    ///
    /// # Returns
    /// * `Result<Vec<u8>>` - Serialized FheUint8 containing the encrypted sum
    ///
    /// # Example
    /// ```ignore
    /// let inputs = vec![encrypt(1), encrypt(2), encrypt(3)];
    /// let result = CensusCircuit::compute_sum_u8(inputs)?;
    /// // decrypt(result) = 6
    /// ```
    pub fn compute_sum_u8(encrypted_inputs: Vec<&[u8]>) -> Result<Vec<u8>> {
        if encrypted_inputs.is_empty() {
            anyhow::bail!("Cannot compute sum of empty input list");
        }

        // Deserialize all input ciphertexts
        let ciphertexts: Result<Vec<FheUint8>, _> = encrypted_inputs
            .iter()
            .map(|bytes| {
                bincode::deserialize::<FheUint8>(bytes)
                    .context("Failed to deserialize FheUint8 ciphertext")
            })
            .collect();

        let ciphertexts = ciphertexts?;

        // Start with the first ciphertext
        let mut sum = ciphertexts[0].clone();

        // Add remaining ciphertexts using FHE addition
        for ct in &ciphertexts[1..] {
            sum = &sum + ct;
        }

        // Serialize the result
        bincode::serialize(&sum)
            .context("Failed to serialize sum result")
    }

    /// Sum with u16 output for larger sums (up to 65535)
    pub fn compute_sum_u16(encrypted_inputs: Vec<&[u8]>) -> Result<Vec<u8>> {
        if encrypted_inputs.is_empty() {
            anyhow::bail!("Cannot compute sum of empty input list");
        }

        // Deserialize as u8, then cast to u16
        let ciphertexts: Result<Vec<FheUint8>, _> = encrypted_inputs
            .iter()
            .map(|bytes| bincode::deserialize(bytes).context("Failed to deserialize"))
            .collect();

        let ciphertexts = ciphertexts?;

        // Cast first value to u16
        let mut sum: FheUint16 = ciphertexts[0].clone().cast_into();

        // Add remaining values (cast each to u16 before adding)
        for ct in &ciphertexts[1..] {
            let ct_u16: FheUint16 = ct.clone().cast_into();
            sum = &sum + &ct_u16;
        }

        bincode::serialize(&sum).context("Failed to serialize sum")
    }

    /// Sum with u32 output for very large sums
    pub fn compute_sum_u32(encrypted_inputs: Vec<&[u8]>) -> Result<Vec<u8>> {
        if encrypted_inputs.is_empty() {
            anyhow::bail!("Cannot compute sum of empty input list");
        }

        let ciphertexts: Result<Vec<FheUint8>, _> = encrypted_inputs
            .iter()
            .map(|bytes| bincode::deserialize(bytes).context("Failed to deserialize"))
            .collect();

        let ciphertexts = ciphertexts?;

        let mut sum: FheUint32 = ciphertexts[0].clone().cast_into();

        for ct in &ciphertexts[1..] {
            let ct_u32: FheUint32 = ct.clone().cast_into();
            sum = &sum + &ct_u32;
        }

        bincode::serialize(&sum).context("Failed to serialize sum")
    }

    /// Auto-detect which sum type to use based on expected max value
    pub fn auto_sum(encrypted_inputs: Vec<&[u8]>, expected_max_value: u8) -> Result<Vec<u8>> {
        let count = encrypted_inputs.len();
        let estimated_max = (count as u64) * (expected_max_value as u64);

        if estimated_max <= 255 {
            Self::compute_sum_u8(encrypted_inputs)
        } else if estimated_max <= 65535 {
            Self::compute_sum_u16(encrypted_inputs)
        } else {
            Self::compute_sum_u32(encrypted_inputs)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tfhe::{ClientKey, ServerKey, ConfigBuilder, generate_keys, set_server_key};
    use tfhe::prelude::{FheTryEncrypt, FheDecrypt};

    fn generate_test_keys() -> (ClientKey, ServerKey) {
        let config = ConfigBuilder::default().build();
        generate_keys(config)
    }

    #[test]
    fn test_sum_two_values() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        // Encrypt two values: 10 + 20
        let v1 = FheUint8::try_encrypt(10u8, &client_key).unwrap();
        let v2 = FheUint8::try_encrypt(20u8, &client_key).unwrap();

        let bytes1 = bincode::serialize(&v1).unwrap();
        let bytes2 = bincode::serialize(&v2).unwrap();

        // Compute sum
        let result_bytes = CensusCircuit::compute_sum_u8(vec![&bytes1, &bytes2]).unwrap();

        // Decrypt and verify
        let result_ct: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let decrypted: u8 = result_ct.decrypt(&client_key);

        assert_eq!(decrypted, 30, "10 + 20 should equal 30");
    }

    #[test]
    fn test_sum_single_value() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        let v1 = FheUint8::try_encrypt(42u8, &client_key).unwrap();
        let bytes1 = bincode::serialize(&v1).unwrap();

        let result_bytes = CensusCircuit::compute_sum_u8(vec![&bytes1]).unwrap();
        let result_ct: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let decrypted: u8 = result_ct.decrypt(&client_key);

        assert_eq!(decrypted, 42);
    }

    #[test]
    fn test_sum_multiple_values() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        // Sum: 1 + 1 + 1 + 1 + 1 = 5
        let mut inputs = vec![];
        for _ in 0..5 {
            let v = FheUint8::try_encrypt(1u8, &client_key).unwrap();
            inputs.push(bincode::serialize(&v).unwrap());
        }

        let input_refs: Vec<&[u8]> = inputs.iter().map(|v| v.as_slice()).collect();
        let result_bytes = CensusCircuit::compute_sum_u8(input_refs).unwrap();

        let result_ct: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let decrypted: u8 = result_ct.decrypt(&client_key);

        assert_eq!(decrypted, 5);
    }

    #[test]
    fn test_sum_empty_input_fails() {
        let result = CensusCircuit::compute_sum_u8(vec![]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_sum_with_zeros() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        let v1 = FheUint8::try_encrypt(0u8, &client_key).unwrap();
        let v2 = FheUint8::try_encrypt(0u8, &client_key).unwrap();
        let v3 = FheUint8::try_encrypt(5u8, &client_key).unwrap();

        let bytes1 = bincode::serialize(&v1).unwrap();
        let bytes2 = bincode::serialize(&v2).unwrap();
        let bytes3 = bincode::serialize(&v3).unwrap();

        let result_bytes = CensusCircuit::compute_sum_u8(vec![&bytes1, &bytes2, &bytes3]).unwrap();
        let result_ct: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let decrypted: u8 = result_ct.decrypt(&client_key);

        assert_eq!(decrypted, 5);
    }

    #[test]
    fn test_sum_u16_large_values() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        // Sum: 100 + 100 + 100 = 300 (overflow u8, needs u16)
        let v1 = FheUint8::try_encrypt(100u8, &client_key).unwrap();
        let v2 = FheUint8::try_encrypt(100u8, &client_key).unwrap();
        let v3 = FheUint8::try_encrypt(100u8, &client_key).unwrap();

        let bytes1 = bincode::serialize(&v1).unwrap();
        let bytes2 = bincode::serialize(&v2).unwrap();
        let bytes3 = bincode::serialize(&v3).unwrap();

        let result_bytes = CensusCircuit::compute_sum_u16(vec![&bytes1, &bytes2, &bytes3]).unwrap();
        let result_ct: FheUint16 = bincode::deserialize(&result_bytes).unwrap();
        let decrypted: u16 = result_ct.decrypt(&client_key);

        assert_eq!(decrypted, 300);
    }

    #[test]
    fn test_auto_sum_chooses_u8() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        let v1 = FheUint8::try_encrypt(10u8, &client_key).unwrap();
        let bytes1 = bincode::serialize(&v1).unwrap();

        // Max value 10, count 1, estimated 10 -> use u8
        let result_bytes = CensusCircuit::auto_sum(vec![&bytes1], 10).unwrap();
        let result_ct: FheUint8 = bincode::deserialize(&result_bytes).unwrap();
        let decrypted: u8 = result_ct.decrypt(&client_key);

        assert_eq!(decrypted, 10);
    }

    #[test]
    fn test_auto_sum_chooses_u16() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        // 10 values of 50 each = 500 (requires u16)
        let mut inputs = vec![];
        for _ in 0..10 {
            let v = FheUint8::try_encrypt(50u8, &client_key).unwrap();
            inputs.push(bincode::serialize(&v).unwrap());
        }

        let input_refs: Vec<&[u8]> = inputs.iter().map(|v| v.as_slice()).collect();

        // Max value 50, count 10, estimated 500 -> use u16
        let result_bytes = CensusCircuit::auto_sum(input_refs, 50).unwrap();
        let result_ct: FheUint16 = bincode::deserialize(&result_bytes).unwrap();
        let decrypted: u16 = result_ct.decrypt(&client_key);

        assert_eq!(decrypted, 500);
    }

    #[test]
    #[ignore] // Only run with --ignored flag (slow test)
    fn test_sum_100_inputs_performance() {
        use std::time::Instant;

        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        // Encrypt 100 values of "1"
        let mut inputs = vec![];
        for _ in 0..100 {
            let v = FheUint8::try_encrypt(1u8, &client_key).unwrap();
            inputs.push(bincode::serialize(&v).unwrap());
        }

        let input_refs: Vec<&[u8]> = inputs.iter().map(|v| v.as_slice()).collect();

        let start = Instant::now();
        let result_bytes = CensusCircuit::compute_sum_u16(input_refs).unwrap();
        let duration = start.elapsed();

        let result_ct: FheUint16 = bincode::deserialize(&result_bytes).unwrap();
        let decrypted: u16 = result_ct.decrypt(&client_key);

        assert_eq!(decrypted, 100);

        println!("Sum(100 inputs) took: {:?}", duration);
        assert!(duration.as_secs() < 5, "Performance target: Sum(100) < 5s");
    }
}
