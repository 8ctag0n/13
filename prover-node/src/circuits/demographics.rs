//! Demographics Circuit - Privacy-preserving statistical analysis
//!
//! Implements Average operation for age/demographic analysis

use super::census::CensusCircuit;
use anyhow::{Context, Result};

pub struct DemographicsCircuit;

impl DemographicsCircuit {
    /// Compute average of encrypted values
    ///
    /// Returns encrypted sum and plaintext count. Client decrypts sum
    /// and computes: average = decrypt(sum) / count
    ///
    /// # Arguments
    /// * `encrypted_inputs` - Vector of serialized FheUint8 ciphertexts
    ///
    /// # Returns
    /// * `Result<(Vec<u8>, u16)>` - Tuple of (encrypted_sum, count)
    ///
    /// # Example
    /// ```ignore
    /// let ages = vec![encrypt(20), encrypt(25), encrypt(30)];
    /// let (sum_encrypted, count) = DemographicsCircuit::compute_average(ages)?;
    /// let sum = decrypt(sum_encrypted); // 75
    /// let avg = sum / count; // 75 / 3 = 25
    /// ```
    pub fn compute_average(encrypted_inputs: Vec<&[u8]>) -> Result<(Vec<u8>, u16)> {
        if encrypted_inputs.is_empty() {
            anyhow::bail!("Cannot compute average of empty input list");
        }

        let count = encrypted_inputs.len() as u16;

        // Use CensusCircuit to compute the sum
        let encrypted_sum = CensusCircuit::compute_sum_u8(encrypted_inputs)
            .context("Failed to compute sum for average")?;

        Ok((encrypted_sum, count))
    }

    /// Compute average with u16 output for larger sums
    ///
    /// Use when sum might exceed 255
    pub fn compute_average_u16(encrypted_inputs: Vec<&[u8]>) -> Result<(Vec<u8>, u16)> {
        if encrypted_inputs.is_empty() {
            anyhow::bail!("Cannot compute average of empty input list");
        }

        let count = encrypted_inputs.len() as u16;

        let encrypted_sum = CensusCircuit::compute_sum_u16(encrypted_inputs)
            .context("Failed to compute sum for average")?;

        Ok((encrypted_sum, count))
    }

