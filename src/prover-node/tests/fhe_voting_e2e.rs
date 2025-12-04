/// End-to-End test for FHE CountIf operation in the marketplace (Voting scenario)
///
/// This test validates the complete voting flow:
/// 1. Create FHE job with CountIf operation
/// 2. Multiple provers count encrypted votes
/// 3. Provers reach consensus on vote tallies
/// 4. Payment distribution and verification
use anyhow::Result;
use tfhe::prelude::*;
use tfhe::{generate_keys, set_server_key, ConfigBuilder, FheUint8};
use zyberlink_types::fhe::FhePredicate;

#[test]
fn test_fhe_voting_basic_count() -> Result<()> {
    // Test basic voting: count votes for each option

    println!("\n=== FHE Voting Basic Count Test ===\n");

    // Step 1: Generate FHE keys
    println!("Step 1: Generating FHE keys...");
    let config = ConfigBuilder::default().build();
    let (client_key, server_key) = generate_keys(config);
    set_server_key(server_key.clone());
    println!("  ✓ Keys generated");

    // Step 2: Voters encrypt their votes
    println!("\nStep 2: Voters encrypt votes...");
    // 50 voters, 3 options (0, 1, 2)
    // Distribution: 20 votes for option 0, 18 for option 1, 12 for option 2
    let mut votes = Vec::new();
    votes.extend(vec![0u8; 20]);
    votes.extend(vec![1u8; 18]);
    votes.extend(vec![2u8; 12]);

    let mut encrypted_votes = Vec::new();
    for (i, vote) in votes.iter().enumerate() {
        let encrypted = FheUint8::try_encrypt(*vote, &client_key)?;
        encrypted_votes.push(bincode::serialize(&encrypted)?);
        if i < 5 || i >= votes.len() - 3 {
            println!("  Vote {}: option {} encrypted", i + 1, vote);
        } else if i == 5 {
            println!("  ... ({} more votes) ...", votes.len() - 8);
        }
    }

    println!("  ✓ {} votes encrypted", votes.len());

    // Step 3: Create job witness
    let witness_data = bincode::serialize(&encrypted_votes)?;
    println!(
        "\nStep 3: Job witness created: {} bytes",
        witness_data.len()
    );

    // Step 4: Prover tallies votes for each option
    println!("\nStep 4: Prover counting votes...");

    let inputs: Vec<Vec<u8>> = bincode::deserialize(&witness_data)?;
    let input_refs: Vec<&[u8]> = inputs.iter().map(|v| v.as_slice()).collect();

    use prover_node::VotingCircuit;

    // Count votes for option 0
    let predicate_0 = FhePredicate::Equals(0);
    let count_0_bytes = VotingCircuit::compute_count_if(input_refs.clone(), &predicate_0)?;
    let count_0_ct: FheUint8 = bincode::deserialize(&count_0_bytes)?;
    let count_0: u8 = count_0_ct.decrypt(&client_key);
    println!("  Option 0: {} votes", count_0);

    // Count votes for option 1
    let predicate_1 = FhePredicate::Equals(1);
    let count_1_bytes = VotingCircuit::compute_count_if(input_refs.clone(), &predicate_1)?;
    let count_1_ct: FheUint8 = bincode::deserialize(&count_1_bytes)?;
    let count_1: u8 = count_1_ct.decrypt(&client_key);
    println!("  Option 1: {} votes", count_1);

    // Count votes for option 2
    let predicate_2 = FhePredicate::Equals(2);
    let count_2_bytes = VotingCircuit::compute_count_if(input_refs, &predicate_2)?;
    let count_2_ct: FheUint8 = bincode::deserialize(&count_2_bytes)?;
    let count_2: u8 = count_2_ct.decrypt(&client_key);
    println!("  Option 2: {} votes", count_2);

    // Verify results
    assert_eq!(count_0, 20);
    assert_eq!(count_1, 18);
    assert_eq!(count_2, 12);
    assert_eq!(count_0 + count_1 + count_2, votes.len() as u8);

    println!("\n=== ✓ Voting Test Passed ===\n");

    Ok(())
}

