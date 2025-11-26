//! E2E Test: Private DAO Voting (CountIf)
//!
//! Scenario: DAO members vote privately, votes are tallied without revealing individual choices

use anyhow::Result;
use prover_node::{VotingCircuit, generate_fhe_keys};
use zyberlink_types::fhe::FhePredicate;
use tfhe::prelude::*;
use tfhe::FheUint8;

#[test]
fn test_dao_voting_simple_count() -> Result<()> {
    println!("\n🗳️  DAO VOTING E2E TEST: Simple Vote Count");
    println!("==========================================");
    println!("Scenario: Count votes for specific option\n");

    // Step 1: Generate FHE keys
    println!("⏳ [1/5] Generating FHE keys...");
    let (client_key, server_key) = generate_fhe_keys()?;
    tfhe::set_server_key(server_key);
    println!("✓ Keys generated\n");

    // Step 2: Members cast votes (encrypted)
    println!("⏳ [2/5] DAO members casting encrypted votes...");
    // Votes: [1, 3, 3, 2, 3, 1] -> count(option 3) = 3
    let votes = vec![1u8, 3, 3, 2, 3, 1];
    let mut encrypted_votes: Vec<Vec<u8>> = Vec::new();

    println!("   Total voters: {}", votes.len());
    println!("   Votes (hidden from server): {:?}", votes);

    for (idx, vote) in votes.iter().enumerate() {
        let vote_ct = FheUint8::try_encrypt(*vote, &client_key)?;
        encrypted_votes.push(bincode::serialize(&vote_ct)?);
        println!("   ✓ Voter {} cast encrypted vote", idx + 1);
    }
    println!("✓ All votes encrypted\n");

    // Step 3: Server counts votes for option 3 WITHOUT seeing individual votes
    println!("⏳ [3/5] Counting votes for Option 3 (encrypted)...");
    let inputs: Vec<&[u8]> = encrypted_votes.iter().map(|v| v.as_slice()).collect();

    let predicate = FhePredicate::Equals(3);
    let encrypted_count = VotingCircuit::compute_count_if(inputs, &predicate)?;
    println!("✓ Count computed on encrypted data\n");

    // Step 4: Decrypt final tally
    println!("⏳ [4/5] Decrypting final tally...");
    let count_ct: FheUint8 = bincode::deserialize(&encrypted_count)?;
    let count: u8 = count_ct.decrypt(&client_key);
    println!("✓ Option 3 received {} votes\n", count);

    // Step 5: Verify result
    println!("⏳ [5/5] Verifying correctness...");
    let expected_count = votes.iter().filter(|&&v| v == 3).count() as u8;
    assert_eq!(count, expected_count, "Count mismatch");
    println!("✓ Verification passed\n");

    // Privacy check
    println!("🔒 PRIVACY GUARANTEE:");
    println!("   ✓ Individual votes NEVER revealed");
    println!("   ✓ Server only saw encrypted ciphertexts");
    println!("   ✓ Only aggregate tally (3) decrypted");
    println!("\n✅ DAO Voting E2E Test PASSED\n");

    Ok(())
}

#[test]
fn test_dao_voting_full_tally() -> Result<()> {
    println!("\n🗳️  DAO VOTING E2E TEST: Full Tally");
    println!("====================================");
    println!("Scenario: Tally all options in DAO vote\n");

    let (client_key, server_key) = generate_fhe_keys()?;
    tfhe::set_server_key(server_key);

    // 50 members vote on 3 options (0, 1, 2)
    let mut votes = Vec::new();
    votes.extend(vec![0u8; 20]); // 20 votes for option 0
    votes.extend(vec![1u8; 18]); // 18 votes for option 1
    votes.extend(vec![2u8; 12]); // 12 votes for option 2

    println!("   Total voters: {}", votes.len());
    println!("   Expected tallies: [20, 18, 12]\n");

    let mut encrypted_votes: Vec<Vec<u8>> = Vec::new();
    for vote in &votes {
        let vote_ct = FheUint8::try_encrypt(*vote, &client_key)?;
        encrypted_votes.push(bincode::serialize(&vote_ct)?);
    }

    let inputs: Vec<&[u8]> = encrypted_votes.iter().map(|v| v.as_slice()).collect();

    // Count each option
    println!("⏳ Tallying votes for all options...");
    let mut tallies = Vec::new();

    for option in 0u8..3 {
        println!("   Counting Option {}...", option);
        let predicate = FhePredicate::Equals(option);
        let encrypted_count = VotingCircuit::compute_count_if(inputs.clone(), &predicate)?;

        let count_ct: FheUint8 = bincode::deserialize(&encrypted_count)?;
        let count: u8 = count_ct.decrypt(&client_key);
        tallies.push(count);
        println!("   ✓ Option {}: {} votes", option, count);
    }

    println!("\n✓ Final tallies: {:?}", tallies);

    // Verify
    assert_eq!(tallies, vec![20, 18, 12]);
    assert_eq!(tallies.iter().sum::<u8>(), 50);

    println!("\n🏆 RESULT: Option 0 wins with 20/50 votes (40%)");
    println!("\n✅ Full Tally Test PASSED\n");

    Ok(())
}

