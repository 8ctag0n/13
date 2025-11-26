//! Multi-Prover Competition Tests using solana-program-test (in-memory)
//!
//! Tests behavior when multiple provers compete for jobs.

mod common;

use anyhow::Result;
use solana_sdk::signature::{Keypair, Signer};
use common::{setup_initialized_marketplace, register_test_prover};

#[tokio::test]
async fn test_two_provers_one_job_race() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST: Two Provers Competing for One Job");
    println!("{}", "=".repeat(80));

    let mut ctx = setup_initialized_marketplace().await?;

    // Register two provers
    println!("\nSetup: Register Two Provers");
    println!("{}", "-".repeat(80));

    let prover1 = Keypair::new();
    let prover2 = Keypair::new();
    let stake_amount = 5_000_000_000u64;

    let _prover1_pda = register_test_prover(&mut ctx, &prover1, stake_amount).await?;
    let _prover2_pda = register_test_prover(&mut ctx, &prover2, stake_amount).await?;

    println!("  Prover 1: {}", prover1.pubkey());
    println!("  Prover 2: {}", prover2.pubkey());

    // Create one job
    println!("\nSetup: Create One Job");
    println!("{}", "-".repeat(80));

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
        2_000_000_000,
        3600,
        None,
    )?;

    ctx.execute_transaction(&[create_job_ix], &[&client]).await?;
    let (job_pda, _) = sdk.get_job_pda(&client.pubkey(), job_id);

    println!("  Job created: {}", job_pda);

    // Test: First prover claims
    println!("\nTest: Both Provers Attempt to Claim");
    println!("{}", "-".repeat(80));

    // Prover 1 claims first
    let claim_job_ix = sdk.claim_job_instruction(&prover1.pubkey(), &job_pda)?;
    ctx.execute_transaction(&[claim_job_ix], &[&prover1]).await?;
    println!("  Prover 1: SUCCESS (claimed first)");

    // Prover 2 tries to claim - should fail
    let claim_job_ix = sdk.claim_job_instruction(&prover2.pubkey(), &job_pda)?;
    let result = ctx.execute_transaction(&[claim_job_ix], &[&prover2]).await;

    assert!(result.is_err(), "Prover 2 should fail to claim already claimed job");
    println!("  Prover 2: FAILED (job already claimed)");

    println!("  Exactly one prover claimed the job");

    println!("\n{}", "=".repeat(80));
    println!("RACE CONDITION TEST PASSED!");
    println!("{}", "=".repeat(80));

    Ok(())
}

