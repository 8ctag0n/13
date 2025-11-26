/// Test SlashProver instruction using solana-program-test (in-memory)
///
/// Tests the authority's ability to slash misbehaving provers

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
async fn test_slash_prover_reduces_stake() -> Result<()> {
    println!("\n=== Testing Slash Prover Reduces Stake ===\n");

    let mut ctx = setup_initialized_marketplace().await?;

    // Register prover with 5 SOL stake
    let prover = Keypair::new();
    let initial_stake = 5_000_000_000u64;
    let prover_pda = register_test_prover(&mut ctx, &prover, initial_stake).await?;
    println!("Prover registered: {} with {} SOL stake", prover.pubkey(), initial_stake as f64 / 1e9);

    // Get initial prover state
    let prover_account = ctx.banks_client
        .get_account(prover_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Prover account not found"))?;
    let mut data_slice = prover_account.data.as_slice();
    let initial_prover: zyberlink_sdk::ProverAccount =
        zyberlink_sdk::ProverAccount::deserialize(&mut data_slice)?;
    println!("Initial stake: {} SOL", initial_prover.stake_amount as f64 / 1e9);
    println!("Initial reputation: {}", initial_prover.reputation_score);

    // Create a job
    let client = Keypair::new();
    let (job_pda, job_id) = create_zk_job(&mut ctx, &client, 2_000_000_000).await?;
    println!("Job created: {} (ID: {})", job_pda, job_id);

    // Claim job
    let sdk = ctx.sdk_client();
    let claim_job_ix = sdk.claim_job_instruction(&prover.pubkey(), &job_pda)?;
    ctx.execute_transaction(&[claim_job_ix], &[&prover]).await?;
    println!("Job claimed by prover");

    // Slash prover (using marketplace authority)
    let authority = Keypair::from_bytes(&ctx.authority.to_bytes())?;
    let slash_ix = sdk.slash_prover_instruction(
        &authority.pubkey(),
        &job_pda,
        &prover.pubkey(),
    )?;
    ctx.execute_transaction(&[slash_ix], &[&authority]).await?;
    println!("Prover slashed by authority");

    // Verify prover was slashed
    let prover_account = ctx.banks_client
        .get_account(prover_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Prover account not found"))?;
    let mut data_slice = prover_account.data.as_slice();
    let slashed_prover: zyberlink_sdk::ProverAccount =
        zyberlink_sdk::ProverAccount::deserialize(&mut data_slice)?;

    println!("After slash stake: {} SOL", slashed_prover.stake_amount as f64 / 1e9);
    println!("After slash reputation: {}", slashed_prover.reputation_score);

    // Verify stake was reduced
    assert!(slashed_prover.stake_amount < initial_prover.stake_amount,
        "Stake should be reduced after slash");
    // Verify reputation was reduced
    assert!(slashed_prover.reputation_score < initial_prover.reputation_score,
        "Reputation should be reduced after slash");

    println!("\nSlash prover test passed!");
    Ok(())
}

#[tokio::test]
async fn test_unauthorized_cannot_slash() -> Result<()> {
    println!("\n=== Testing Unauthorized Cannot Slash ===\n");

    let mut ctx = setup_initialized_marketplace().await?;

    // Register prover
    let prover = Keypair::new();
    let _prover_pda = register_test_prover(&mut ctx, &prover, 5_000_000_000).await?;
    println!("Prover registered: {}", prover.pubkey());

    // Create a job
    let client = Keypair::new();
    let (job_pda, job_id) = create_zk_job(&mut ctx, &client, 2_000_000_000).await?;
    println!("Job created: {} (ID: {})", job_pda, job_id);

    // Claim job
    let sdk = ctx.sdk_client();
    let claim_job_ix = sdk.claim_job_instruction(&prover.pubkey(), &job_pda)?;
    ctx.execute_transaction(&[claim_job_ix], &[&prover]).await?;
    println!("Job claimed by prover");

    // Try to slash with unauthorized user
    let unauthorized = Keypair::new();
    ctx.fund_account(&unauthorized.pubkey(), 1_000_000_000).await?;

    let slash_ix = sdk.slash_prover_instruction(
        &unauthorized.pubkey(),
        &job_pda,
        &prover.pubkey(),
    )?;
    let result = ctx.execute_transaction(&[slash_ix], &[&unauthorized]).await;

    assert!(result.is_err(), "Unauthorized user should not be able to slash");
    println!("Slash correctly rejected for unauthorized user");

    println!("\nUnauthorized cannot slash test passed!");
    Ok(())
}

#[tokio::test]
async fn test_cannot_slash_pending_job() -> Result<()> {
    println!("\n=== Testing Cannot Slash Pending Job ===\n");

    let mut ctx = setup_initialized_marketplace().await?;

    // Register prover
    let prover = Keypair::new();
    let _prover_pda = register_test_prover(&mut ctx, &prover, 5_000_000_000).await?;
    println!("Prover registered: {}", prover.pubkey());

    // Create a job but don't claim it
    let client = Keypair::new();
    let (job_pda, job_id) = create_zk_job(&mut ctx, &client, 2_000_000_000).await?;
    println!("Job created (pending): {} (ID: {})", job_pda, job_id);

    // Try to slash without job being claimed
    let authority = Keypair::from_bytes(&ctx.authority.to_bytes())?;
    let sdk = ctx.sdk_client();
    let slash_ix = sdk.slash_prover_instruction(
        &authority.pubkey(),
        &job_pda,
        &prover.pubkey(),
    )?;
    let result = ctx.execute_transaction(&[slash_ix], &[&authority]).await;

    assert!(result.is_err(), "Should not be able to slash for pending job");
    println!("Slash correctly rejected for pending job");

    println!("\nCannot slash pending job test passed!");
    Ok(())
}
