/// Test Initialize Marketplace using solana-program-test (in-memory)
///
/// This test verifies the Initialize instruction creates a valid config PDA
/// with the correct parameters.

mod common;

use anyhow::Result;
use solana_sdk::signature::{Keypair, Signer};
use borsh::BorshDeserialize;
use common::{setup_test_environment, TestContext};

#[tokio::test]
async fn test_initialize_marketplace() -> Result<()> {
    println!("\n=== Testing Initialize Marketplace ===\n");

    let mut ctx = setup_test_environment().await?;

    // Fund authority
    println!("Funding authority...");
    ctx.fund_account(&ctx.authority.pubkey(), 10_000_000_000).await?;

    // Create SDK client
    let sdk = ctx.sdk_client();
    let authority_pubkey = ctx.authority.pubkey();

    // Build Initialize instruction
    println!("Creating Initialize instruction...");
    let initialize_ix = sdk.initialize_instruction(
        &authority_pubkey,
        1000,           // 10% platform fee
        1_000_000_000,  // 1 SOL minimum stake
        500,            // 500/1000 minimum reputation
        600,            // 10 minutes default timeout
    )?;

    // Execute transaction
    println!("Sending Initialize transaction...");
    let authority_clone = Keypair::from_bytes(&ctx.authority.to_bytes())?;
    ctx.execute_transaction(&[initialize_ix], &[&authority_clone]).await?;

    println!("\nInitialize succeeded!");

    // Verify config was created
    let (config_pda, _) = sdk.get_config_pda();
    let config_account = ctx.banks_client
        .get_account(config_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Config account not found"))?;

    println!("   Config PDA: {}", config_pda);
    println!("   Config data length: {} bytes", config_account.data.len());

    // Deserialize and verify config
    let config: zyberlink_sdk::MarketplaceConfig =
        borsh::from_slice(&config_account.data)?;

    println!("   Platform fee: {}%", config.fee_basis_points as f64 / 100.0);
    println!("   Min stake: {} SOL", config.min_stake_amount as f64 / 1_000_000_000.0);
    println!("   Min reputation: {}", config.min_reputation_score);
    println!("   Default timeout: {} seconds", config.default_job_timeout_seconds);
    println!("   Authority: {}", config.authority);

    assert_eq!(config.fee_basis_points, 1000);
    assert_eq!(config.min_stake_amount, 1_000_000_000);
    assert_eq!(config.min_reputation_score, 500);
    assert_eq!(config.default_job_timeout_seconds, 600);
    assert_eq!(config.authority, authority_pubkey);
    assert_eq!(config.next_job_id, 0);

    println!("\nAll assertions passed!");

    Ok(())
}

#[tokio::test]
async fn test_initialize_idempotent() -> Result<()> {
    println!("\n=== Testing Initialize is Idempotent ===\n");

    let mut ctx = setup_test_environment().await?;

    // Fund authority
    ctx.fund_account(&ctx.authority.pubkey(), 10_000_000_000).await?;

    let sdk = ctx.sdk_client();
    let authority_pubkey = ctx.authority.pubkey();
    let authority_clone = Keypair::from_bytes(&ctx.authority.to_bytes())?;

    // First initialization
    let initialize_ix = sdk.initialize_instruction(
        &authority_pubkey,
        500,
        2_000_000_000,
        100,
        300,
    )?;

    ctx.execute_transaction(&[initialize_ix], &[&authority_clone]).await?;
    println!("First initialization succeeded");

    // Try to initialize again - should fail
    let authority_clone2 = Keypair::from_bytes(&ctx.authority.to_bytes())?;
    let initialize_ix2 = sdk.initialize_instruction(
        &authority_pubkey,
        1000,
        3_000_000_000,
        200,
        600,
    )?;

    let result = ctx.execute_transaction(&[initialize_ix2], &[&authority_clone2]).await;

    assert!(result.is_err(), "Second initialization should fail");
    println!("Second initialization correctly rejected");

    // Verify original config unchanged
    let (config_pda, _) = sdk.get_config_pda();
    let config_account = ctx.banks_client
        .get_account(config_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Config account not found"))?;

    let config: zyberlink_sdk::MarketplaceConfig =
        borsh::from_slice(&config_account.data)?;

    assert_eq!(config.fee_basis_points, 500, "Config should be unchanged");
    println!("Config verified unchanged");

    Ok(())
}
