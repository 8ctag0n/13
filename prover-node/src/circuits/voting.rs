//! Voting Circuit - Private DAO voting and tallying
//!
//! Implements CountIf and Histogram operations for vote counting

use anyhow::{Result, Context};
use cypherlink_types::fhe::FhePredicate;
use tfhe::{FheUint8, FheBool};
use tfhe::prelude::*;
use super::passport::PassportCircuit;

pub struct VotingCircuit;

impl VotingCircuit {
    /// Count how many encrypted values satisfy a predicate
    ///
    /// # Arguments
    /// * `encrypted_inputs` - Vector of serialized FheUint8 ciphertexts
    /// * `predicate` - The condition to evaluate
    ///
    /// # Returns
    /// * `Result<Vec<u8>>` - Serialized FheUint8 containing the count
    ///
    /// # Example
    /// ```ignore
    /// // Count votes for option 3
    /// let votes = vec![encrypt(1), encrypt(3), encrypt(3), encrypt(2)];
    /// let predicate = FhePredicate::Equals(3);
    /// let count = VotingCircuit::compute_count_if(votes, &predicate)?;
    /// // decrypt(count) = 2
    /// ```
    pub fn compute_count_if(
        encrypted_inputs: Vec<&[u8]>,
        predicate: &FhePredicate,
    ) -> Result<Vec<u8>> {
        if encrypted_inputs.is_empty() {
            anyhow::bail!("Cannot count if on empty input list");
        }

        // Initialize counter to 0
        let mut count = FheUint8::try_encrypt_trivial(0u8)
            .context("Failed to initialize counter")?;

        // For each encrypted value, evaluate predicate and add to count
        for input_bytes in encrypted_inputs {
            let matches = Self::evaluate_predicate(input_bytes, predicate)?;

            // Convert FheBool to FheUint8 (0 or 1)
            let one = FheUint8::try_encrypt_trivial(1u8)?;
            let zero = FheUint8::try_encrypt_trivial(0u8)?;
            let increment = matches.if_then_else(&one, &zero);

            // Add to count
            count = &count + &increment;
        }

        bincode::serialize(&count)
            .context("Failed to serialize count result")
    }

    /// Evaluate a predicate on an encrypted value
    ///
    /// Returns FheBool: true if predicate satisfied, false otherwise
    fn evaluate_predicate(encrypted_value: &[u8], predicate: &FhePredicate) -> Result<FheBool> {
        match predicate {
            FhePredicate::Equals(value) => {
                // Deserialize input
                let value_ct: FheUint8 = bincode::deserialize(encrypted_value)
                    .context("Failed to deserialize value")?;

                // Create encrypted constant
                let target_ct = FheUint8::try_encrypt_trivial(*value)
                    .context("Failed to encrypt target value")?;

                // Compare: value == target
                Ok(value_ct.eq(&target_ct))
            }

            FhePredicate::GreaterThan(threshold) => {
                // Use PassportCircuit::compute_threshold with opposite check
                // GreaterThan means NOT (value < threshold OR value == threshold)
                // Equivalently: value >= (threshold + 1)
                let adjusted_threshold = threshold.saturating_add(1);

                let result_bytes = PassportCircuit::compute_threshold(
                    encrypted_value,
                    adjusted_threshold,
                    true, // greater_or_equal
                )?;

                bincode::deserialize(&result_bytes)
                    .context("Failed to deserialize threshold result")
            }

            FhePredicate::LessThan(threshold) => {
                // Use PassportCircuit::compute_threshold
                let result_bytes = PassportCircuit::compute_threshold(
                    encrypted_value,
                    *threshold,
                    false, // less_than
                )?;

                bincode::deserialize(&result_bytes)
                    .context("Failed to deserialize threshold result")
            }

            FhePredicate::InRange { min, max } => {
                // Use PassportCircuit::compute_range_check
                let result_bytes = PassportCircuit::compute_range_check(
                    encrypted_value,
                    *min,
                    *max,
                )?;

                bincode::deserialize(&result_bytes)
                    .context("Failed to deserialize range check result")
            }

            FhePredicate::NotEquals(value) => {
                // Deserialize input
                let value_ct: FheUint8 = bincode::deserialize(encrypted_value)
                    .context("Failed to deserialize value")?;

                // Create encrypted constant
                let target_ct = FheUint8::try_encrypt_trivial(*value)
                    .context("Failed to encrypt target value")?;

                // Compare: value != target (negation of equals)
                let eq_result = value_ct.eq(&target_ct);
                Ok(!eq_result)
            }
        }
    }

    // TODO: Agent 2 will implement Histogram on Day 5
    // Depends on: Self::compute_count_if
    //
    // pub fn compute_histogram(
    //     encrypted_inputs: Vec<&[u8]>,
    //     bins: &[HistogramBin],
    // ) -> Result<Vec<Vec<u8>>> {
    //     // For each bin, run compute_count_if with InRange predicate
    // }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tfhe::{ClientKey, ServerKey, ConfigBuilder, generate_keys, set_server_key};
    use cypherlink_types::fhe::FhePredicate;

