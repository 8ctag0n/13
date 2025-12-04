//! E2E Test: zk-Passport Age Verification
//!
//! Scenario: Prove you're an adult (>= 18) without revealing exact age

use anyhow::Result;
use prover_node::{PassportCircuit, generate_fhe_keys};
use tfhe::prelude::*;
use tfhe::{ClientKey, FheUint8, FheBool};

#[test]
fn test_zkpassport_age_verification_adult() -> Result<()> {
    println!("\n🛂 ZK-PASSPORT E2E TEST: Adult Verification");
    println!("============================================");

    // Step 1: Setup
    println!("⏳ [1/4] Generating FHE keys...");
    let (client_key, server_key) = generate_fhe_keys()?;
    tfhe::set_server_key(server_key);
    println!("✓ Keys generated\n");

    // Step 2: User encrypts real age (25)
    println!("⏳ [2/4] User encrypting age...");
    let real_age = 25u8;
    println!("   Real age: {} (encrypted, hidden from server)", real_age);

    let age_ct = FheUint8::try_encrypt(real_age, &client_key)?;
    let age_bytes = bincode::serialize(&age_ct)?;
    println!("✓ Age encrypted\n");

    // Step 3: Server verifies age >= 18 WITHOUT seeing real age
    println!("⏳ [3/4] Server verifying age >= 18 (zero-knowledge)...");
    let threshold = 18u8;

    let result_bytes = PassportCircuit::compute_threshold(
        &age_bytes,
        threshold,
        true, // greater_or_equal
    )?;
    println!("✓ Verification computed on encrypted data\n");

    // Step 4: Client decrypts result
    println!("⏳ [4/4] Decrypting verification result...");
    let result_ct: FheBool = bincode::deserialize(&result_bytes)?;
    let is_adult: bool = result_ct.decrypt(&client_key);
    println!("✓ Result: {}\n", if is_adult { "ADULT ✓" } else { "MINOR ✗" });

    // Verify
    assert!(is_adult, "Expected age 25 >= 18");

    // Privacy check
    println!("🔒 PRIVACY GUARANTEE:");
    println!("   ✓ Real age (25) NEVER revealed to server");
    println!("   ✓ Server only computed threshold check");
    println!("   ✓ Result: Boolean (adult/minor) only");
    println!("\n✅ zk-Passport Adult Verification PASSED\n");

    Ok(())
}

#[test]
fn test_zkpassport_age_verification_minor() -> Result<()> {
    println!("\n🛂 ZK-PASSPORT E2E TEST: Minor Verification");
    println!("============================================");

    let (client_key, server_key) = generate_fhe_keys()?;
    tfhe::set_server_key(server_key);

    // User age: 15 (minor)
    let real_age = 15u8;
    println!("   Real age: {} (encrypted, hidden from server)", real_age);

    let age_ct = FheUint8::try_encrypt(real_age, &client_key)?;
    let age_bytes = bincode::serialize(&age_ct)?;

    let result_bytes = PassportCircuit::compute_threshold(&age_bytes, 18, true)?;
    let result_ct: FheBool = bincode::deserialize(&result_bytes)?;
    let is_adult: bool = result_ct.decrypt(&client_key);

    assert!(!is_adult, "Expected age 15 < 18");
    println!("✓ Result: MINOR ✗ (correctly identified)\n");
    println!("✅ Minor correctly identified (age < 18)\n");

    Ok(())
}

#[test]
fn test_zkpassport_working_age_range() -> Result<()> {
    println!("\n🛂 ZK-PASSPORT E2E TEST: Working Age Range");
    println!("===========================================");

    let (client_key, server_key) = generate_fhe_keys()?;
    tfhe::set_server_key(server_key);

    // Scenario: Job requires age 18-65
    let user_age = 45u8;
    println!("   User age: {} (encrypted)", user_age);

    let age_ct = FheUint8::try_encrypt(user_age, &client_key)?;
    let age_bytes = bincode::serialize(&age_ct)?;

    println!("⏳ Checking if age in range [18, 65]...");
    let result_bytes = PassportCircuit::compute_range_check(&age_bytes, 18, 65)?;

    let result_ct: FheBool = bincode::deserialize(&result_bytes)?;
    let in_range: bool = result_ct.decrypt(&client_key);

    assert!(in_range, "Expected age 45 in [18, 65]");
    println!("✓ Age in valid working range\n");

    println!("🔒 PRIVACY:");
    println!("   ✓ Exact age (45) hidden from employer");
    println!("   ✓ Only eligibility status revealed");
    println!("\n✅ Working Age Range Test PASSED\n");

    Ok(())
}