#[tokio::test]
async fn test_three_provers_three_jobs_concurrent() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST: Three Provers Processing Three Jobs");
    println!("{}", "=".repeat(80));

    let mut ctx = setup_initialized_marketplace().await?;

    // Register three provers
    println!("\nSetup: Register Three Provers");
    println!("{}", "-".repeat(80));

    let prover1 = Keypair::new();
    let prover2 = Keypair::new();
    let prover3 = Keypair::new();
    let stake_amount = 5_000_000_000u64;

    let _prover1_pda = register_test_prover(&mut ctx, &prover1, stake_amount).await?;
    let _prover2_pda = register_test_prover(&mut ctx, &prover2, stake_amount).await?;
    let _prover3_pda = register_test_prover(&mut ctx, &prover3, stake_amount).await?;

    println!("  Prover 1: {}", prover1.pubkey());
    println!("  Prover 2: {}", prover2.pubkey());
    println!("  Prover 3: {}", prover3.pubkey());

    // Create three jobs
    println!("\nSetup: Create Three Jobs");
    println!("{}", "-".repeat(80));

    let sdk = ctx.sdk_client();
    let mut job_pdas = vec![];

    for i in 0..3 {
        let client = Keypair::new();
        ctx.fund_account(&client.pubkey(), 10_000_000_000).await?;

        let config_account = ctx.banks_client
            .get_account(ctx.config_pda)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Config account not found"))?;

        let config: zyberlink_sdk::MarketplaceConfig = borsh::from_slice(&config_account.data)?;
        let job_id = config.next_job_id;

        let create_job_ix = sdk.create_job_instruction(
            &client.pubkey(),
            job_id,
            zyberlink_types::CircuitType::ZcashOrchard,
            [i as u8; 32],
            1024,
            1_000_000_000,
            3600,
            None,
        )?;

        ctx.execute_transaction(&[create_job_ix], &[&client]).await?;
        let (job_pda, _) = sdk.get_job_pda(&client.pubkey(), job_id);
        job_pdas.push(job_pda);

        println!("  Job {}: {}", i + 1, job_pda);
    }

    // Each prover claims a different job
    println!("\nTest: Each Prover Claims a Different Job");
    println!("{}", "-".repeat(80));

    let provers = [&prover1, &prover2, &prover3];
    for (i, (prover, job_pda)) in provers.iter().zip(job_pdas.iter()).enumerate() {
        let claim_job_ix = sdk.claim_job_instruction(&prover.pubkey(), job_pda)?;
        ctx.execute_transaction(&[claim_job_ix], &[prover]).await?;
        println!("  Prover {} claimed job {} successfully", i + 1, i + 1);
    }

    println!("\n{}", "=".repeat(80));
    println!("CONCURRENT PROCESSING TEST PASSED!");
    println!("{}", "=".repeat(80));

    Ok(())
}

#[tokio::test]
async fn test_five_provers_two_jobs() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST: Five Provers Competing for Two Jobs");
    println!("{}", "=".repeat(80));

    let mut ctx = setup_initialized_marketplace().await?;

    // Register five provers
    println!("\nSetup: Register Five Provers");
    println!("{}", "-".repeat(80));

    let mut provers = vec![];
    let stake_amount = 5_000_000_000u64;

    for i in 0..5 {
        let prover = Keypair::new();
        let _prover_pda = register_test_prover(&mut ctx, &prover, stake_amount).await?;
        println!("  Prover {}: {}", i + 1, prover.pubkey());
        provers.push(prover);
    }

    // Create two jobs
    println!("\nSetup: Create Two Jobs");
    println!("{}", "-".repeat(80));

    let sdk = ctx.sdk_client();
    let mut job_pdas = vec![];

    for i in 0..2 {
        let client = Keypair::new();
        ctx.fund_account(&client.pubkey(), 10_000_000_000).await?;

        let config_account = ctx.banks_client
            .get_account(ctx.config_pda)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Config account not found"))?;

        let config: zyberlink_sdk::MarketplaceConfig = borsh::from_slice(&config_account.data)?;
        let job_id = config.next_job_id;

        let create_job_ix = sdk.create_job_instruction(
            &client.pubkey(),
            job_id,
            zyberlink_types::CircuitType::ZcashOrchard,
            [i as u8; 32],
            1024,
            1_000_000_000,
            3600,
            None,
        )?;

        ctx.execute_transaction(&[create_job_ix], &[&client]).await?;
        let (job_pda, _) = sdk.get_job_pda(&client.pubkey(), job_id);
        job_pdas.push(job_pda);

        println!("  Job {}: {}", i + 1, job_pda);
    }

    // All provers try to claim jobs
    println!("\nTest: All Five Provers Attempt to Claim Jobs");
    println!("{}", "-".repeat(80));

    let mut successful_claims = 0;

    // First two provers claim the two jobs
    for (i, job_pda) in job_pdas.iter().enumerate() {
        let prover = &provers[i];
        let claim_job_ix = sdk.claim_job_instruction(&prover.pubkey(), job_pda)?;
        ctx.execute_transaction(&[claim_job_ix], &[prover]).await?;
        println!("  Prover {} claimed job {}", i + 1, i + 1);
        successful_claims += 1;
    }

    // Remaining provers try to claim already claimed jobs - should fail
    for i in 2..5 {
        let prover = &provers[i];

        // Try job 1
        let claim_job_ix = sdk.claim_job_instruction(&prover.pubkey(), &job_pdas[0])?;
        let result1 = ctx.execute_transaction(&[claim_job_ix], &[prover]).await;

        // Try job 2
        let claim_job_ix = sdk.claim_job_instruction(&prover.pubkey(), &job_pdas[1])?;
        let result2 = ctx.execute_transaction(&[claim_job_ix], &[prover]).await;

        assert!(result1.is_err() && result2.is_err(),
            "Prover {} should fail to claim any job", i + 1);
        println!("  Prover {} failed to claim any job (all taken)", i + 1);
    }

    assert_eq!(successful_claims, 2, "Exactly 2 jobs should be claimed");
    println!("  Exactly 2 provers claimed jobs (3 remained idle)");

    println!("\n{}", "=".repeat(80));
    println!("MORE PROVERS THAN JOBS TEST PASSED!");
    println!("{}", "=".repeat(80));

    Ok(())
}