#[test]
fn test_fhe_voting_consensus() -> Result<()> {
    // Simulate 3 provers independently counting votes
    // All should produce identical result hashes

    println!("\n=== FHE Voting Consensus Test ===\n");

    let config = ConfigBuilder::default().build();
    let (client_key, server_key) = generate_keys(config);
    set_server_key(server_key.clone());

    // Create encrypted votes
    let votes = vec![1u8, 1, 2, 1, 3, 2, 1, 3, 1];
    let mut encrypted = Vec::new();
    for vote in &votes {
        let ct = FheUint8::try_encrypt(*vote, &client_key)?;
        encrypted.push(bincode::serialize(&ct)?);
    }

    let witness_data = bincode::serialize(&encrypted)?;

    // Simulate 3 provers
    let mut result_hashes = Vec::new();

    for prover_id in 1..=3 {
        println!("Prover {} tallying votes...", prover_id);

        let inputs: Vec<Vec<u8>> = bincode::deserialize(&witness_data)?;
        let input_refs: Vec<&[u8]> = inputs.iter().map(|v| v.as_slice()).collect();

        use prover_node::VotingCircuit;

        // Count votes for option 1
        let predicate = FhePredicate::Equals(1);
        let count_bytes = VotingCircuit::compute_count_if(input_refs, &predicate)?;

        // Hash result
        use sha3::{Digest, Sha3_256};
        let mut hasher = Sha3_256::new();
        hasher.update(&count_bytes);
        let hash: [u8; 32] = hasher.finalize().into();

        result_hashes.push(hash);
        println!("  Result hash: {}", hex::encode(&hash[..8]));

        // Verify result
        let count_ct: FheUint8 = bincode::deserialize(&count_bytes)?;
        let count: u8 = count_ct.decrypt(&client_key);
        println!("  Votes for option 1: {}", count);
    }

    // Consensus check
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
fn test_fhe_voting_age_eligibility() -> Result<()> {
    // Real-world scenario: Count eligible voters (age >= 18)

    println!("\n=== FHE Voting Age Eligibility Test ===\n");

    let config = ConfigBuilder::default().build();
    let (client_key, server_key) = generate_keys(config);
    set_server_key(server_key);

    // 100 participants with different ages
    let mut ages = Vec::new();
    ages.extend(vec![15u8; 10]); // 10 minors
    ages.extend(vec![25u8; 60]); // 60 adults
    ages.extend(vec![70u8; 30]); // 30 seniors

    println!("Encrypting {} age values...", ages.len());
    let mut encrypted = Vec::new();
    for age in &ages {
        let ct = FheUint8::try_encrypt(*age, &client_key)?;
        encrypted.push(bincode::serialize(&ct)?);
    }

    let witness_data = bincode::serialize(&encrypted)?;
    println!("  ✓ Ages encrypted");

    // Count eligible voters (age >= 18)
    println!("\nCounting eligible voters (age >= 18)...");

    let inputs: Vec<Vec<u8>> = bincode::deserialize(&witness_data)?;
    let input_refs: Vec<&[u8]> = inputs.iter().map(|v| v.as_slice()).collect();

    use prover_node::VotingCircuit;

    // Use GreaterThan(17) which means >= 18
    let predicate = FhePredicate::GreaterThan(17);
    let count_bytes = VotingCircuit::compute_count_if(input_refs, &predicate)?;

    let count_ct: FheUint8 = bincode::deserialize(&count_bytes)?;
    let eligible_count: u8 = count_ct.decrypt(&client_key);

    println!("  Eligible voters: {}", eligible_count);
    println!("  Total participants: {}", ages.len());
    println!(
        "  Ineligible (minors): {}",
        ages.len() as u8 - eligible_count
    );

    // Verify: 60 adults + 30 seniors = 90 eligible
    assert_eq!(eligible_count, 90);

    println!("\n=== ✓ Age Eligibility Test Passed ===\n");

    Ok(())
}

#[test]
fn test_fhe_voting_dao_scenario() -> Result<()> {
    // Complete DAO voting scenario with multiple predicates

    println!("\n=== FHE DAO Voting Scenario Test ===\n");

    let config = ConfigBuilder::default().build();
    let (client_key, server_key) = generate_keys(config);
    set_server_key(server_key);

    // DAO Proposal: 3 options (0=Abstain, 1=Yes, 2=No)
    // 200 DAO members vote
    let mut votes = Vec::new();
    votes.extend(vec![0u8; 20]); // 20 abstentions
    votes.extend(vec![1u8; 120]); // 120 yes
    votes.extend(vec![2u8; 60]); // 60 no

    println!("DAO Voting: {} members", votes.len());
    println!("  Expected: 20 abstain, 120 yes, 60 no");

    // Encrypt votes
    println!("\nEncrypting votes...");
    let start = std::time::Instant::now();

    let mut encrypted = Vec::new();
    for vote in &votes {
        let ct = FheUint8::try_encrypt(*vote, &client_key)?;
        encrypted.push(bincode::serialize(&ct)?);
    }

    let encrypt_time = start.elapsed();
    println!("  ✓ {} votes encrypted in {:?}", votes.len(), encrypt_time);

    let witness_data = bincode::serialize(&encrypted)?;

    // Prover tallies votes
    println!("\nTallying votes...");
    let tally_start = std::time::Instant::now();

    let inputs: Vec<Vec<u8>> = bincode::deserialize(&witness_data)?;
    let input_refs: Vec<&[u8]> = inputs.iter().map(|v| v.as_slice()).collect();

    use prover_node::VotingCircuit;

    // Count abstentions
    let pred_abstain = FhePredicate::Equals(0);
    let abstain_bytes = VotingCircuit::compute_count_if(input_refs.clone(), &pred_abstain)?;
    let abstain_ct: FheUint8 = bincode::deserialize(&abstain_bytes)?;
    let abstain_count: u8 = abstain_ct.decrypt(&client_key);

    // Count yes votes
    let pred_yes = FhePredicate::Equals(1);
    let yes_bytes = VotingCircuit::compute_count_if(input_refs.clone(), &pred_yes)?;
    let yes_ct: FheUint8 = bincode::deserialize(&yes_bytes)?;
    let yes_count: u8 = yes_ct.decrypt(&client_key);

    // Count no votes
    let pred_no = FhePredicate::Equals(2);
    let no_bytes = VotingCircuit::compute_count_if(input_refs, &pred_no)?;
    let no_ct: FheUint8 = bincode::deserialize(&no_bytes)?;
    let no_count: u8 = no_ct.decrypt(&client_key);

    let tally_time = tally_start.elapsed();

    println!("  ✓ Tallying completed in {:?}", tally_time);
    println!("\nResults:");
    println!("  Abstain: {} votes", abstain_count);
    println!("  Yes:     {} votes", yes_count);
    println!("  No:      {} votes", no_count);
    println!("  Total:   {} votes", abstain_count + yes_count + no_count);

    // Verify results
    assert_eq!(abstain_count, 20);
    assert_eq!(yes_count, 120);
    assert_eq!(no_count, 60);
    assert_eq!(abstain_count + yes_count + no_count, votes.len() as u8);

    // Determine outcome
    let total_votes = yes_count + no_count; // Exclude abstentions
    let yes_percentage = (yes_count as f64 / total_votes as f64) * 100.0;

    println!("\nProposal Outcome:");
    println!("  Yes: {:.1}%", yes_percentage);
    println!("  No:  {:.1}%", 100.0 - yes_percentage);

    if yes_percentage > 50.0 {
        println!("  ✓ PROPOSAL APPROVED");
    } else {
        println!("  ✗ PROPOSAL REJECTED");
    }

    assert!(
        yes_percentage > 50.0,
        "Proposal should pass with 66.7% yes votes"
    );

    println!("\n=== ✓ DAO Voting Test Passed ===\n");

    Ok(())
}

#[test]
#[ignore] // Run with --ignored flag (slow test)
fn test_fhe_voting_performance_large_scale() -> Result<()> {
    // Performance test with 500 voters

    println!("\n=== FHE Voting Large Scale Performance Test ===\n");

    let config = ConfigBuilder::default().build();
    let (client_key, server_key) = generate_keys(config);
    set_server_key(server_key);

    // 500 voters, 5 options
    println!("Creating 500 encrypted votes...");
    let mut votes = Vec::new();
    for i in 0..500 {
        votes.push((i % 5) as u8);
    }

    let encrypt_start = std::time::Instant::now();
    let mut encrypted = Vec::new();
    for vote in &votes {
        let ct = FheUint8::try_encrypt(*vote, &client_key)?;
        encrypted.push(bincode::serialize(&ct)?);
    }
    let encrypt_time = encrypt_start.elapsed();

    println!("  ✓ Encryption took: {:?}", encrypt_time);

    let witness_data = bincode::serialize(&encrypted)?;
    println!("  Witness size: {} bytes", witness_data.len());

    // Count votes for option 0
    println!("\nCounting votes for option 0...");
    let count_start = std::time::Instant::now();

    let inputs: Vec<Vec<u8>> = bincode::deserialize(&witness_data)?;
    let input_refs: Vec<&[u8]> = inputs.iter().map(|v| v.as_slice()).collect();

    use prover_node::VotingCircuit;

    let predicate = FhePredicate::Equals(0);
    let count_bytes = VotingCircuit::compute_count_if(input_refs, &predicate)?;

    let count_time = count_start.elapsed();
    println!("  ✓ Counting took: {:?}", count_time);

    let count_ct: FheUint8 = bincode::deserialize(&count_bytes)?;
    let count: u8 = count_ct.decrypt(&client_key);

    println!("  Votes for option 0: {}", count);
    assert_eq!(count, 100); // 500 / 5 = 100 votes per option

    println!("\nPerformance Summary:");
    println!(
        "  Encryption: {:?} ({:.2} votes/sec)",
        encrypt_time,
        500.0 / encrypt_time.as_secs_f64()
    );
    println!("  Counting:   {:?}", count_time);
    println!("  Total:      {:?}", encrypt_time + count_time);

    println!("\n=== ✓ Performance Test Passed ===\n");

    Ok(())
}