#[test]
fn test_dao_voting_age_eligibility() -> Result<()> {
    println!("\n🗳️  DAO VOTING E2E TEST: Age Eligibility Count");
    println!("==============================================");
    println!("Scenario: Count eligible voters (age > 18)\n");

    let (client_key, server_key) = generate_fhe_keys()?;
    tfhe::set_server_key(server_key);

    // Member ages
    let ages = vec![15u8, 20, 18, 25, 30, 17, 22];
    println!("   Ages (hidden): {:?}", ages);
    println!("   Expected eligible (age > 18): 4\n");

    let mut encrypted_ages: Vec<Vec<u8>> = Vec::new();
    for age in &ages {
        let age_ct = FheUint8::try_encrypt(*age, &client_key)?;
        encrypted_ages.push(bincode::serialize(&age_ct)?);
    }

    let inputs: Vec<&[u8]> = encrypted_ages.iter().map(|v| v.as_slice()).collect();

    println!("⏳ Counting eligible voters (age > 18)...");
    let predicate = FhePredicate::GreaterThan(18);
    let encrypted_count = VotingCircuit::compute_count_if(inputs, &predicate)?;

    let count_ct: FheUint8 = bincode::deserialize(&encrypted_count)?;
    let eligible_count: u8 = count_ct.decrypt(&client_key);

    println!("✓ Eligible voters: {}/{}\n", eligible_count, ages.len());

    // Verify: ages > 18 are: 20, 25, 30, 22 = 4
    assert_eq!(eligible_count, 4);

    println!("🔒 PRIVACY:");
    println!("   ✓ Individual ages remain private");
    println!("   ✓ Only eligibility count revealed");
    println!("\n✅ Age Eligibility Test PASSED\n");

    Ok(())
}

#[test]
fn test_dao_voting_range_based() -> Result<()> {
    println!("\n🗳️  DAO VOTING E2E TEST: Range-Based Counting");
    println!("=============================================");
    println!("Scenario: Count members in specific age range\n");

    let (client_key, server_key) = generate_fhe_keys()?;
    tfhe::set_server_key(server_key);

    // Ages: count working age population (18-65)
    let ages = vec![15u8, 25, 35, 45, 70, 20, 68, 30];
    println!("   Ages: {:?}", ages);
    println!("   Counting working age (18-65)...\n");

    let mut encrypted_ages: Vec<Vec<u8>> = Vec::new();
    for age in &ages {
        let age_ct = FheUint8::try_encrypt(*age, &client_key)?;
        encrypted_ages.push(bincode::serialize(&age_ct)?);
    }

    let inputs: Vec<&[u8]> = encrypted_ages.iter().map(|v| v.as_slice()).collect();

    let predicate = FhePredicate::InRange { min: 18, max: 65 };
    let encrypted_count = VotingCircuit::compute_count_if(inputs, &predicate)?;

    let count_ct: FheUint8 = bincode::deserialize(&encrypted_count)?;
    let working_age_count: u8 = count_ct.decrypt(&client_key);

    println!("✓ Working age population: {}/{}\n", working_age_count, ages.len());

    // Verify: 25, 35, 45, 20, 30 = 5 (15, 70, 68 excluded)
    assert_eq!(working_age_count, 5);

    println!("✅ Range-Based Counting Test PASSED\n");

    Ok(())
}

#[test]
fn test_dao_voting_unanimous_decision() -> Result<()> {
    println!("\n🗳️  DAO VOTING E2E TEST: Unanimous Decision");
    println!("===========================================");

    let (client_key, server_key) = generate_fhe_keys()?;
    tfhe::set_server_key(server_key);

    // All members vote for option 1
    let votes = vec![1u8; 100];
    println!("   Testing unanimous vote: 100/100 for Option 1\n");

    let mut encrypted_votes: Vec<Vec<u8>> = Vec::new();
    for vote in &votes {
        let vote_ct = FheUint8::try_encrypt(*vote, &client_key)?;
        encrypted_votes.push(bincode::serialize(&vote_ct)?);
    }

    let inputs: Vec<&[u8]> = encrypted_votes.iter().map(|v| v.as_slice()).collect();

    let predicate = FhePredicate::Equals(1);
    let encrypted_count = VotingCircuit::compute_count_if(inputs, &predicate)?;

    let count_ct: FheUint8 = bincode::deserialize(&encrypted_count)?;
    let count: u8 = count_ct.decrypt(&client_key);

    assert_eq!(count, 100);
    println!("✓ Unanimous: 100/100 votes for Option 1");
    println!("✅ Unanimous Decision Test PASSED\n");

    Ok(())
}