#[tokio::test]
async fn test_prover_can_claim_multiple_jobs_sequentially() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST: One Prover Claims Multiple Jobs Sequentially");
    println!("{}", "=".repeat(80));

    let mut ctx = setup_initialized_marketplace().await?;

    // Register one prover
    let prover = Keypair::new();
    let stake_amount = 5_000_000_000u64;
    let prover_pda = register_test_prover(&mut ctx, &prover, stake_amount).await?;
    println!("Prover registered: {}", prover.pubkey());

    let sdk = ctx.sdk_client();

    // Create and complete multiple jobs
    for job_num in 0..3 {
        println!("\n--- Job {} ---", job_num + 1);

        // Create job
        let client = Keypair::new();
        ctx.fund_account(&client.pubkey(), 10_000_000_000).await?;

        let config_account = ctx.banks_client
            .get_account(ctx.config_pda)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Config account not found"))?;

        let config: zyberlink_sdk::MarketplaceConfig = borsh::from_slice(&config_account.data)?;
        let job_id = config.next_job_id;

        let create_job_ix = sdk.create_job_instruction(
            &client.pubkey(),
            job_id,
            zyberlink_types::CircuitType::ZcashOrchard,
            [job_num as u8; 32],
            1024,
            1_000_000_000,
            3600,
            None,
        )?;

        ctx.execute_transaction(&[create_job_ix], &[&client]).await?;
        let (job_pda, _) = sdk.get_job_pda(&client.pubkey(), job_id);
        println!("  Created job {}", job_id);

        // Claim job
        let claim_job_ix = sdk.claim_job_instruction(&prover.pubkey(), &job_pda)?;
        ctx.execute_transaction(&[claim_job_ix], &[&prover]).await?;
        println!("  Claimed job");

        // Submit proof
        let submit_proof_ix = sdk.submit_proof_instruction_with_recipient(
            &prover.pubkey(),
            &job_pda,
            &client.pubkey(),
            &config.protocol_fee_recipient,
            [99u8; 32],
            512,
        )?;

        ctx.execute_transaction(&[submit_proof_ix], &[&prover]).await?;
        println!("  Completed job");
    }

    // Verify prover stats
    let prover_account = ctx.banks_client
        .get_account(prover_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Prover account not found"))?;

    use borsh::BorshDeserialize;
    let mut data_slice = prover_account.data.as_slice();
    let prover_data = zyberlink_sdk::ProverAccount::deserialize(&mut data_slice)?;

    assert_eq!(prover_data.total_jobs_completed, 3);
    println!("\nProver completed {} jobs total", prover_data.total_jobs_completed);

    println!("\n{}", "=".repeat(80));
    println!("SEQUENTIAL JOBS TEST PASSED!");
    println!("{}", "=".repeat(80));

    Ok(())
}
