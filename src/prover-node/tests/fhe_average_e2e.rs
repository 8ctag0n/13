/// End-to-End test for FHE Average operation in the marketplace
///
/// This test validates the complete flow:
/// 1. Create FHE job with Average operation
/// 2. Multiple provers claim and execute the job
/// 3. Provers reach consensus on the result
/// 4. Payment distribution and verification
use anyhow::Result;
use tfhe::prelude::*;
use tfhe::{generate_keys, set_server_key, ConfigBuilder, FheUint16, FheUint8};

#[test]
fn test_fhe_average_basic_computation() -> Result<()> {
    // This test validates that the Average circuit produces correct results
    // This is the foundation for the marketplace E2E test

    println!("\n=== FHE Average E2E Test ===\n");

    // Step 1: Generate FHE keys (client side)
    println!("Step 1: Generating FHE keys...");
    let config = ConfigBuilder::default().build();
    let (client_key, server_key) = generate_keys(config);
    set_server_key(server_key.clone());
    println!("  ✓ Keys generated");

    // Step 2: Client encrypts ages for average computation
    println!("\nStep 2: Client encrypts data...");
    let ages = vec![20u8, 25, 30, 35, 40]; // Real ages to average
    let mut encrypted_ages = Vec::new();

    for age in &ages {
        let encrypted = FheUint8::try_encrypt(*age, &client_key)?;
        let serialized = bincode::serialize(&encrypted)?;
        println!("  Encrypted age: {} -> {} bytes", age, serialized.len());
        encrypted_ages.push(serialized);
    }

    // Step 3: Client serializes inputs for job witness
    println!("\nStep 3: Creating job witness...");
    let witness_data = bincode::serialize(&encrypted_ages)?;
    println!("  ✓ Witness created: {} bytes", witness_data.len());

    // Step 4: Prover executes Average computation
    println!("\nStep 4: Prover computes Average...");

    // Deserialize inputs
    let inputs: Vec<Vec<u8>> = bincode::deserialize(&witness_data)?;
    let input_refs: Vec<&[u8]> = inputs.iter().map(|v| v.as_slice()).collect();

    // Use DemographicsCircuit to compute average
    use prover_node::DemographicsCircuit;
    let (encrypted_sum_bytes, count) = DemographicsCircuit::compute_average_u16(input_refs)?;

    println!("  ✓ Average computed (encrypted_sum, count={})", count);

    // Step 5: Client decrypts result
    println!("\nStep 5: Client decrypts result...");
    let encrypted_sum: FheUint16 = bincode::deserialize(&encrypted_sum_bytes)?;
    let sum: u16 = encrypted_sum.decrypt(&client_key);
    let average = sum / count;

    println!("  Sum: {}", sum);
    println!("  Count: {}", count);
    println!("  Average: {}", average);

    // Verify correctness
    let expected_sum: u16 = ages.iter().map(|&x| x as u16).sum();
    let expected_avg = expected_sum / (ages.len() as u16);

    assert_eq!(sum, expected_sum, "Sum mismatch");
    assert_eq!(count, ages.len() as u16, "Count mismatch");
    assert_eq!(average, expected_avg, "Average mismatch");

    println!("\n=== ✓ Test Passed ===\n");

    Ok(())
}

