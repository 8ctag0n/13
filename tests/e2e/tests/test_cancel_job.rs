/// Test CancelJob instruction using solana-program-test (in-memory)
///
/// Tests the ability of job creators to cancel pending jobs
/// and recover escrowed funds

mod common;

use anyhow::Result;
use blake2::{Blake2s256, Digest};
use zyberlink_types::CircuitType;
use solana_sdk::signature::{Keypair, Signer};
use borsh::BorshDeserialize;
use common::{setup_initialized_marketplace, register_test_prover};

/// Helper to create a ZK job
async fn create_zk_job(
    ctx: &mut common::TestContext,
    creator: &Keypair,
    price_lamports: u64,
) -> Result<(solana_sdk::pubkey::Pubkey, u64)> {
    // Fund creator
    ctx.fund_account(&creator.pubkey(), 10_000_000_000).await?;

    // Get next job ID from config
    let config_account = ctx.banks_client
        .get_account(ctx.config_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Config account not found"))?;

    let config: zyberlink_sdk::MarketplaceConfig =
        borsh::from_slice(&config_account.data)?;
    let job_id = config.next_job_id;

    // Create witness commitment
    let witness_data = vec![42u8; 1024];
    let mut hasher = Blake2s256::new();
    hasher.update(&witness_data);
    let witness_commitment: [u8; 32] = hasher.finalize().into();

    // Create job
    let sdk = ctx.sdk_client();
    let create_job_ix = sdk.create_job_instruction(
        &creator.pubkey(),
        job_id,
        CircuitType::ZcashOrchard,
        witness_commitment,
        2048,
        price_lamports,
        600, // 10 min timeout
        None,
    )?;

    ctx.execute_transaction(&[create_job_ix], &[creator]).await?;

    // Return job PDA
    let job_id_bytes = job_id.to_le_bytes();
    let (job_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[b"job", creator.pubkey().as_ref(), &job_id_bytes],
        &ctx.program_id,
    );

    Ok((job_pda, job_id))
}

#[tokio::test]
async fn test_cancel_pending_job() -> Result<()> {
    println!("\n=== Testing Cancel Pending Job ===\n");

    let mut ctx = setup_initialized_marketplace().await?;

    let client = Keypair::new();
    println!("Client: {}", client.pubkey());

    // Create job
    let price_lamports = 2_000_000_000u64; // 2 SOL
    let (job_pda, job_id) = create_zk_job(&mut ctx, &client, price_lamports).await?;
    println!("Job created: {} (ID: {})", job_pda, job_id);

    // Get client balance before cancel
    let client_account_before = ctx.banks_client
        .get_account(client.pubkey())
        .await?
        .ok_or_else(|| anyhow::anyhow!("Client account not found"))?;
    let balance_before = client_account_before.lamports;
    println!("Client balance before cancel: {} SOL", balance_before as f64 / 1_000_000_000.0);

    // Cancel job
    let sdk = ctx.sdk_client();
    let cancel_job_ix = sdk.cancel_job_instruction(&client.pubkey(), &job_pda)?;
    ctx.execute_transaction(&[cancel_job_ix], &[&client]).await?;
    println!("Job cancelled successfully");

    // Verify job status is Cancelled
    let job_account = ctx.banks_client
        .get_account(job_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Job account not found"))?;

    // Use deserialize with mutable slice to allow partial reads (account has fixed 203 bytes)
    let mut data_slice = job_account.data.as_slice();
    let job = zyberlink_sdk::JobAccount::deserialize(&mut data_slice)?;
    assert_eq!(job.status, zyberlink_types::JobStatus::Cancelled, "Job should be cancelled");
    println!("Job status verified: Cancelled");

    // Check client got refund
    let client_account_after = ctx.banks_client
        .get_account(client.pubkey())
        .await?
        .ok_or_else(|| anyhow::anyhow!("Client account not found"))?;
    let balance_after = client_account_after.lamports;
    println!("Client balance after cancel: {} SOL", balance_after as f64 / 1_000_000_000.0);

    // Client should have recovered the price (minus small tx fees)
    let refund = balance_after as i64 - balance_before as i64;
    println!("Refund received: {} lamports", refund);
    assert!(refund > 0, "Client should receive refund");

    println!("\nCancel pending job test passed!");
    Ok(())
}

#[tokio::test]
async fn test_cannot_cancel_claimed_job() -> Result<()> {
    println!("\n=== Testing Cannot Cancel Claimed Job ===\n");

    let mut ctx = setup_initialized_marketplace().await?;

    // Register prover
    let prover = Keypair::new();
    let _prover_pda = register_test_prover(&mut ctx, &prover, 5_000_000_000).await?;
    println!("Prover registered: {}", prover.pubkey());

    // Create job
    let client = Keypair::new();
    let (job_pda, job_id) = create_zk_job(&mut ctx, &client, 2_000_000_000).await?;
    println!("Job created: {} (ID: {})", job_pda, job_id);

    // Claim job
    let sdk = ctx.sdk_client();
    let claim_job_ix = sdk.claim_job_instruction(&prover.pubkey(), &job_pda)?;
    ctx.execute_transaction(&[claim_job_ix], &[&prover]).await?;
    println!("Job claimed by prover");

    // Verify job is claimed
    let job_account = ctx.banks_client
        .get_account(job_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Job account not found"))?;
    let mut data_slice = job_account.data.as_slice();
    let job = zyberlink_sdk::JobAccount::deserialize(&mut data_slice)?;
    assert_eq!(job.status, zyberlink_types::JobStatus::Claimed, "Job should be claimed");

    // Try to cancel (should fail)
    let cancel_job_ix = sdk.cancel_job_instruction(&client.pubkey(), &job_pda)?;
    let result = ctx.execute_transaction(&[cancel_job_ix], &[&client]).await;

    assert!(result.is_err(), "Should not be able to cancel claimed job");
    println!("Cancel correctly rejected for claimed job");

    println!("\nCannot cancel claimed job test passed!");
    Ok(())
}

#[tokio::test]
async fn test_cannot_cancel_other_users_job() -> Result<()> {
    println!("\n=== Testing Cannot Cancel Other User's Job ===\n");

    let mut ctx = setup_initialized_marketplace().await?;

    // Creator makes a job
    let creator = Keypair::new();
    let (job_pda, job_id) = create_zk_job(&mut ctx, &creator, 2_000_000_000).await?;
    println!("Job created by {}: {} (ID: {})", creator.pubkey(), job_pda, job_id);

    // Another user tries to cancel
    let other_user = Keypair::new();
    ctx.fund_account(&other_user.pubkey(), 1_000_000_000).await?;

    let sdk = ctx.sdk_client();
    let cancel_job_ix = sdk.cancel_job_instruction(&other_user.pubkey(), &job_pda)?;
    let result = ctx.execute_transaction(&[cancel_job_ix], &[&other_user]).await;

    assert!(result.is_err(), "Other user should not be able to cancel job");
    println!("Cancel correctly rejected for unauthorized user");

    println!("\nCannot cancel other user's job test passed!");
    Ok(())
}
