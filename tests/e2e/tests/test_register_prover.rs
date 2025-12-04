/// Test Register Prover using solana-program-test (in-memory)
///
/// This test verifies:
/// - Prover registration with stake
/// - Prover account created with correct data
/// - Initial reputation and active status

mod common;

use anyhow::Result;
use solana_sdk::signature::{Keypair, Signer};
use borsh::BorshDeserialize;
use common::{setup_initialized_marketplace, register_test_prover};

#[tokio::test]
async fn test_register_prover() -> Result<()> {
    println!("\n=== Testing Register Prover ===\n");

    let mut ctx = setup_initialized_marketplace().await?;

    // Generate prover keypair
    let prover = Keypair::new();
    println!("Prover Authority: {}", prover.pubkey());

    // Register prover with 5 SOL stake
    let stake_amount = 5_000_000_000u64;
    let prover_pda = register_test_prover(&mut ctx, &prover, stake_amount).await?;

    println!("Prover registered successfully!");
    println!("   Prover PDA: {}", prover_pda);

    // Verify prover account
    let prover_account = ctx.banks_client
        .get_account(prover_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Prover account not found"))?;

    println!("   Prover data length: {} bytes", prover_account.data.len());

    // Deserialize and verify prover data
    let prover_data: zyberlink_sdk::ProverAccount =
        borsh::from_slice(&prover_account.data)?;

    println!("   Authority: {}", prover_data.authority);
    println!("   Stake Amount: {} SOL", prover_data.stake_amount as f64 / 1_000_000_000.0);
    println!("   Reputation Score: {}/1000", prover_data.reputation_score);
    println!("   Is Active: {}", prover_data.is_active);

    // Verify prover data
    assert_eq!(prover_data.authority, prover.pubkey(), "Authority should match");
    assert_eq!(prover_data.stake_amount, stake_amount, "Stake should match");
    assert_eq!(prover_data.reputation_score, 1000, "Should start with perfect reputation");
    assert_eq!(prover_data.total_jobs_completed, 0, "Should have 0 completed jobs");
    assert_eq!(prover_data.total_jobs_failed, 0, "Should have 0 failed jobs");
    assert!(prover_data.is_active, "Should be active");

    println!("\nAll prover account verifications passed!");

    Ok(())
}

#[tokio::test]
async fn test_register_prover_insufficient_stake() -> Result<()> {
    println!("\n=== Testing Register Prover - Insufficient Stake ===\n");

    let mut ctx = setup_initialized_marketplace().await?;

    let prover = Keypair::new();

    // Try to register with less than min stake (config has 5 SOL min)
    let insufficient_stake = 1_000_000_000u64; // 1 SOL

    ctx.fund_account(&prover.pubkey(), 10_000_000_000).await?;

    let sdk = ctx.sdk_client();
    let encryption_key = [42u8; 32];

    let register_ix = sdk.register_prover_instruction(
        &prover.pubkey(),
        insufficient_stake,
        encryption_key,
    )?;

    let result = ctx.execute_transaction(&[register_ix], &[&prover]).await;

    assert!(result.is_err(), "Registration with insufficient stake should fail");
    println!("Registration correctly rejected for insufficient stake");

    Ok(())
}

#[tokio::test]
async fn test_register_prover_duplicate() -> Result<()> {
    println!("\n=== Testing Register Prover - Duplicate Registration ===\n");

    let mut ctx = setup_initialized_marketplace().await?;

    let prover = Keypair::new();

    // First registration
    let _prover_pda = register_test_prover(&mut ctx, &prover, 5_000_000_000).await?;
    println!("First registration succeeded");

    // Try duplicate registration - fund more SOL first
    ctx.fund_account(&prover.pubkey(), 10_000_000_000).await?;

    let sdk = ctx.sdk_client();
    let encryption_key = [42u8; 32];

    let register_ix = sdk.register_prover_instruction(
        &prover.pubkey(),
        5_000_000_000,
        encryption_key,
    )?;

    let result = ctx.execute_transaction(&[register_ix], &[&prover]).await;

    assert!(result.is_err(), "Duplicate registration should fail");
    println!("Duplicate registration correctly rejected");

    Ok(())
}
