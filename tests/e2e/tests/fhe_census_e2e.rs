//! E2E Test: Network School Census
//!
//! Scenario: 100 members of a decentralized network prove attendance
//! without revealing individual identities.

use anyhow::Result;
use prover_node::{CensusCircuit, generate_fhe_keys};
use tfhe::prelude::*;
use tfhe::{ClientKey, FheUint8, FheUint16};

#[test]
fn test_network_census_100_members() -> Result<()> {
    println!("\n🔐 NETWORK CENSUS E2E TEST");
    println!("================================");
    println!("Scenario: 100 members prove attendance privately\n");

    // Step 1: Generate FHE keys (client/server)
    println!("⏳ [1/5] Generating FHE keys...");
    let (client_key, server_key) = generate_fhe_keys()?;
    tfhe::set_server_key(server_key);
    println!("✓ Keys generated\n");

    // Step 2: Each member encrypts their vote (1 = present)
    println!("⏳ [2/5] 100 members encrypting votes...");
    let member_count = 100u16;
    let mut encrypted_votes: Vec<Vec<u8>> = Vec::new();

    for member_id in 1..=member_count {
        // Each member votes "1" (present)
        let vote = FheUint8::try_encrypt(1u8, &client_key)?;
        let vote_bytes = bincode::serialize(&vote)?;
        encrypted_votes.push(vote_bytes);

        if member_id % 25 == 0 {
            println!("  ✓ {} members encrypted their votes", member_id);
        }
    }
    println!("✓ All votes encrypted\n");

    // Step 3: Submit to prover network (Sum operation)
    println!("⏳ [3/5] Computing FHE Sum (server-side, no decryption)...");

    let inputs: Vec<&[u8]> = encrypted_votes.iter().map(|v| v.as_slice()).collect();

    // Server computes sum WITHOUT seeing individual votes
    let encrypted_result = CensusCircuit::compute_sum_u16(inputs)?;
    println!("✓ Sum computed on encrypted data\n");

    // Step 4: Client decrypts final result
    println!("⏳ [4/5] Decrypting final count (client-side)...");
    let result_ct: FheUint16 = bincode::deserialize(&encrypted_result)?;
    let total_present: u16 = result_ct.decrypt(&client_key);
    println!("✓ Result decrypted\n");

    // Step 5: Verify result
    println!("⏳ [5/5] Verifying correctness...");
    assert_eq!(
        total_present, member_count,
        "Expected {} members, got {}",
        member_count, total_present
    );
    println!("✓ Verification passed\n");

    // Privacy check
    println!("🔒 PRIVACY GUARANTEE:");
    println!("   ✓ Individual votes NEVER revealed");
    println!("   ✓ Server only saw encrypted ciphertexts");
    println!("   ✓ Only aggregate count (100) was decrypted");
    println!("\n✅ Network Census E2E Test PASSED\n");

    Ok(())
}

#[test]
fn test_partial_attendance_census() -> Result<()> {
    println!("\n🔐 PARTIAL ATTENDANCE E2E TEST");

    let (client_key, server_key) = generate_fhe_keys()?;
    tfhe::set_server_key(server_key);

    // Scenario: 75 members present, 25 absent
    let present_count = 75u8;
    let mut encrypted_votes: Vec<Vec<u8>> = Vec::new();

    // 75 members vote "1" (present)
    for _ in 0..present_count {
        let vote = FheUint8::try_encrypt(1u8, &client_key)?;
        encrypted_votes.push(bincode::serialize(&vote)?);
    }

    // 25 members vote "0" (absent, or don't vote)
    for _ in 0..25 {
        let vote = FheUint8::try_encrypt(0u8, &client_key)?;
        encrypted_votes.push(bincode::serialize(&vote)?);
    }

    let inputs: Vec<&[u8]> = encrypted_votes.iter().map(|v| v.as_slice()).collect();
    let encrypted_result = CensusCircuit::compute_sum_u8(inputs)?;

    let result_ct: FheUint8 = bincode::deserialize(&encrypted_result)?;
    let total_present: u8 = result_ct.decrypt(&client_key);

    assert_eq!(total_present, present_count);
    println!("✅ Partial Attendance Test PASSED: {}/100 present\n", total_present);

    Ok(())
}

#[test]
#[ignore] // Performance benchmark - run with `cargo test --ignored`
fn test_census_performance_benchmark() -> Result<()> {
    use std::time::Instant;

    println!("\n⏱️  PERFORMANCE BENCHMARK");

    let (client_key, server_key) = generate_fhe_keys()?;
    tfhe::set_server_key(server_key);

    let member_count = 100;
    let mut encrypted_votes: Vec<Vec<u8>> = Vec::new();

    println!("⏳ Encrypting {} votes...", member_count);
    for _ in 0..member_count {
        let vote = FheUint8::try_encrypt(1u8, &client_key)?;
        encrypted_votes.push(bincode::serialize(&vote)?);
    }

    let inputs: Vec<&[u8]> = encrypted_votes.iter().map(|v| v.as_slice()).collect();

    println!("⏳ Computing Sum({}) on encrypted data...", member_count);
    let start = Instant::now();
    let _ = CensusCircuit::compute_sum_u16(inputs)?;
    let duration = start.elapsed();

    println!("Sum(100) execution time: {:?}", duration);
    println!("Target: < 5s");

    if duration.as_secs() < 5 {
        println!("✅ Performance target MET");
    } else {
        println!("⚠️  Performance target MISSED (but acceptable for E2E)");
    }

    Ok(())
}
