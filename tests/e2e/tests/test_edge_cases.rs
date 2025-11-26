//! Edge case tests for marketplace instructions using solana-program-test
//!
//! Tests error handling for invalid inputs and edge cases

mod common;

use anyhow::Result;
use solana_sdk::signature::{Keypair, Signer};
use common::{setup_initialized_marketplace, register_test_prover};

#[tokio::test]
async fn test_register_prover_insufficient_stake() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST: Register Prover with Insufficient Stake");
    println!("{}", "=".repeat(80));

    let mut ctx = setup_initialized_marketplace().await?;

    // Get minimum stake from config
    let config_account = ctx.banks_client
        .get_account(ctx.config_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Config account not found"))?;

    let config: zyberlink_sdk::MarketplaceConfig = borsh::from_slice(&config_account.data)?;
    let min_stake = config.min_stake_amount;

    println!("  Minimum stake required: {} SOL", min_stake as f64 / 1e9);

    // Create prover with insufficient stake
    let prover = Keypair::new();
    ctx.fund_account(&prover.pubkey(), 20_000_000_000).await?;

    let insufficient_stake = min_stake - 1_000_000_000; // 1 SOL less than minimum
    println!("  Attempting to register with: {} SOL", insufficient_stake as f64 / 1e9);

    let sdk = ctx.sdk_client();
    let encryption_key = [99u8; 32];

    let register_ix = sdk.register_prover_instruction(
        &prover.pubkey(),
        insufficient_stake,
        encryption_key,
    )?;

    let result = ctx.execute_transaction(&[register_ix], &[&prover]).await;

    assert!(result.is_err(), "Should not register with insufficient stake");
    println!("  Registration failed as expected");
    println!("  Error: Stake amount below minimum");

    println!("\n{}", "=".repeat(80));
    println!("INSUFFICIENT STAKE TEST PASSED!");
    println!("{}", "=".repeat(80));

    Ok(())
}

#[tokio::test]
async fn test_create_job_zero_price() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST: Create Job with Zero Price");
    println!("{}", "=".repeat(80));

    let mut ctx = setup_initialized_marketplace().await?;

    let client = Keypair::new();
    ctx.fund_account(&client.pubkey(), 10_000_000_000).await?;

    let config_account = ctx.banks_client
        .get_account(ctx.config_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Config account not found"))?;

    let config: zyberlink_sdk::MarketplaceConfig = borsh::from_slice(&config_account.data)?;
    let job_id = config.next_job_id;

    println!("  Attempting to create job with 0 price");

    let sdk = ctx.sdk_client();
    let create_job_ix = sdk.create_job_instruction(
        &client.pubkey(),
        job_id,
        zyberlink_types::CircuitType::ZcashOrchard,
        [42u8; 32],
        2048,
        0, // Zero price - should fail
        600,
        None,
    )?;

    let result = ctx.execute_transaction(&[create_job_ix], &[&client]).await;

    assert!(result.is_err(), "Should not create job with zero price");
    println!("  Job creation failed as expected");
    println!("  Error: Price must be greater than zero");

    println!("\n{}", "=".repeat(80));
    println!("ZERO PRICE TEST PASSED!");
    println!("{}", "=".repeat(80));

    Ok(())
}

#[tokio::test]
async fn test_claim_job_twice() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST: Cannot Claim Already Claimed Job");
    println!("{}", "=".repeat(80));

    let mut ctx = setup_initialized_marketplace().await?;

    // Register two provers
    println!("\nSetup: Register Provers");
    let prover1 = Keypair::new();
    let prover2 = Keypair::new();
    let stake_amount = 5_000_000_000u64;

    let _prover1_pda = register_test_prover(&mut ctx, &prover1, stake_amount).await?;
    let _prover2_pda = register_test_prover(&mut ctx, &prover2, stake_amount).await?;
    println!("  Provers registered");

    // Create job
    println!("\nSetup: Create Job");
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
        2048,
        2_000_000_000,
        600,
        None,
    )?;

    ctx.execute_transaction(&[create_job_ix], &[&client]).await?;
    let (job_pda, _) = sdk.get_job_pda(&client.pubkey(), job_id);
    println!("  Job created");

    // Prover 1 claims job
    println!("\nTest: Prover 1 Claims Job");
    let claim_ix = sdk.claim_job_instruction(&prover1.pubkey(), &job_pda)?;
    ctx.execute_transaction(&[claim_ix], &[&prover1]).await?;
    println!("  Prover 1 claimed successfully");

    // Prover 2 tries to claim same job
    println!("\nTest: Prover 2 Tries to Claim Same Job");
    let claim_ix = sdk.claim_job_instruction(&prover2.pubkey(), &job_pda)?;
    let result = ctx.execute_transaction(&[claim_ix], &[&prover2]).await;

    assert!(result.is_err(), "Should not claim already claimed job");
    println!("  Claim failed as expected");
    println!("  Error: Job already claimed");

    println!("\n{}", "=".repeat(80));
    println!("CLAIM TWICE TEST PASSED!");
    println!("{}", "=".repeat(80));

    Ok(())
}