#[test]
fn test_zkpassport_retirement_age() -> Result<()> {
    println!("\n🛂 ZK-PASSPORT E2E TEST: Retirement Age");
    println!("========================================");

    let (client_key, server_key) = generate_fhe_keys()?;
    tfhe::set_server_key(server_key);

    // User age: 70 (retired, above working range)
    let user_age = 70u8;
    println!("   User age: {} (encrypted)", user_age);

    let age_ct = FheUint8::try_encrypt(user_age, &client_key)?;
    let age_bytes = bincode::serialize(&age_ct)?;

    let result_bytes = PassportCircuit::compute_range_check(&age_bytes, 18, 65)?;
    let result_ct: FheBool = bincode::deserialize(&result_bytes)?;
    let in_range: bool = result_ct.decrypt(&client_key);

    assert!(!in_range, "Expected age 70 > 65 (out of range)");
    println!("✓ Result: OUT OF RANGE (correctly identified)\n");
    println!("✅ Retirement age correctly identified\n");

    Ok(())
}

#[test]
#[ignore] // Performance benchmark - run manually
fn test_passport_performance_benchmark() -> Result<()> {
    use std::time::Instant;

    println!("\n⏱️  PASSPORT PERFORMANCE BENCHMARK");
    println!("====================================");

    println!("⏳ Generating FHE keys (this is slow, ~30-40s)...");
    let key_gen_start = Instant::now();
    let (client_key, server_key) = generate_fhe_keys()?;
    tfhe::set_server_key(server_key);
    let key_gen_duration = key_gen_start.elapsed();
    println!("✓ Key generation took: {:?}\n", key_gen_duration);

    println!("⏳ Encrypting test data...");
    let age_ct = FheUint8::try_encrypt(25u8, &client_key)?;
    let age_bytes = bincode::serialize(&age_ct)?;
    println!("✓ Data encrypted\n");

    // Benchmark Threshold
    println!("⏳ [1/2] Benchmarking Threshold operation...");
    let start = Instant::now();
    let _ = PassportCircuit::compute_threshold(&age_bytes, 18, true)?;
    let threshold_duration = start.elapsed();
    println!("✓ Threshold execution time: {:?}", threshold_duration);
    println!("   Target: < 120s (excluding keygen)");

    if threshold_duration.as_secs() < 120 {
        println!("   ✅ Threshold performance target MET\n");
    } else {
        println!("   ⚠️  Threshold performance target MISSED\n");
    }

    // Benchmark RangeCheck
    println!("⏳ [2/2] Benchmarking RangeCheck operation...");
    let start = Instant::now();
    let _ = PassportCircuit::compute_range_check(&age_bytes, 18, 65)?;
    let range_duration = start.elapsed();
    println!("✓ RangeCheck execution time: {:?}", range_duration);
    println!("   Target: < 180s (excluding keygen)");

    if range_duration.as_secs() < 180 {
        println!("   ✅ RangeCheck performance target MET\n");
    } else {
        println!("   ⚠️  RangeCheck performance target MISSED\n");
    }

    println!("=== SUMMARY ===");
    println!("Key Generation:      {:?}", key_gen_duration);
    println!("Threshold Operation: {:?}", threshold_duration);
    println!("RangeCheck Operation: {:?}", range_duration);
    println!("\nNOTE: FHE operations are computationally expensive.");
    println!("Production optimizations: hardware acceleration, parameter tuning, caching");

    Ok(())
}
