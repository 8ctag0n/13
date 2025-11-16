/// Self-Executing FHE Tests (No External Validator Required)
///
/// These tests use solana-program-test to run a local in-memory validator.
/// Perfect for CI/CD and rapid development.
///
/// Run with: cargo test --test fhe_self_executing

mod common;
mod fhe_test_utils;

use anyhow::Result;
use common::{setup_initialized_marketplace, register_test_prover, create_test_fhe_job};
use fhe_test_utils::*;
use solana_sdk::signature::{Keypair, Signer};
use blake2::{Blake2s256, Digest as Blake2Digest};

/// Test 1: Create FHE Job (Self-Executing)
#[tokio::test]
async fn test_create_fhe_job_self_executing() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST: Create FHE Job (Self-Executing)");
    println!("{}", "=".repeat(80));

    // Setup test environment (no external validator needed!)
    println!("  1. Setting up test environment...");
    let mut ctx = setup_initialized_marketplace().await?;
    println!("     Program ID: {}", ctx.program_id);
    println!("     Config PDA: {}", ctx.config_pda);

    // Create job creator
    println!("  2. Creating FHE job...");
    let creator = Keypair::new();

    let job_pda = create_test_fhe_job(
        &mut ctx,
        &creator,
        3,  // required_provers
        2,  // consensus_threshold
    ).await?;

    println!("     Job PDA: {}", job_pda);

    // Verify job account exists
    println!("  3. Verifying job account...");
    let job_account = ctx.banks_client
        .get_account(job_pda)
        .await?
        .expect("Job account should exist");

    println!("     Job account size: {} bytes", job_account.data.len());
    println!("     Job account owner: {}", job_account.owner);

    // Deserialize and verify job data
    use cypherlink::JobAccount;

    let job: JobAccount = borsh::BorshDeserialize::deserialize(&mut &job_account.data[..])?;
    println!("     Job ID: {}", job.id);
    println!("     Job creator: {}", job.creator);
    println!("     Job status: {:?}", job.status);

    let fhe_config = job.fhe_config.expect("Should have FHE config");
    println!("     FHE config:");
    println!("       - Required provers: {}", fhe_config.required_provers);
    println!("       - Consensus threshold: {}", fhe_config.consensus_threshold);
    println!("       - Operation: {:?}", fhe_config.operation);

    assert_eq!(job.creator, creator.pubkey());
    assert_eq!(fhe_config.required_provers, 3);
    assert_eq!(fhe_config.consensus_threshold, 2);

    println!("\n  ✓ FHE job created successfully!");
    println!("{}", "=".repeat(80));

    Ok(())
}

/// Test 2: Multi-Prover Claiming (Self-Executing)
#[tokio::test]
async fn test_multi_prover_claiming_self_executing() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST: Multi-Prover Claiming (Self-Executing)");
    println!("{}", "=".repeat(80));

    // Setup
    println!("  1. Setting up test environment...");
    let mut ctx = setup_initialized_marketplace().await?;

    // Register 3 provers
    println!("  2. Registering 3 provers...");
    let prover1 = Keypair::new();
    let prover2 = Keypair::new();
    let prover3 = Keypair::new();

    register_test_prover(&mut ctx, &prover1, 5_000_000_000).await?;
    register_test_prover(&mut ctx, &prover2, 5_000_000_000).await?;
    register_test_prover(&mut ctx, &prover3, 5_000_000_000).await?;

    println!("     Prover 1: {}", prover1.pubkey());
    println!("     Prover 2: {}", prover2.pubkey());
    println!("     Prover 3: {}", prover3.pubkey());

    // Create FHE job
    println!("  3. Creating FHE job...");
    let creator = Keypair::new();
    let job_pda = create_test_fhe_job(&mut ctx, &creator, 3, 2).await?;

    // Claim job with each prover
    println!("  4. Provers claiming job...");

    let sdk = ctx.sdk_client();

    for (i, prover) in [&prover1, &prover2, &prover3].iter().enumerate() {
        let claim_ix = sdk.claim_job_instruction(&prover.pubkey(), &job_pda)?;
        ctx.execute_transaction(&[claim_ix], &[prover]).await?;
        println!("     Prover {} claimed", i + 1);
    }

    // Verify job status changed to Claimed
    println!("  5. Verifying job status...");
    let job_account = ctx.banks_client
        .get_account(job_pda)
        .await?
        .expect("Job account should exist");

    use cypherlink::JobAccount;
    use cypherlink_types::JobStatus;

    let job: JobAccount = borsh::BorshDeserialize::deserialize(&mut &job_account.data[..])?;

    assert_eq!(job.status, JobStatus::Claimed);
    assert_eq!(job.claimed_provers.len(), 3);
    assert!(job.claimed_provers.contains(&prover1.pubkey()));
    assert!(job.claimed_provers.contains(&prover2.pubkey()));
    assert!(job.claimed_provers.contains(&prover3.pubkey()));

    println!("     Job status: {:?}", job.status);
    println!("     Claimed provers: {}", job.claimed_provers.len());

    println!("\n  ✓ Multi-prover claiming successful!");
    println!("{}", "=".repeat(80));

    Ok(())
}

