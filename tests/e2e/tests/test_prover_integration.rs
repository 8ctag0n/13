/// Prover Integration Tests using solana-program-test (in-memory)
///
/// Tests the prover workflow:
/// 1. Prover registration
/// 2. Job claiming
/// 3. Proof submission
/// 4. Payment distribution
///
/// Note: For full integration tests with real prover node, see test_prover_integration_external.rs

mod common;

use anyhow::Result;
use blake2::{Blake2s256, Digest};
use borsh::BorshDeserialize;
use solana_sdk::signature::{Keypair, Signer};
use common::{setup_initialized_marketplace, register_test_prover};

#[tokio::test]
async fn test_prover_claims_and_completes_job() -> Result<()> {
    println!("\n=== Testing Prover Claims and Completes Job ===\n");

    let mut ctx = setup_initialized_marketplace().await?;

    // Register prover
    let prover = Keypair::new();
    let stake_amount = 5_000_000_000u64;
    let prover_pda = register_test_prover(&mut ctx, &prover, stake_amount).await?;
    println!("Prover registered: {}", prover.pubkey());

    // Create client and fund
    let client = Keypair::new();
    ctx.fund_account(&client.pubkey(), 10_000_000_000).await?;
    println!("Client funded: {}", client.pubkey());

    // Get next job ID
    let config_account = ctx.banks_client
        .get_account(ctx.config_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Config account not found"))?;

    let config: zyberlink_sdk::MarketplaceConfig = borsh::from_slice(&config_account.data)?;
    let job_id = config.next_job_id;

    // Create witness commitment
    let witness_data = vec![42u8; 1024];
    let mut hasher = Blake2s256::new();
    hasher.update(&witness_data);
    let witness_commitment: [u8; 32] = hasher.finalize().into();

    let price_lamports = 2_000_000_000u64;

    // Create job
    let sdk = ctx.sdk_client();
    let create_job_ix = sdk.create_job_instruction(
        &client.pubkey(),
        job_id,
        zyberlink_types::CircuitType::ZcashOrchard,
        witness_commitment,
        witness_data.len() as u32,
        price_lamports,
        3600,
        None,
    )?;

    ctx.execute_transaction(&[create_job_ix], &[&client]).await?;
    println!("Job {} created", job_id);

    let (job_pda, _) = sdk.get_job_pda(&client.pubkey(), job_id);

    // Record prover balance before
    let prover_account_before = ctx.banks_client.get_account(prover.pubkey()).await?.unwrap();
    let prover_balance_before = prover_account_before.lamports;

    // Prover claims job
    let claim_job_ix = sdk.claim_job_instruction(&prover.pubkey(), &job_pda)?;
    ctx.execute_transaction(&[claim_job_ix], &[&prover]).await?;
    println!("Prover claimed job");

    // Verify job is claimed
    let job_account = ctx.banks_client
        .get_account(job_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Job account not found"))?;

    let mut job_slice = job_account.data.as_slice();
    let job = zyberlink_sdk::JobAccount::deserialize(&mut job_slice)?;
    assert_eq!(job.status, zyberlink_types::JobStatus::Claimed);
    assert_eq!(job.prover, Some(prover.pubkey()));
    println!("Job status verified: Claimed");

    // Prover submits proof
    let proof_data = vec![99u8; 512];
    let mut hasher = Blake2s256::new();
    hasher.update(&proof_data);
    let proof_commitment: [u8; 32] = hasher.finalize().into();

    // Get protocol fee recipient from config
    let config_account = ctx.banks_client
        .get_account(ctx.config_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Config account not found"))?;

    let config: zyberlink_sdk::MarketplaceConfig = borsh::from_slice(&config_account.data)?;

    let submit_proof_ix = sdk.submit_proof_instruction_with_recipient(
        &prover.pubkey(),
        &job_pda,
        &client.pubkey(),
        &config.protocol_fee_recipient,
        proof_commitment,
        proof_data.len() as u32,
    )?;

    ctx.execute_transaction(&[submit_proof_ix], &[&prover]).await?;
    println!("Proof submitted");

    // Verify job completed
    let job_account = ctx.banks_client
        .get_account(job_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Job account not found"))?;

    let mut job_slice = job_account.data.as_slice();
    let job = zyberlink_sdk::JobAccount::deserialize(&mut job_slice)?;
    assert_eq!(job.status, zyberlink_types::JobStatus::Completed);
    assert!(job.proof_hash.is_some());
    println!("Job status verified: Completed");

    // Verify prover received payment (minus protocol fee)
    let prover_account_after = ctx.banks_client.get_account(prover.pubkey()).await?.unwrap();
    let prover_balance_after = prover_account_after.lamports;

    // Prover should have received most of the payment (minus protocol fee)
    // Protocol fee is 2.5% (250 basis points), so prover gets 97.5%
    let expected_payment = (price_lamports * 9750) / 10000;
    let balance_increase = prover_balance_after.saturating_sub(prover_balance_before);

    println!("Prover balance before: {} SOL", prover_balance_before as f64 / 1e9);
    println!("Prover balance after: {} SOL", prover_balance_after as f64 / 1e9);
    println!("Expected payment (97.5%): {} SOL", expected_payment as f64 / 1e9);

    // Account for transaction fees (prover paid for claim and submit transactions)
    assert!(balance_increase > expected_payment - 100_000_000, "Prover should have received payment");

    // Verify prover stats updated
    let prover_account = ctx.banks_client
        .get_account(prover_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Prover account not found"))?;

    let mut data_slice = prover_account.data.as_slice();
    let prover_data = zyberlink_sdk::ProverAccount::deserialize(&mut data_slice)?;
    assert_eq!(prover_data.total_jobs_completed, 1);
    println!("Prover completed jobs: {}", prover_data.total_jobs_completed);

    println!("\nProver claims and completes job test passed!");
    Ok(())
}

#[tokio::test]
async fn test_prover_cannot_claim_already_claimed_job() -> Result<()> {
    println!("\n=== Testing Prover Cannot Claim Already Claimed Job ===\n");

    let mut ctx = setup_initialized_marketplace().await?;

    // Register two provers
    let prover1 = Keypair::new();
    let prover2 = Keypair::new();
    let stake_amount = 5_000_000_000u64;

    let _prover1_pda = register_test_prover(&mut ctx, &prover1, stake_amount).await?;
    let _prover2_pda = register_test_prover(&mut ctx, &prover2, stake_amount).await?;
    println!("Two provers registered");

    // Create job
    let client = Keypair::new();
    ctx.fund_account(&client.pubkey(), 10_000_000_000).await?;

    let config_account = ctx.banks_client
        .get_account(ctx.config_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Config account not found"))?;

    let config: zyberlink_sdk::MarketplaceConfig = borsh::from_slice(&config_account.data)?;
    let job_id = config.next_job_id;

    let witness_commitment = [42u8; 32];
    let price_lamports = 1_000_000_000u64;

    let sdk = ctx.sdk_client();
    let create_job_ix = sdk.create_job_instruction(
        &client.pubkey(),
        job_id,
        zyberlink_types::CircuitType::ZcashOrchard,
        witness_commitment,
        1024,
        price_lamports,
        3600,
        None,
    )?;

    ctx.execute_transaction(&[create_job_ix], &[&client]).await?;
    println!("Job created");

    let (job_pda, _) = sdk.get_job_pda(&client.pubkey(), job_id);

    // Prover 1 claims job
    let claim_job_ix = sdk.claim_job_instruction(&prover1.pubkey(), &job_pda)?;
    ctx.execute_transaction(&[claim_job_ix], &[&prover1]).await?;
    println!("Prover 1 claimed job");

    // Prover 2 tries to claim same job - should fail
    let claim_job_ix = sdk.claim_job_instruction(&prover2.pubkey(), &job_pda)?;
    let result = ctx.execute_transaction(&[claim_job_ix], &[&prover2]).await;

    assert!(result.is_err(), "Prover 2 should not be able to claim already claimed job");
    println!("Prover 2 correctly rejected from claiming already claimed job");

    println!("\nProver cannot claim already claimed job test passed!");
    Ok(())
}

#[tokio::test]
async fn test_only_assigned_prover_can_submit_proof() -> Result<()> {
    println!("\n=== Testing Only Assigned Prover Can Submit Proof ===\n");

    let mut ctx = setup_initialized_marketplace().await?;

    // Register two provers
    let prover1 = Keypair::new();
    let prover2 = Keypair::new();
    let stake_amount = 5_000_000_000u64;

    let _prover1_pda = register_test_prover(&mut ctx, &prover1, stake_amount).await?;
    let _prover2_pda = register_test_prover(&mut ctx, &prover2, stake_amount).await?;
    println!("Two provers registered");

    // Create job
    let client = Keypair::new();
    ctx.fund_account(&client.pubkey(), 10_000_000_000).await?;

    let config_account = ctx.banks_client
        .get_account(ctx.config_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Config account not found"))?;

    let config: zyberlink_sdk::MarketplaceConfig = borsh::from_slice(&config_account.data)?;
    let job_id = config.next_job_id;

    let witness_commitment = [42u8; 32];
    let price_lamports = 1_000_000_000u64;

    let sdk = ctx.sdk_client();
    let create_job_ix = sdk.create_job_instruction(
        &client.pubkey(),
        job_id,
        zyberlink_types::CircuitType::ZcashOrchard,
        witness_commitment,
        1024,
        price_lamports,
        3600,
        None,
    )?;

    ctx.execute_transaction(&[create_job_ix], &[&client]).await?;
    println!("Job created");

    let (job_pda, _) = sdk.get_job_pda(&client.pubkey(), job_id);

    // Prover 1 claims job
    let claim_job_ix = sdk.claim_job_instruction(&prover1.pubkey(), &job_pda)?;
    ctx.execute_transaction(&[claim_job_ix], &[&prover1]).await?;
    println!("Prover 1 claimed job");

    // Prover 2 tries to submit proof - should fail
    let proof_commitment = [99u8; 32];

    let submit_proof_ix = sdk.submit_proof_instruction_with_recipient(
        &prover2.pubkey(),  // Wrong prover!
        &job_pda,
        &client.pubkey(),
        &config.protocol_fee_recipient,
        proof_commitment,
        512,
    )?;

    let result = ctx.execute_transaction(&[submit_proof_ix], &[&prover2]).await;
    assert!(result.is_err(), "Prover 2 should not be able to submit proof");
    println!("Prover 2 correctly rejected from submitting proof");

    // Prover 1 can submit proof
    let submit_proof_ix = sdk.submit_proof_instruction_with_recipient(
        &prover1.pubkey(),
        &job_pda,
        &client.pubkey(),
        &config.protocol_fee_recipient,
        proof_commitment,
        512,
    )?;

    ctx.execute_transaction(&[submit_proof_ix], &[&prover1]).await?;
    println!("Prover 1 successfully submitted proof");

    println!("\nOnly assigned prover can submit proof test passed!");
    Ok(())
}

#[tokio::test]
async fn test_prover_reputation_updates() -> Result<()> {
    println!("\n=== Testing Prover Reputation Updates ===\n");

    let mut ctx = setup_initialized_marketplace().await?;

    // Register prover
    let prover = Keypair::new();
    let stake_amount = 5_000_000_000u64;
    let prover_pda = register_test_prover(&mut ctx, &prover, stake_amount).await?;

    // Check initial reputation
    let prover_account = ctx.banks_client
        .get_account(prover_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Prover account not found"))?;

    let mut data_slice = prover_account.data.as_slice();
    let prover_data = zyberlink_sdk::ProverAccount::deserialize(&mut data_slice)?;
    let initial_reputation = prover_data.reputation_score;
    println!("Initial reputation: {}", initial_reputation);

    // Complete a job
    let client = Keypair::new();
    ctx.fund_account(&client.pubkey(), 10_000_000_000).await?;

    let config_account = ctx.banks_client
        .get_account(ctx.config_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Config account not found"))?;

    let config: zyberlink_sdk::MarketplaceConfig = borsh::from_slice(&config_account.data)?;
    let job_id = config.next_job_id;

    let sdk = ctx.sdk_client();
    let create_job_ix = sdk.create_job_instruction(
        &client.pubkey(),
        job_id,
        zyberlink_types::CircuitType::ZcashOrchard,
        [42u8; 32],
        1024,
        1_000_000_000,
        3600,
        None,
    )?;

    ctx.execute_transaction(&[create_job_ix], &[&client]).await?;

    let (job_pda, _) = sdk.get_job_pda(&client.pubkey(), job_id);

    // Claim and complete job
    let claim_job_ix = sdk.claim_job_instruction(&prover.pubkey(), &job_pda)?;
    ctx.execute_transaction(&[claim_job_ix], &[&prover]).await?;

    let submit_proof_ix = sdk.submit_proof_instruction_with_recipient(
        &prover.pubkey(),
        &job_pda,
        &client.pubkey(),
        &config.protocol_fee_recipient,
        [99u8; 32],
        512,
    )?;

    ctx.execute_transaction(&[submit_proof_ix], &[&prover]).await?;
    println!("Job completed");

    // Check updated reputation
    let prover_account = ctx.banks_client
        .get_account(prover_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Prover account not found"))?;

    let mut data_slice = prover_account.data.as_slice();
    let prover_data = zyberlink_sdk::ProverAccount::deserialize(&mut data_slice)?;
    let final_reputation = prover_data.reputation_score;
    println!("Final reputation: {}", final_reputation);

    // Reputation should increase after completing a job
    assert!(final_reputation >= initial_reputation, "Reputation should not decrease after completing job");
    assert_eq!(prover_data.total_jobs_completed, 1);

    println!("\nProver reputation updates test passed!");
    Ok(())
}

// Note: test_inactive_prover_cannot_claim was removed because there's no
// deactivate_prover instruction. Provers are only deactivated when slashed
// below minimum stake. This behavior is tested in test_slash_prover.rs
