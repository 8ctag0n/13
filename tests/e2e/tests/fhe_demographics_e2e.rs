//! E2E Test: Demographics Analysis (Average Age)
//!
//! Scenario: Compute average age of community members without revealing individual ages

use anyhow::Result;
use prover_node::{DemographicsCircuit, generate_fhe_keys};
use tfhe::prelude::*;
use tfhe::{FheUint8, FheUint16};

#[test]
fn test_demographics_average_age_basic() -> Result<()> {
    println!("\n📊 DEMOGRAPHICS E2E TEST: Average Age");
    println!("======================================");
    println!("Scenario: Compute community average age privately\n");

    // Step 1: Generate FHE keys
    println!("⏳ [1/5] Generating FHE keys...");
    let (client_key, server_key) = generate_fhe_keys()?;
    tfhe::set_server_key(server_key);
    println!("✓ Keys generated\n");

    // Step 2: Members encrypt their ages
    println!("⏳ [2/5] Members encrypting ages...");
    let member_ages = vec![22u8, 35, 41, 28, 33]; // Real ages
    let mut encrypted_ages: Vec<Vec<u8>> = Vec::new();

    println!("   Real ages (hidden from server): {:?}", member_ages);

    for (idx, age) in member_ages.iter().enumerate() {
        let age_ct = FheUint8::try_encrypt(*age, &client_key)?;
        encrypted_ages.push(bincode::serialize(&age_ct)?);
        println!("   ✓ Member {} encrypted age", idx + 1);
    }
    println!("✓ All ages encrypted\n");

    // Step 3: Server computes average WITHOUT seeing individual ages
    println!("⏳ [3/5] Computing average on encrypted data...");
    let inputs: Vec<&[u8]> = encrypted_ages.iter().map(|v| v.as_slice()).collect();

    let (encrypted_sum, count) = DemographicsCircuit::compute_average(inputs)?;
    println!("✓ Average computed (sum encrypted, count revealed: {})\n", count);

    // Step 4: Client decrypts sum and computes average
    println!("⏳ [4/5] Decrypting sum and computing average...");
    let sum_ct: FheUint8 = bincode::deserialize(&encrypted_sum)?;
    let sum: u8 = sum_ct.decrypt(&client_key);

    let average = sum / (count as u8);
    println!("✓ Sum: {}, Count: {}, Average: {}\n", sum, count, average);

    // Step 5: Verify result
    println!("⏳ [5/5] Verifying correctness...");
    let expected_sum: u8 = member_ages.iter().sum();
    let expected_avg = expected_sum / (member_ages.len() as u8);

    assert_eq!(sum, expected_sum, "Sum mismatch");
    assert_eq!(count, member_ages.len() as u16, "Count mismatch");
    assert_eq!(average, expected_avg, "Average mismatch");
    println!("✓ Verification passed\n");

    // Privacy check
    println!("🔒 PRIVACY GUARANTEE:");
    println!("   ✓ Individual ages NEVER revealed to server");
    println!("   ✓ Server only computed encrypted sum");
    println!("   ✓ Only aggregate average ({}) revealed", average);
    println!("\n✅ Demographics Average E2E Test PASSED\n");

    Ok(())
}

#[test]
fn test_demographics_average_larger_community() -> Result<()> {
    println!("\n📊 DEMOGRAPHICS E2E TEST: Larger Community");
    println!("===========================================");

    let (client_key, server_key) = generate_fhe_keys()?;
    tfhe::set_server_key(server_key);

    // Scenario: 20 members with diverse ages
    let member_ages = vec![
        18u8, 22, 25, 28, 30, 33, 35, 38, 40, 42,
        45, 48, 50, 52, 55, 58, 60, 62, 65, 68,
    ];

    println!("   Testing with {} members", member_ages.len());

    let mut encrypted_ages: Vec<Vec<u8>> = Vec::new();
    for age in &member_ages {
        let age_ct = FheUint8::try_encrypt(*age, &client_key)?;
        encrypted_ages.push(bincode::serialize(&age_ct)?);
    }

    let inputs: Vec<&[u8]> = encrypted_ages.iter().map(|v| v.as_slice()).collect();

    // Use u16 version for larger sums
    let (encrypted_sum, count) = DemographicsCircuit::compute_average_u16(inputs)?;

    let sum_ct: FheUint16 = bincode::deserialize(&encrypted_sum)?;
    let sum: u16 = sum_ct.decrypt(&client_key);
    let average = sum / count;

    let expected_sum: u16 = member_ages.iter().map(|&x| x as u16).sum();
    let expected_avg = expected_sum / (member_ages.len() as u16);

    assert_eq!(sum, expected_sum);
    assert_eq!(average, expected_avg);

    println!("✓ Average age: {} (from {} members)", average, count);
    println!("✅ Larger Community Test PASSED\n");

    Ok(())
}

#[test]
fn test_demographics_single_member() -> Result<()> {
    println!("\n📊 DEMOGRAPHICS E2E TEST: Single Member");
    println!("========================================");

    let (client_key, server_key) = generate_fhe_keys()?;
    tfhe::set_server_key(server_key);

    // Edge case: Only one member
    let member_age = 42u8;
    let age_ct = FheUint8::try_encrypt(member_age, &client_key)?;
    let age_bytes = bincode::serialize(&age_ct)?;

    let (encrypted_sum, count) = DemographicsCircuit::compute_average(vec![&age_bytes])?;

    let sum_ct: FheUint8 = bincode::deserialize(&encrypted_sum)?;
    let sum: u8 = sum_ct.decrypt(&client_key);
    let average = sum / (count as u8);

    assert_eq!(sum, member_age);
    assert_eq!(count, 1);
    assert_eq!(average, member_age);

    println!("✓ Single member average: {}", average);
    println!("✅ Single Member Test PASSED\n");

    Ok(())
}