/// Test 3: FHE Result Submission (Self-Executing)
#[tokio::test]
async fn test_fhe_result_submission_self_executing() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST: FHE Result Submission (Self-Executing)");
    println!("{}", "=".repeat(80));

    // Setup
    let mut ctx = setup_initialized_marketplace().await?;

    // Register provers
    let prover1 = Keypair::new();
    let prover2 = Keypair::new();
    let prover3 = Keypair::new();

    register_test_prover(&mut ctx, &prover1, 5_000_000_000).await?;
    register_test_prover(&mut ctx, &prover2, 5_000_000_000).await?;
    register_test_prover(&mut ctx, &prover3, 5_000_000_000).await?;

    // Create and claim job
    let creator = Keypair::new();
    let job_pda = create_test_fhe_job(&mut ctx, &creator, 3, 2).await?;

    let sdk = ctx.sdk_client();

    for prover in [&prover1, &prover2, &prover3] {
        let claim_ix = sdk.claim_job_instruction(&prover.pubkey(), &job_pda)?;
        ctx.execute_transaction(&[claim_ix], &[prover]).await?;
    }

    println!("  1. Job created and claimed by 3 provers");

    // Perform FHE computation off-chain
    println!("  2. Performing FHE computations...");
    let (client_key, server_key) = setup_fhe_keys()?;

    let input_value = 100u8;
    let encrypted_input = encrypt_value(input_value, &client_key)?;

    // All 3 provers compute same result
    let result1 = compute_fhe_add(&encrypted_input, 10, &server_key)?;
    let result2 = compute_fhe_add(&encrypted_input, 10, &server_key)?;
    let result3 = compute_fhe_add(&encrypted_input, 10, &server_key)?;

    let hash1 = hash_fhe_result(&result1);
    let hash2 = hash_fhe_result(&result2);
    let hash3 = hash_fhe_result(&result3);

    println!("     Hash 1: {}", hex::encode(&hash1[..8]));
    println!("     Hash 2: {}", hex::encode(&hash2[..8]));
    println!("     Hash 3: {}", hex::encode(&hash3[..8]));

    assert_eq!(hash1, hash2);
    assert_eq!(hash2, hash3);

    // Submit results on-chain
    println!("  3. Submitting FHE results on-chain...");

    for (i, (prover, hash)) in [(prover1, hash1), (prover2, hash2), (prover3, hash3)]
        .iter()
        .enumerate()
    {
        let submit_ix = sdk.submit_fhe_result_instruction(
            &prover.pubkey(),
            &job_pda,
            *hash,
        )?;

        ctx.execute_transaction(&[submit_ix], &[prover]).await?;
        println!("     Prover {} submitted result", i + 1);
    }

    // Verify results stored in job
    println!("  4. Verifying results stored...");
    let job_account = ctx.banks_client
        .get_account(job_pda)
        .await?
        .expect("Job account should exist");

    use cypherlink::JobAccount;

    let job: JobAccount = borsh::BorshDeserialize::deserialize(&mut &job_account.data[..])?;

    assert_eq!(job.fhe_results.len(), 3);
    println!("     Stored results: {}", job.fhe_results.len());

    println!("\n  ✓ FHE result submission successful!");
    println!("{}", "=".repeat(80));

    Ok(())
}

