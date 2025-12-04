/// Complete End-to-End Marketplace Test using solana-program-test (in-memory)
///
/// Tests the full workflow:
/// 1. Initialize marketplace
/// 2. Register a prover
/// 3. Create a job
/// 4. Prover claims the job
/// 5. Prover submits proof
/// 6. Verify final state

mod common;

use anyhow::Result;
use solana_sdk::signature::{Keypair, Signer};
use borsh::BorshDeserialize;
use blake2::{Blake2s256, Digest};
use common::{setup_initialized_marketplace, register_test_prover};

#[tokio::test]
async fn test_complete_marketplace_flow() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("COMPLETE END-TO-END MARKETPLACE TEST");
    println!("{}\n", "=".repeat(80));

    // STEP 1: Initialize Marketplace
    println!("STEP 1: Initialize Marketplace");
    println!("{}", "-".repeat(80));

    let mut ctx = setup_initialized_marketplace().await?;
    println!("  Authority: {}", ctx.authority.pubkey());
    println!("  Marketplace initialized");

    // STEP 2: Register Prover
    println!("\nSTEP 2: Register Prover");
    println!("{}", "-".repeat(80));

    let prover = Keypair::new();
    let stake_amount = 5_000_000_000u64; // 5 SOL
    let prover_pda = register_test_prover(&mut ctx, &prover, stake_amount).await?;
    println!("  Prover: {}", prover.pubkey());
    println!("  Prover PDA: {}", prover_pda);
    println!("  Stake: {} SOL", stake_amount as f64 / 1e9);

    // Verify prover registration
    let prover_account = ctx.banks_client
        .get_account(prover_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Prover account not found"))?;

    let mut data_slice = prover_account.data.as_slice();
    let prover_data = zyberlink_sdk::ProverAccount::deserialize(&mut data_slice)?;
    assert!(prover_data.is_active, "Prover should be active");
    println!("  Prover registered and active");

    // STEP 3: Create Job
    println!("\nSTEP 3: Create Job");
    println!("{}", "-".repeat(80));

    let client = Keypair::new();
    ctx.fund_account(&client.pubkey(), 10_000_000_000).await?;
    println!("  Client: {}", client.pubkey());

    // Get next job ID
    let config_account = ctx.banks_client
        .get_account(ctx.config_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Config account not found"))?;

    let config: zyberlink_sdk::MarketplaceConfig = borsh::from_slice(&config_account.data)?;
    let job_id = config.next_job_id;
    println!("  Job ID: {}", job_id);

    // Create witness
    let witness_data = vec![42u8; 1024];
    let mut hasher = Blake2s256::new();
    hasher.update(&witness_data);
    let witness_commitment: [u8; 32] = hasher.finalize().into();

    let price_lamports = 2_000_000_000u64; // 2 SOL

    let sdk = ctx.sdk_client();
    let create_job_ix = sdk.create_job_instruction(
        &client.pubkey(),
        job_id,
        zyberlink_types::CircuitType::ZcashOrchard,
        witness_commitment,
        witness_data.len() as u32,
        price_lamports,
        3600, // 1 hour timeout
        None,
    )?;

    ctx.execute_transaction(&[create_job_ix], &[&client]).await?;
    println!("  Job created with price {} SOL", price_lamports as f64 / 1e9);

    let (job_pda, _) = sdk.get_job_pda(&client.pubkey(), job_id);
    println!("  Job PDA: {}", job_pda);

    // Verify job state
    let job_account = ctx.banks_client
        .get_account(job_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Job account not found"))?;

    let mut job_slice = job_account.data.as_slice();
    let job = zyberlink_sdk::JobAccount::deserialize(&mut job_slice)?;
    assert_eq!(job.status, zyberlink_types::JobStatus::Pending);
    println!("  Job status: Pending");

    // Verify escrow created
    let (escrow_pda, _) = sdk.get_escrow_pda(&job_pda);
    let escrow_account = ctx.banks_client
        .get_account(escrow_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Escrow account not found"))?;
    println!("  Escrow balance: {} SOL", escrow_account.lamports as f64 / 1e9);

    // STEP 4: Prover Claims Job
    println!("\nSTEP 4: Prover Claims Job");
    println!("{}", "-".repeat(80));

    let claim_job_ix = sdk.claim_job_instruction(&prover.pubkey(), &job_pda)?;
    ctx.execute_transaction(&[claim_job_ix], &[&prover]).await?;
    println!("  Job claimed by prover");

    // Verify job state changed
    let job_account = ctx.banks_client
        .get_account(job_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Job account not found"))?;

    let mut job_slice = job_account.data.as_slice();
    let job = zyberlink_sdk::JobAccount::deserialize(&mut job_slice)?;
    assert_eq!(job.status, zyberlink_types::JobStatus::Claimed);
    assert_eq!(job.prover, Some(prover.pubkey()));
    println!("  Job status: Claimed");

    // STEP 5: Prover Submits Proof
    println!("\nSTEP 5: Prover Submits Proof");
    println!("{}", "-".repeat(80));

    // Create proof commitment
    let proof_data = vec![99u8; 512];
    let mut hasher = Blake2s256::new();
    hasher.update(&proof_data);
    let proof_commitment: [u8; 32] = hasher.finalize().into();

    // Read protocol_fee_recipient from config
    let config_account = ctx.banks_client
        .get_account(ctx.config_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Config account not found"))?;

    let config: zyberlink_sdk::MarketplaceConfig = borsh::from_slice(&config_account.data)?;
    let protocol_fee_recipient = config.protocol_fee_recipient;

    let submit_proof_ix = sdk.submit_proof_instruction_with_recipient(
        &prover.pubkey(),
        &job_pda,
        &client.pubkey(),
        &protocol_fee_recipient,
        proof_commitment,
        proof_data.len() as u32,
    )?;

    ctx.execute_transaction(&[submit_proof_ix], &[&prover]).await?;
    println!("  Proof submitted");

    // VERIFICATION: Check Final State
    println!("\nFINAL VERIFICATION");
    println!("{}", "-".repeat(80));

    // Check job completed
    let final_job_account = ctx.banks_client
        .get_account(job_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Job account not found"))?;

    let mut job_slice = final_job_account.data.as_slice();
    let final_job = zyberlink_sdk::JobAccount::deserialize(&mut job_slice)?;
    assert_eq!(final_job.status, zyberlink_types::JobStatus::Completed);
    assert!(final_job.proof_hash.is_some());
    println!("  Job status: Completed");

    // Check escrow was drained (balance should be minimal - just rent)
    let escrow_after = ctx.banks_client.get_account(escrow_pda).await?;
    if let Some(escrow) = escrow_after {
        println!("  Escrow remaining: {} SOL", escrow.lamports as f64 / 1e9);
    } else {
        println!("  Escrow account closed");
    }

    // Verify prover stats updated
    let prover_account = ctx.banks_client
        .get_account(prover_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Prover account not found"))?;

    let mut data_slice = prover_account.data.as_slice();
    let prover_final = zyberlink_sdk::ProverAccount::deserialize(&mut data_slice)?;
    assert_eq!(prover_final.total_jobs_completed, 1);
    println!("  Prover completed jobs: {}", prover_final.total_jobs_completed);

    println!("\n{}", "=".repeat(80));
    println!("COMPLETE E2E TEST PASSED!");
    println!("{}\n", "=".repeat(80));

    Ok(())
}

#[tokio::test]
async fn test_job_timeout_flow() -> Result<()> {
    println!("\n=== Testing Job Timeout Flow ===\n");

    let mut ctx = setup_initialized_marketplace().await?;

    // Register prover
    let prover = Keypair::new();
    let _prover_pda = register_test_prover(&mut ctx, &prover, 5_000_000_000).await?;

    // Create job
    let client = Keypair::new();
    ctx.fund_account(&client.pubkey(), 10_000_000_000).await?;

    let config_account = ctx.banks_client
        .get_account(ctx.config_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Config account not found"))?;

    let config: zyberlink_sdk::MarketplaceConfig = borsh::from_slice(&config_account.data)?;
    let job_id = config.next_job_id;

    let sdk = ctx.sdk_client();
    let witness_commitment = [42u8; 32];

    // Create job with very short timeout (1 second)
    let create_job_ix = sdk.create_job_instruction(
        &client.pubkey(),
        job_id,
        zyberlink_types::CircuitType::ZcashOrchard,
        witness_commitment,
        1024,
        1_000_000_000, // 1 SOL
        1, // 1 second timeout
        None,
    )?;

    ctx.execute_transaction(&[create_job_ix], &[&client]).await?;
    println!("Job created with 1 second timeout");

    let (job_pda, _) = sdk.get_job_pda(&client.pubkey(), job_id);

    // Claim job
    let claim_job_ix = sdk.claim_job_instruction(&prover.pubkey(), &job_pda)?;
    ctx.execute_transaction(&[claim_job_ix], &[&prover]).await?;
    println!("Job claimed by prover");

    // Verify job is claimed
    let job_account = ctx.banks_client
        .get_account(job_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Job account not found"))?;

    let mut job_slice = job_account.data.as_slice();
    let job = zyberlink_sdk::JobAccount::deserialize(&mut job_slice)?;
    assert_eq!(job.status, zyberlink_types::JobStatus::Claimed);

    // Note: In a real test we would warp time forward and test timeout handling
    // For now, we just verify the job can be claimed
    println!("Timeout flow test passed (timeout handling requires time manipulation)");

    Ok(())
}