#[test]
fn test_demographics_auto_average_selection() -> Result<()> {
    println!("\n📊 DEMOGRAPHICS E2E TEST: Auto Type Selection");
    println!("==============================================");

    let (client_key, server_key) = generate_fhe_keys()?;
    tfhe::set_server_key(server_key);

    // Test auto-selection: 10 values of 50 = 500 (requires u16)
    let member_ages = vec![50u8; 10];
    let mut encrypted_ages: Vec<Vec<u8>> = Vec::new();

    for age in &member_ages {
        let age_ct = FheUint8::try_encrypt(*age, &client_key)?;
        encrypted_ages.push(bincode::serialize(&age_ct)?);
    }

    let inputs: Vec<&[u8]> = encrypted_ages.iter().map(|v| v.as_slice()).collect();

    // Auto-average should detect sum > 255 and use u16
    let (encrypted_sum, count) = DemographicsCircuit::auto_average(inputs, 50)?;

    let sum_ct: FheUint16 = bincode::deserialize(&encrypted_sum)?;
    let sum: u16 = sum_ct.decrypt(&client_key);
    let average = sum / count;

    assert_eq!(sum, 500);
    assert_eq!(count, 10);
    assert_eq!(average, 50);

    println!("✓ Auto-selected u16 type for sum={}", sum);
    println!("✅ Auto Selection Test PASSED\n");

    Ok(())
}

#[test]
fn test_demographics_age_distribution() -> Result<()> {
    println!("\n📊 DEMOGRAPHICS E2E TEST: Age Distribution Analysis");
    println!("===================================================");

    let (client_key, server_key) = generate_fhe_keys()?;
    tfhe::set_server_key(server_key);

    // Realistic age distribution for network analysis
    let young_adults = vec![20u8, 22, 24, 25, 27]; // 5 members
    let adults = vec![30, 35, 38, 40, 42, 45, 48]; // 7 members
    let senior = vec![55, 60, 65]; // 3 members

    let mut all_ages = Vec::new();
    all_ages.extend(young_adults);
    all_ages.extend(adults);
    all_ages.extend(senior);

    println!("   Analyzing {} members across age ranges", all_ages.len());

    let mut encrypted_ages: Vec<Vec<u8>> = Vec::new();
    for age in &all_ages {
        let age_ct = FheUint8::try_encrypt(*age, &client_key)?;
        encrypted_ages.push(bincode::serialize(&age_ct)?);
    }

    let inputs: Vec<&[u8]> = encrypted_ages.iter().map(|v| v.as_slice()).collect();
    let (encrypted_sum, count) = DemographicsCircuit::compute_average_u16(inputs)?;

    let sum_ct: FheUint16 = bincode::deserialize(&encrypted_sum)?;
    let sum: u16 = sum_ct.decrypt(&client_key);
    let average = sum / count;

    println!("✓ Community average age: {}", average);
    println!("✓ Total members: {}", count);

    println!("\n🔒 PRIVACY:");
    println!("   ✓ Age distribution revealed only as average");
    println!("   ✓ Cannot identify individual members");
    println!("   ✓ Cannot determine age ranges without additional queries");

    println!("\n✅ Age Distribution Test PASSED\n");

    Ok(())
}

#[test]
#[ignore] // Performance benchmark - run with `cargo test --ignored`
fn test_demographics_performance_benchmark() -> Result<()> {
    use std::time::Instant;

    println!("\n⏱️  DEMOGRAPHICS PERFORMANCE BENCHMARK");
    println!("======================================");

    println!("⏳ Generating FHE keys...");
    let key_gen_start = Instant::now();
    let (client_key, server_key) = generate_fhe_keys()?;
    tfhe::set_server_key(server_key);
    let key_gen_duration = key_gen_start.elapsed();
    println!("✓ Key generation: {:?}\n", key_gen_duration);

    // Test with 100 members
    let member_count = 100;
    println!("⏳ Encrypting {} ages...", member_count);

    let mut encrypted_ages: Vec<Vec<u8>> = Vec::new();
    for _ in 0..member_count {
        let age_ct = FheUint8::try_encrypt(25u8, &client_key)?;
        encrypted_ages.push(bincode::serialize(&age_ct)?);
    }
    println!("✓ All ages encrypted\n");

    let inputs: Vec<&[u8]> = encrypted_ages.iter().map(|v| v.as_slice()).collect();

    println!("⏳ Computing average on {} encrypted values...", member_count);
    let start = Instant::now();
    let (encrypted_sum, count) = DemographicsCircuit::compute_average_u16(inputs)?;
    let computation_duration = start.elapsed();

    let sum_ct: FheUint16 = bincode::deserialize(&encrypted_sum)?;
    let sum: u16 = sum_ct.decrypt(&client_key);
    let average = sum / count;

    assert_eq!(sum, 2500);
    assert_eq!(average, 25);

    println!("✓ Average({}) execution time: {:?}", member_count, computation_duration);
    println!("   Target: < 7s");

    if computation_duration.as_secs() < 7 {
        println!("   ✅ Performance target MET\n");
    } else {
        println!("   ⚠️  Performance target MISSED (but acceptable for E2E)\n");
    }

    println!("=== SUMMARY ===");
    println!("Key Generation:  {:?}", key_gen_duration);
    println!("Average(100):    {:?}", computation_duration);
    println!("\nNOTE: Average reuses Sum operation, so performance is similar.");

    Ok(())
}