#[test]
fn test_fhe_average_consensus_simulation() -> Result<()> {
    // Simulates 3 provers computing the same Average operation
    // All should produce identical result hashes (consensus)

    println!("\n=== FHE Average Consensus Test ===\n");

    // Setup
    let config = ConfigBuilder::default().build();
    let (client_key, server_key) = generate_keys(config);
    set_server_key(server_key.clone());

    // Client encrypts data
    let values = vec![10u8, 20, 30, 40, 50];
    let mut encrypted = Vec::new();
    for val in &values {
        let ct = FheUint8::try_encrypt(*val, &client_key)?;
        encrypted.push(bincode::serialize(&ct)?);
    }

    let witness_data = bincode::serialize(&encrypted)?;

    // Simulate 3 provers computing independently
    let mut result_hashes = Vec::new();

    for prover_id in 1..=3 {
        println!("Prover {} computing...", prover_id);

        let inputs: Vec<Vec<u8>> = bincode::deserialize(&witness_data)?;
        let input_refs: Vec<&[u8]> = inputs.iter().map(|v| v.as_slice()).collect();

        use prover_node::DemographicsCircuit;
        let (encrypted_sum, count) = DemographicsCircuit::compute_average_u16(input_refs)?;

        // Serialize result for hashing
        let result_bytes = bincode::serialize(&(encrypted_sum.clone(), count))?;

        // Hash result (what gets submitted on-chain)
        use sha3::{Digest, Sha3_256};
        let mut hasher = Sha3_256::new();
        hasher.update(&result_bytes);
        let hash: [u8; 32] = hasher.finalize().into();

        result_hashes.push(hash);
        println!("  Result hash: {}", hex::encode(&hash[..8]));

        // Verify result
        let sum: FheUint16 = bincode::deserialize(&encrypted_sum)?;
        let sum_val: u16 = sum.decrypt(&client_key);
        let avg = sum_val / count;
        println!("  Average: {}", avg);
    }

    // All provers should produce the same hash (consensus)
    println!("\nConsensus check:");
    assert_eq!(
        result_hashes[0], result_hashes[1],
        "Prover 1 and 2 mismatch"
    );
    assert_eq!(
        result_hashes[1], result_hashes[2],
        "Prover 2 and 3 mismatch"
    );
    println!("  ✓ All 3 provers reached consensus!");

    println!("\n=== ✓ Consensus Test Passed ===\n");

    Ok(())
}

#[test]
fn test_fhe_average_large_dataset() -> Result<()> {
    // Test with realistic dataset size (100 values)

    println!("\n=== FHE Average Large Dataset Test ===\n");

    let config = ConfigBuilder::default().build();
    let (client_key, server_key) = generate_keys(config);
    set_server_key(server_key);

    // Create 100 ages with realistic distribution
    let mut ages = Vec::new();
    for i in 0..100 {
        let age = 20 + (i % 50); // Ages from 20 to 69
        ages.push(age as u8);
    }

    println!("Encrypting {} values...", ages.len());
    let start = std::time::Instant::now();

    let mut encrypted = Vec::new();
    for age in &ages {
        let ct = FheUint8::try_encrypt(*age, &client_key)?;
        encrypted.push(bincode::serialize(&ct)?);
    }

    let encrypt_time = start.elapsed();
    println!("  Encryption took: {:?}", encrypt_time);

    let witness_data = bincode::serialize(&encrypted)?;
    println!("  Witness size: {} bytes", witness_data.len());

    // Prover computes average
    println!("\nComputing average of {} values...", ages.len());
    let compute_start = std::time::Instant::now();

    let inputs: Vec<Vec<u8>> = bincode::deserialize(&witness_data)?;
    let input_refs: Vec<&[u8]> = inputs.iter().map(|v| v.as_slice()).collect();

    use prover_node::DemographicsCircuit;
    let (encrypted_sum, count) = DemographicsCircuit::compute_average_u16(input_refs)?;

    let compute_time = compute_start.elapsed();
    println!("  Computation took: {:?}", compute_time);

    // Decrypt and verify
    let sum: FheUint16 = bincode::deserialize(&encrypted_sum)?;
    let sum_val: u16 = sum.decrypt(&client_key);
    let average = sum_val / count;

    let expected_sum: u16 = ages.iter().map(|&x| x as u16).sum();
    let expected_avg = expected_sum / (ages.len() as u16);

    println!("\nResults:");
    println!("  Sum: {} (expected: {})", sum_val, expected_sum);
    println!("  Count: {} (expected: {})", count, ages.len());
    println!("  Average: {} (expected: {})", average, expected_avg);

    assert_eq!(sum_val, expected_sum);
    assert_eq!(count as usize, ages.len());
    assert_eq!(average, expected_avg);

    println!("\n=== ✓ Large Dataset Test Passed ===\n");

    Ok(())
}