#[tokio::test]
async fn test_submit_proof_wrong_prover() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST: Cannot Submit Proof as Wrong Prover");
    println!("{}", "=".repeat(80));

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
    println!("Job created");

    // Prover 1 claims job
    let claim_ix = sdk.claim_job_instruction(&prover1.pubkey(), &job_pda)?;
    ctx.execute_transaction(&[claim_ix], &[&prover1]).await?;
    println!("Prover 1 claimed job");

    // Prover 2 tries to submit proof - should fail
    let submit_proof_ix = sdk.submit_proof_instruction_with_recipient(
        &prover2.pubkey(), // Wrong prover!
        &job_pda,
        &client.pubkey(),
        &config.protocol_fee_recipient,
        [99u8; 32],
        512,
    )?;

    let result = ctx.execute_transaction(&[submit_proof_ix], &[&prover2]).await;
    assert!(result.is_err(), "Wrong prover should not submit proof");
    println!("Prover 2 correctly rejected from submitting proof");

    println!("\n{}", "=".repeat(80));
    println!("WRONG PROVER SUBMIT TEST PASSED!");
    println!("{}", "=".repeat(80));

    Ok(())
}

#[tokio::test]
async fn test_cancel_claimed_job() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST: Cannot Cancel Claimed Job");
    println!("{}", "=".repeat(80));

    let mut ctx = setup_initialized_marketplace().await?;

    // Register prover
    let prover = Keypair::new();
    let stake_amount = 5_000_000_000u64;
    let _prover_pda = register_test_prover(&mut ctx, &prover, stake_amount).await?;
    println!("Prover registered");

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
    println!("Job created");

    // Prover claims job
    let claim_ix = sdk.claim_job_instruction(&prover.pubkey(), &job_pda)?;
    ctx.execute_transaction(&[claim_ix], &[&prover]).await?;
    println!("Job claimed by prover");

    // Client tries to cancel claimed job - should fail
    let cancel_ix = sdk.cancel_job_instruction(&client.pubkey(), &job_pda)?;
    let result = ctx.execute_transaction(&[cancel_ix], &[&client]).await;

    assert!(result.is_err(), "Should not cancel claimed job");
    println!("Cancel correctly rejected for claimed job");

    println!("\n{}", "=".repeat(80));
    println!("CANCEL CLAIMED JOB TEST PASSED!");
    println!("{}", "=".repeat(80));

    Ok(())
}

#[tokio::test]
async fn test_cancel_job_wrong_creator() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST: Cannot Cancel Job as Wrong Creator");
    println!("{}", "=".repeat(80));

    let mut ctx = setup_initialized_marketplace().await?;

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
    println!("Job created by client: {}", client.pubkey());

    // Someone else tries to cancel
    let impostor = Keypair::new();
    ctx.fund_account(&impostor.pubkey(), 1_000_000_000).await?;

    let cancel_ix = sdk.cancel_job_instruction(&impostor.pubkey(), &job_pda)?;
    let result = ctx.execute_transaction(&[cancel_ix], &[&impostor]).await;

    assert!(result.is_err(), "Wrong creator should not cancel job");
    println!("Cancel correctly rejected for wrong creator");

    println!("\n{}", "=".repeat(80));
    println!("WRONG CREATOR CANCEL TEST PASSED!");
    println!("{}", "=".repeat(80));

    Ok(())
}

#[tokio::test]
async fn test_claim_pending_job_unregistered_prover() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST: Unregistered Prover Cannot Claim Job");
    println!("{}", "=".repeat(80));

    let mut ctx = setup_initialized_marketplace().await?;

    // Create job (no prover registered)
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
    println!("Job created");

    // Unregistered prover tries to claim
    let unregistered_prover = Keypair::new();
    ctx.fund_account(&unregistered_prover.pubkey(), 1_000_000_000).await?;

    let claim_ix = sdk.claim_job_instruction(&unregistered_prover.pubkey(), &job_pda)?;
    let result = ctx.execute_transaction(&[claim_ix], &[&unregistered_prover]).await;

    assert!(result.is_err(), "Unregistered prover should not claim job");
    println!("Unregistered prover correctly rejected");

    println!("\n{}", "=".repeat(80));
    println!("UNREGISTERED PROVER TEST PASSED!");
    println!("{}", "=".repeat(80));

    Ok(())
}