#[test]
fn test_dao_voting_no_votes() -> Result<()> {
    println!("\n🗳️  DAO VOTING E2E TEST: Zero Votes");
    println!("===================================");

    let (client_key, server_key) = generate_fhe_keys()?;
    tfhe::set_server_key(server_key);

    // Members vote, but none vote for option 5
    let votes = vec![0u8, 1, 2, 3, 4];
    println!("   Votes: {:?}", votes);
    println!("   Counting Option 5 (none voted)...\n");

    let mut encrypted_votes: Vec<Vec<u8>> = Vec::new();
    for vote in &votes {
        let vote_ct = FheUint8::try_encrypt(*vote, &client_key)?;
        encrypted_votes.push(bincode::serialize(&vote_ct)?);
    }

    let inputs: Vec<&[u8]> = encrypted_votes.iter().map(|v| v.as_slice()).collect();

    let predicate = FhePredicate::Equals(5);
    let encrypted_count = VotingCircuit::compute_count_if(inputs, &predicate)?;

    let count_ct: FheUint8 = bincode::deserialize(&encrypted_count)?;
    let count: u8 = count_ct.decrypt(&client_key);

    assert_eq!(count, 0);
    println!("✓ Option 5: 0 votes (correctly counted)");
    println!("✅ Zero Votes Test PASSED\n");

    Ok(())
}

#[test]
fn test_dao_voting_not_equals_predicate() -> Result<()> {
    println!("\n🗳️  DAO VOTING E2E TEST: NotEquals Predicate");
    println!("============================================");

    let (client_key, server_key) = generate_fhe_keys()?;
    tfhe::set_server_key(server_key);

    // Count members who did NOT vote for abstain (option 0)
    let votes = vec![1u8, 0, 2, 0, 3, 1, 0];
    println!("   Votes: {:?}", votes);
    println!("   Counting non-abstain votes (vote != 0)...\n");

    let mut encrypted_votes: Vec<Vec<u8>> = Vec::new();
    for vote in &votes {
        let vote_ct = FheUint8::try_encrypt(*vote, &client_key)?;
        encrypted_votes.push(bincode::serialize(&vote_ct)?);
    }

    let inputs: Vec<&[u8]> = encrypted_votes.iter().map(|v| v.as_slice()).collect();

    let predicate = FhePredicate::NotEquals(0);
    let encrypted_count = VotingCircuit::compute_count_if(inputs, &predicate)?;

    let count_ct: FheUint8 = bincode::deserialize(&encrypted_count)?;
    let non_abstain: u8 = count_ct.decrypt(&client_key);

    // Non-abstain: 1, 2, 3, 1 = 4
    assert_eq!(non_abstain, 4);
    println!("✓ Non-abstain votes: 4/7");
    println!("✅ NotEquals Predicate Test PASSED\n");

    Ok(())
}

#[test]
#[ignore] // Performance benchmark - run with `cargo test --ignored`
fn test_dao_voting_performance_benchmark() -> Result<()> {
    use std::time::Instant;

    println!("\n⏱️  DAO VOTING PERFORMANCE BENCHMARK");
    println!("====================================");

    println!("⏳ Generating FHE keys...");
    let key_gen_start = Instant::now();
    let (client_key, server_key) = generate_fhe_keys()?;
    tfhe::set_server_key(server_key);
    let key_gen_duration = key_gen_start.elapsed();
    println!("✓ Key generation: {:?}\n", key_gen_duration);

    // Test with 100 votes
    let vote_count = 100;
    println!("⏳ Encrypting {} votes...", vote_count);

    let mut encrypted_votes: Vec<Vec<u8>> = Vec::new();
    for i in 0..vote_count {
        // 60 votes for option 1, 40 for option 0
        let vote = if i < 60 { 1u8 } else { 0u8 };
        let vote_ct = FheUint8::try_encrypt(vote, &client_key)?;
        encrypted_votes.push(bincode::serialize(&vote_ct)?);
    }
    println!("✓ All votes encrypted\n");

    let inputs: Vec<&[u8]> = encrypted_votes.iter().map(|v| v.as_slice()).collect();

    println!("⏳ Counting votes for Option 1 (encrypted)...");
    let predicate = FhePredicate::Equals(1);

    let start = Instant::now();
    let encrypted_count = VotingCircuit::compute_count_if(inputs, &predicate)?;
    let computation_duration = start.elapsed();

    let count_ct: FheUint8 = bincode::deserialize(&encrypted_count)?;
    let count: u8 = count_ct.decrypt(&client_key);

    assert_eq!(count, 60);

    println!("✓ CountIf({}) execution time: {:?}", vote_count, computation_duration);
    println!("   Target: < 10s");

    if computation_duration.as_secs() < 10 {
        println!("   ✅ Performance target MET\n");
    } else {
        println!("   ⚠️  Performance target MISSED (but acceptable for E2E)\n");
    }

    println!("=== SUMMARY ===");
    println!("Key Generation:  {:?}", key_gen_duration);
    println!("CountIf(100):    {:?}", computation_duration);
    println!("\nNOTE: CountIf involves {} FHE comparisons + additions.", vote_count);
    println!("Production optimizations: batching, caching, hardware acceleration");

    Ok(())
}