/// Test 4: Consensus and Finalization (Self-Executing)
#[tokio::test]
async fn test_consensus_finalization_self_executing() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST: Consensus and Finalization (Self-Executing)");
    println!("{}", "=".repeat(80));

    // Setup
    let mut ctx = setup_initialized_marketplace().await?;

    // Register provers
    let prover1 = Keypair::new();
    let prover2 = Keypair::new();
    let prover3 = Keypair::new();

    register_test_prover(&mut ctx, &prover1, 5_000_000_000).await?;
    register_test_prover(&mut ctx, &prover2, 5_000_000_000).await?;
    register_test_prover(&mut ctx, &prover3, 5_000_000_000).await?;

    // Create and claim job
    let creator = Keypair::new();
    let job_pda = create_test_fhe_job(&mut ctx, &creator, 3, 2).await?;

    let sdk = ctx.sdk_client();

    for prover in [&prover1, &prover2, &prover3] {
        let claim_ix = sdk.claim_job_instruction(&prover.pubkey(), &job_pda)?;
        ctx.execute_transaction(&[claim_ix], &[prover]).await?;
    }

    // Compute FHE results
    let (client_key, server_key) = setup_fhe_keys()?;
    let encrypted_input = encrypt_value(100, &client_key)?;

    // Provers 1 & 2: Correct result
    let correct_result = compute_fhe_add(&encrypted_input, 10, &server_key)?;
    let correct_hash = hash_fhe_result(&correct_result);

    // Prover 3: Wrong result
    let wrong_result = compute_fhe_multiply(&encrypted_input, 2, &server_key)?;
    let wrong_hash = hash_fhe_result(&wrong_result);

    println!("  1. FHE computations complete:");
    println!("     Provers 1&2 (correct): {}", hex::encode(&correct_hash[..8]));
    println!("     Prover 3 (wrong):      {}", hex::encode(&wrong_hash[..8]));

    // Submit results
    println!("  2. Submitting results...");
    for (prover, hash) in [
        (&prover1, correct_hash),
        (&prover2, correct_hash),
        (&prover3, wrong_hash),
    ] {
        let submit_ix = sdk.submit_fhe_result_instruction(
            &prover.pubkey(),
            &job_pda,
            hash,
        )?;
        ctx.execute_transaction(&[submit_ix], &[prover]).await?;
    }

    // Finalize job
    println!("  3. Finalizing job with consensus...");
    let finalizer = Keypair::new();
    ctx.fund_account(&finalizer.pubkey(), 1_000_000_000).await?;

    let finalize_ix = sdk.finalize_fhe_job_instruction_with_recipient(
        &finalizer.pubkey(),
        &job_pda,
        &creator.pubkey(),
        &ctx.authority.pubkey(),  // protocol fee recipient
        &[prover1.pubkey(), prover2.pubkey()],  // matching provers
    )?;

    ctx.execute_transaction(&[finalize_ix], &[&finalizer]).await?;

    // Verify job finalized
    println!("  4. Verifying finalization...");
    let job_account = ctx.banks_client
        .get_account(job_pda)
        .await?
        .expect("Job account should exist");

    use cypherlink::JobAccount;
    use cypherlink_types::JobStatus;

    let job: JobAccount = borsh::BorshDeserialize::deserialize(&mut &job_account.data[..])?;

    assert_eq!(job.status, JobStatus::Completed);
    assert!(job.fhe_consensus_hash.is_some());
    assert_eq!(job.fhe_consensus_hash.unwrap(), correct_hash);

    println!("     Job status: {:?}", job.status);
    println!("     Consensus hash: {}", hex::encode(job.fhe_consensus_hash.unwrap()));

    println!("\n  ✓ Consensus and finalization successful!");
    println!("{}", "=".repeat(80));

    Ok(())
}