    fn generate_test_keys() -> (ClientKey, ServerKey) {
        let config = ConfigBuilder::default().build();
        generate_keys(config)
    }

    #[test]
    fn test_count_if_equals() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        // Votes: [1, 3, 3, 2, 3, 1] -> count(3) = 3
        let votes = vec![1u8, 3, 3, 2, 3, 1];
        let mut encrypted = vec![];

        for vote in votes {
            let ct = FheUint8::try_encrypt(vote, &client_key).unwrap();
            encrypted.push(bincode::serialize(&ct).unwrap());
        }

        let input_refs: Vec<&[u8]> = encrypted.iter().map(|v| v.as_slice()).collect();

        let predicate = FhePredicate::Equals(3);
        let count_bytes = VotingCircuit::compute_count_if(input_refs, &predicate).unwrap();

        let count_ct: FheUint8 = bincode::deserialize(&count_bytes).unwrap();
        let count: u8 = count_ct.decrypt(&client_key);

        assert_eq!(count, 3);
    }

    #[test]
    fn test_count_if_greater_than() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        // Ages: [15, 20, 18, 25, 30, 17] -> count(age > 18) = 3 (20, 25, 30)
        let ages = vec![15u8, 20, 18, 25, 30, 17];
        let mut encrypted = vec![];

        for age in ages {
            let ct = FheUint8::try_encrypt(age, &client_key).unwrap();
            encrypted.push(bincode::serialize(&ct).unwrap());
        }

        let input_refs: Vec<&[u8]> = encrypted.iter().map(|v| v.as_slice()).collect();

        let predicate = FhePredicate::GreaterThan(18);
        let count_bytes = VotingCircuit::compute_count_if(input_refs, &predicate).unwrap();

        let count_ct: FheUint8 = bincode::deserialize(&count_bytes).unwrap();
        let count: u8 = count_ct.decrypt(&client_key);

        assert_eq!(count, 3);
    }

    #[test]
    fn test_count_if_less_than() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        // Values: [5, 10, 15, 3, 8] -> count(value < 10) = 3 (5, 3, 8)
        let values = vec![5u8, 10, 15, 3, 8];
        let mut encrypted = vec![];

        for value in values {
            let ct = FheUint8::try_encrypt(value, &client_key).unwrap();
            encrypted.push(bincode::serialize(&ct).unwrap());
        }

        let input_refs: Vec<&[u8]> = encrypted.iter().map(|v| v.as_slice()).collect();

        let predicate = FhePredicate::LessThan(10);
        let count_bytes = VotingCircuit::compute_count_if(input_refs, &predicate).unwrap();

        let count_ct: FheUint8 = bincode::deserialize(&count_bytes).unwrap();
        let count: u8 = count_ct.decrypt(&client_key);

        assert_eq!(count, 3);
    }

    #[test]
    fn test_count_if_in_range() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        // Ages: [15, 25, 35, 45, 70, 20] -> count(18 <= age <= 65) = 4
        let ages = vec![15u8, 25, 35, 45, 70, 20];
        let mut encrypted = vec![];

        for age in ages {
            let ct = FheUint8::try_encrypt(age, &client_key).unwrap();
            encrypted.push(bincode::serialize(&ct).unwrap());
        }

        let input_refs: Vec<&[u8]> = encrypted.iter().map(|v| v.as_slice()).collect();

        let predicate = FhePredicate::InRange { min: 18, max: 65 };
        let count_bytes = VotingCircuit::compute_count_if(input_refs, &predicate).unwrap();

        let count_ct: FheUint8 = bincode::deserialize(&count_bytes).unwrap();
        let count: u8 = count_ct.decrypt(&client_key);

        assert_eq!(count, 4);
    }

    #[test]
    fn test_count_if_not_equals() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        // Values: [1, 2, 2, 3, 2, 4] -> count(value != 2) = 3 (1, 3, 4)
        let values = vec![1u8, 2, 2, 3, 2, 4];
        let mut encrypted = vec![];

        for value in values {
            let ct = FheUint8::try_encrypt(value, &client_key).unwrap();
            encrypted.push(bincode::serialize(&ct).unwrap());
        }

        let input_refs: Vec<&[u8]> = encrypted.iter().map(|v| v.as_slice()).collect();

        let predicate = FhePredicate::NotEquals(2);
        let count_bytes = VotingCircuit::compute_count_if(input_refs, &predicate).unwrap();

        let count_ct: FheUint8 = bincode::deserialize(&count_bytes).unwrap();
        let count: u8 = count_ct.decrypt(&client_key);

        assert_eq!(count, 3);
    }

    #[test]
    fn test_count_if_none_match() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        // All zeros: [0, 0, 0] -> count(value == 1) = 0
        let values = vec![0u8, 0, 0];
        let mut encrypted = vec![];

        for value in values {
            let ct = FheUint8::try_encrypt(value, &client_key).unwrap();
            encrypted.push(bincode::serialize(&ct).unwrap());
        }

        let input_refs: Vec<&[u8]> = encrypted.iter().map(|v| v.as_slice()).collect();

        let predicate = FhePredicate::Equals(1);
        let count_bytes = VotingCircuit::compute_count_if(input_refs, &predicate).unwrap();

        let count_ct: FheUint8 = bincode::deserialize(&count_bytes).unwrap();
        let count: u8 = count_ct.decrypt(&client_key);

        assert_eq!(count, 0);
    }

    #[test]
    fn test_count_if_all_match() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        // All ones: [1, 1, 1, 1] -> count(value == 1) = 4
        let values = vec![1u8, 1, 1, 1];
        let mut encrypted = vec![];

        for value in values {
            let ct = FheUint8::try_encrypt(value, &client_key).unwrap();
            encrypted.push(bincode::serialize(&ct).unwrap());
        }

        let input_refs: Vec<&[u8]> = encrypted.iter().map(|v| v.as_slice()).collect();

        let predicate = FhePredicate::Equals(1);
        let count_bytes = VotingCircuit::compute_count_if(input_refs, &predicate).unwrap();

        let count_ct: FheUint8 = bincode::deserialize(&count_bytes).unwrap();
        let count: u8 = count_ct.decrypt(&client_key);

        assert_eq!(count, 4);
    }

    #[test]
    fn test_count_if_empty_fails() {
        let predicate = FhePredicate::Equals(1);
        let result = VotingCircuit::compute_count_if(vec![], &predicate);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_count_if_dao_voting_scenario() {
        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        // DAO voting: 50 members vote on 3 options (0, 1, 2)
        // Votes: 20x option 0, 18x option 1, 12x option 2
        let mut votes = vec![];
        votes.extend(vec![0u8; 20]);
        votes.extend(vec![1u8; 18]);
        votes.extend(vec![2u8; 12]);

        let mut encrypted = vec![];
        for vote in votes {
            let ct = FheUint8::try_encrypt(vote, &client_key).unwrap();
            encrypted.push(bincode::serialize(&ct).unwrap());
        }

        let input_refs: Vec<&[u8]> = encrypted.iter().map(|v| v.as_slice()).collect();

        // Count votes for option 0
        let predicate = FhePredicate::Equals(0);
        let count_bytes = VotingCircuit::compute_count_if(input_refs.clone(), &predicate).unwrap();
        let count_ct: FheUint8 = bincode::deserialize(&count_bytes).unwrap();
        let count_option_0: u8 = count_ct.decrypt(&client_key);

        // Count votes for option 1
        let predicate = FhePredicate::Equals(1);
        let count_bytes = VotingCircuit::compute_count_if(input_refs.clone(), &predicate).unwrap();
        let count_ct: FheUint8 = bincode::deserialize(&count_bytes).unwrap();
        let count_option_1: u8 = count_ct.decrypt(&client_key);

        // Count votes for option 2
        let predicate = FhePredicate::Equals(2);
        let count_bytes = VotingCircuit::compute_count_if(input_refs, &predicate).unwrap();
        let count_ct: FheUint8 = bincode::deserialize(&count_bytes).unwrap();
        let count_option_2: u8 = count_ct.decrypt(&client_key);

        assert_eq!(count_option_0, 20);
        assert_eq!(count_option_1, 18);
        assert_eq!(count_option_2, 12);

        // Verify total
        assert_eq!(count_option_0 + count_option_1 + count_option_2, 50);
    }

    #[test]
    #[ignore] // Only run with --ignored flag (slow test)
    fn test_count_if_performance() {
        use std::time::Instant;

        let (client_key, server_key) = generate_test_keys();
        set_server_key(server_key);

        // Encrypt 100 votes
        let mut votes = vec![];
        for i in 0..100 {
            let vote = if i < 60 { 1u8 } else { 0u8 }; // 60 votes for option 1
            let ct = FheUint8::try_encrypt(vote, &client_key).unwrap();
            votes.push(bincode::serialize(&ct).unwrap());
        }

        let input_refs: Vec<&[u8]> = votes.iter().map(|v| v.as_slice()).collect();

        let predicate = FhePredicate::Equals(1);

        let start = Instant::now();
        let count_bytes = VotingCircuit::compute_count_if(input_refs, &predicate).unwrap();
        let duration = start.elapsed();

        let count_ct: FheUint8 = bincode::deserialize(&count_bytes).unwrap();
        let count: u8 = count_ct.decrypt(&client_key);

        assert_eq!(count, 60);

        println!("CountIf(100 values) took: {:?}", duration);

        // CountIf involves comparison + accumulation, expect < 10s
        assert!(duration.as_secs() < 10, "Performance target: CountIf(100) < 10s");
    }
}