    /// Auto-detect which average type to use based on inputs
    pub fn auto_average(
        encrypted_inputs: Vec<&[u8]>,
        expected_max_value: u8,
    ) -> Result<(Vec<u8>, u16)> {
        if encrypted_inputs.is_empty() {
            anyhow::bail!("Cannot compute average of empty input list");
        }

        let count = encrypted_inputs.len() as u16;
        let estimated_max = (count as u64) * (expected_max_value as u64);

        let encrypted_sum = if estimated_max <= 255 {
            CensusCircuit::compute_sum_u8(encrypted_inputs)?
        } else if estimated_max <= 65535 {
            CensusCircuit::compute_sum_u16(encrypted_inputs)?
        } else {
            CensusCircuit::compute_sum_u32(encrypted_inputs)?
        };

        Ok((encrypted_sum, count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tfhe::prelude::*;
    use tfhe::{
        generate_keys, set_server_key, ClientKey, ConfigBuilder, FheUint16, FheUint8, ServerKey,
    };

    fn generate_test_keys() -> (ClientKey, ServerKey) {
        let config = ConfigBuilder::default().build();
        generate_keys(config)
    }

    #[test]
    fn test_average_basic() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        // Ages: [20, 25, 30] -> sum=75, count=3, avg=25
        let age1 = FheUint8::try_encrypt(20u8, &client_key).unwrap();
        let age2 = FheUint8::try_encrypt(25u8, &client_key).unwrap();
        let age3 = FheUint8::try_encrypt(30u8, &client_key).unwrap();

        let bytes1 = bincode::serialize(&age1).unwrap();
        let bytes2 = bincode::serialize(&age2).unwrap();
        let bytes3 = bincode::serialize(&age3).unwrap();

        let (sum_bytes, count) =
            DemographicsCircuit::compute_average(vec![&bytes1, &bytes2, &bytes3]).unwrap();

        let sum_ct: FheUint8 = bincode::deserialize(&sum_bytes).unwrap();
        let sum: u8 = sum_ct.decrypt(&client_key);

        assert_eq!(sum, 75);
        assert_eq!(count, 3);

        let average = sum / (count as u8);
        assert_eq!(average, 25);
    }

    #[test]
    fn test_average_single_value() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        let value = FheUint8::try_encrypt(42u8, &client_key).unwrap();
        let bytes = bincode::serialize(&value).unwrap();

        let (sum_bytes, count) = DemographicsCircuit::compute_average(vec![&bytes]).unwrap();

        let sum_ct: FheUint8 = bincode::deserialize(&sum_bytes).unwrap();
        let sum: u8 = sum_ct.decrypt(&client_key);

        assert_eq!(sum, 42);
        assert_eq!(count, 1);
        assert_eq!(sum / (count as u8), 42);
    }

    #[test]
    fn test_average_empty_fails() {
        let result = DemographicsCircuit::compute_average(vec![]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_average_with_u16() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        // Values: [50, 50, 50, 50] -> sum=200, count=4, avg=50
        let mut inputs = vec![];
        for _ in 0..4 {
            let v = FheUint8::try_encrypt(50u8, &client_key).unwrap();
            inputs.push(bincode::serialize(&v).unwrap());
        }

        let input_refs: Vec<&[u8]> = inputs.iter().map(|v| v.as_slice()).collect();

        let (sum_bytes, count) = DemographicsCircuit::compute_average_u16(input_refs).unwrap();

        let sum_ct: FheUint16 = bincode::deserialize(&sum_bytes).unwrap();
        let sum: u16 = sum_ct.decrypt(&client_key);

        assert_eq!(sum, 200);
        assert_eq!(count, 4);
        assert_eq!(sum / count, 50);
    }

    #[test]
    fn test_auto_average_chooses_u8() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        let v1 = FheUint8::try_encrypt(10u8, &client_key).unwrap();
        let v2 = FheUint8::try_encrypt(20u8, &client_key).unwrap();

        let bytes1 = bincode::serialize(&v1).unwrap();
        let bytes2 = bincode::serialize(&v2).unwrap();

        // Max value 20, count 2, estimated 40 -> use u8
        let (sum_bytes, count) =
            DemographicsCircuit::auto_average(vec![&bytes1, &bytes2], 20).unwrap();

        let sum_ct: FheUint8 = bincode::deserialize(&sum_bytes).unwrap();
        let sum: u8 = sum_ct.decrypt(&client_key);

        assert_eq!(sum, 30);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_auto_average_chooses_u16() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        // 10 values of 50 each = 500 (requires u16)
        let mut inputs = vec![];
        for _ in 0..10 {
            let v = FheUint8::try_encrypt(50u8, &client_key).unwrap();
            inputs.push(bincode::serialize(&v).unwrap());
        }

        let input_refs: Vec<&[u8]> = inputs.iter().map(|v| v.as_slice()).collect();

        let (sum_bytes, count) = DemographicsCircuit::auto_average(input_refs, 50).unwrap();

        let sum_ct: FheUint16 = bincode::deserialize(&sum_bytes).unwrap();
        let sum: u16 = sum_ct.decrypt(&client_key);

        assert_eq!(sum, 500);
        assert_eq!(count, 10);
        assert_eq!(sum / count, 50);
    }

    #[test]
    fn test_average_real_world_ages() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        // Realistic age distribution: [22, 35, 41, 28, 33]
        // sum = 159, count = 5, avg = 31.8 ≈ 31
        let ages = vec![22u8, 35, 41, 28, 33];
        let mut encrypted = vec![];

        for age in ages {
            let ct = FheUint8::try_encrypt(age, &client_key).unwrap();
            encrypted.push(bincode::serialize(&ct).unwrap());
        }

        let input_refs: Vec<&[u8]> = encrypted.iter().map(|v| v.as_slice()).collect();

        let (sum_bytes, count) = DemographicsCircuit::compute_average(input_refs).unwrap();

        let sum_ct: FheUint8 = bincode::deserialize(&sum_bytes).unwrap();
        let sum: u8 = sum_ct.decrypt(&client_key);

        assert_eq!(sum, 159);
        assert_eq!(count, 5);

        let avg = sum / (count as u8);
        assert_eq!(avg, 31); // integer division
    }

    #[test]
    #[ignore] // Only run with --ignored flag (slow test)
    fn test_average_performance_100_values() {
        use std::time::Instant;

        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        // Encrypt 100 ages around 25
        let mut inputs = vec![];
        for _ in 0..100 {
            let v = FheUint8::try_encrypt(25u8, &client_key).unwrap();
            inputs.push(bincode::serialize(&v).unwrap());
        }

        let input_refs: Vec<&[u8]> = inputs.iter().map(|v| v.as_slice()).collect();

        let start = Instant::now();
        let (sum_bytes, count) = DemographicsCircuit::compute_average_u16(input_refs).unwrap();
        let duration = start.elapsed();

        let sum_ct: FheUint16 = bincode::deserialize(&sum_bytes).unwrap();
        let sum: u16 = sum_ct.decrypt(&client_key);

        assert_eq!(sum, 2500);
        assert_eq!(count, 100);
        assert_eq!(sum / count, 25);

        println!("Average(100 values) took: {:?}", duration);

        // Average should take similar time to Sum(100) < 5s
        assert!(
            duration.as_secs() < 7,
            "Performance target: Average(100) < 7s"
        );
    }
}
