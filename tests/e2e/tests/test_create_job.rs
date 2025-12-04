/// Test CreateJob instruction using solana-program-test (in-memory)
///
/// Tests job creation functionality including:
/// - Basic ZK job creation
/// - FHE job creation with consensus config
/// - Escrow account verification
/// - Job parameter validation

mod common;

use anyhow::Result;
use blake2::{Blake2s256, Digest};
use zyberlink_types::{CircuitType, FheConsensusConfig, FheOperation};
use solana_sdk::signature::{Keypair, Signer};
use borsh::BorshDeserialize;
use common::setup_initialized_marketplace;

// Circuit type constants (matches JobAccount in program)
const CIRCUIT_ZCASH_ORCHARD: u8 = 0;
const CIRCUIT_FHE_ADD: u8 = 4;

#[tokio::test]
async fn test_create_zk_job() -> Result<()> {
    println!("\n=== Testing Create ZK Job ===\n");

    let mut ctx = setup_initialized_marketplace().await?;

    let job_creator = Keypair::new();
    ctx.fund_account(&job_creator.pubkey(), 10_000_000_000).await?;
    println!("Job Creator: {}", job_creator.pubkey());

    // Get next job ID from config
    let config_account = ctx.banks_client
        .get_account(ctx.config_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Config account not found"))?;

    let config: zyberlink_sdk::MarketplaceConfig =
        borsh::from_slice(&config_account.data)?;
    let job_id = config.next_job_id;
    println!("Job ID: {}", job_id);

    // Create witness commitment
    let witness_data = vec![42u8; 1024];
    let mut hasher = Blake2s256::new();
    hasher.update(&witness_data);
    let witness_commitment: [u8; 32] = hasher.finalize().into();

    // Job parameters
    let circuit_type = CircuitType::ZcashOrchard;
    let witness_size = 2048u32;
    let price_lamports = 2_000_000_000u64; // 2 SOL
    let timeout_seconds = 600i64;

    println!("Creating job:");
    println!("   Circuit Type: {:?}", circuit_type);
    println!("   Price: {} SOL", price_lamports as f64 / 1e9);
    println!("   Timeout: {} seconds", timeout_seconds);

    // Create job instruction
    let sdk = ctx.sdk_client();
    let create_job_ix = sdk.create_job_instruction(
        &job_creator.pubkey(),
        job_id,
        circuit_type,
        witness_commitment,
        witness_size,
        price_lamports,
        timeout_seconds,
        None, // No FHE config for ZK job
    )?;

    ctx.execute_transaction(&[create_job_ix], &[&job_creator]).await?;
    println!("Job created successfully!");

    // Verify job account
    let (job_pda, _) = sdk.get_job_pda(&job_creator.pubkey(), job_id);
    let job_account = ctx.banks_client
        .get_account(job_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Job account not found"))?;

    println!("   Job PDA: {}", job_pda);
    println!("   Job data length: {} bytes", job_account.data.len());

    // Deserialize job
    let mut data_slice = job_account.data.as_slice();
    let job = zyberlink_sdk::JobAccount::deserialize(&mut data_slice)?;

    assert_eq!(job.id, job_id, "Job ID should match");
    assert_eq!(job.creator, job_creator.pubkey(), "Creator should match");
    assert_eq!(job.circuit_type, CIRCUIT_ZCASH_ORCHARD, "Circuit type should match");
    assert_eq!(job.price_lamports, price_lamports, "Price should match");
    assert_eq!(job.status, zyberlink_types::JobStatus::Pending, "Status should be Pending");
    assert!(job.prover.is_none(), "Prover should be None for new job");

    println!("   Status: {:?}", job.status);
    println!("   Circuit Type: {}", job.circuit_type);

    // Verify escrow account
    let (escrow_pda, _) = sdk.get_escrow_pda(&job_pda);
    let escrow_account = ctx.banks_client
        .get_account(escrow_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Escrow account not found"))?;

    println!("   Escrow PDA: {}", escrow_pda);
    println!("   Escrow Balance: {} SOL", escrow_account.lamports as f64 / 1e9);

    assert!(escrow_account.lamports >= price_lamports, "Escrow should hold at least job price");

    println!("\nCreate ZK job test passed!");
    Ok(())
}

#[tokio::test]
async fn test_create_fhe_job() -> Result<()> {
    println!("\n=== Testing Create FHE Job ===\n");

    let mut ctx = setup_initialized_marketplace().await?;

    let job_creator = Keypair::new();
    ctx.fund_account(&job_creator.pubkey(), 20_000_000_000).await?;
    println!("Job Creator: {}", job_creator.pubkey());

    // Get next job ID
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

    // FHE job parameters
    let circuit_type = CircuitType::FheComputation(FheOperation::Add(10)); // FHE Add operation
    let witness_size = 4096u32;
    let price_lamports = 5_000_000_000u64; // 5 SOL (FHE jobs cost more)
    let timeout_seconds = 1200i64; // 20 minutes

    // FHE consensus config
    let fhe_config = FheConsensusConfig {
        required_provers: 3,
        consensus_threshold: 2, // 2 of 3 must agree
        submission_timeout_secs: 600,
        operation: FheOperation::Add(10),
    };

    println!("Creating FHE job:");
    println!("   Circuit Type: {:?}", circuit_type);
    println!("   Price: {} SOL", price_lamports as f64 / 1e9);
    println!("   Required Provers: {}", fhe_config.required_provers);
    println!("   Consensus Threshold: {}", fhe_config.consensus_threshold);

    // Create job instruction
    let sdk = ctx.sdk_client();
    let create_job_ix = sdk.create_job_instruction(
        &job_creator.pubkey(),
        job_id,
        circuit_type,
        witness_commitment,
        witness_size,
        price_lamports,
        timeout_seconds,
        Some(fhe_config),
    )?;

    ctx.execute_transaction(&[create_job_ix], &[&job_creator]).await?;
    println!("FHE job created successfully!");

    // Verify job account
    let (job_pda, _) = sdk.get_job_pda(&job_creator.pubkey(), job_id);
    let job_account = ctx.banks_client
        .get_account(job_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Job account not found"))?;

    let mut data_slice = job_account.data.as_slice();
    let job = zyberlink_sdk::JobAccount::deserialize(&mut data_slice)?;

    assert_eq!(job.circuit_type, CIRCUIT_FHE_ADD, "Should be FHE circuit type");
    assert!(job.fhe_consensus_bump.is_some(), "FHE job should have consensus bump");

    // Verify FHE consensus account was created
    let (fhe_consensus_pda, _) = sdk.get_fhe_consensus_pda(job_id);
    let fhe_consensus_account = ctx.banks_client
        .get_account(fhe_consensus_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("FHE consensus account not found"))?;

    println!("   FHE Consensus PDA: {}", fhe_consensus_pda);
    println!("   FHE Consensus data length: {} bytes", fhe_consensus_account.data.len());

    // Deserialize FHE consensus data
    let mut consensus_slice = fhe_consensus_account.data.as_slice();
    let consensus = zyberlink_sdk::FheConsensusData::deserialize(&mut consensus_slice)?;

    assert_eq!(consensus.required_provers, 3, "Required provers should match");
    assert_eq!(consensus.consensus_threshold, 2, "Threshold should match");
    assert_eq!(consensus.results_count, 0, "No results submitted yet");

    println!("\nCreate FHE job test passed!");
    Ok(())
}

#[tokio::test]
async fn test_create_job_insufficient_funds() -> Result<()> {
    println!("\n=== Testing Create Job - Insufficient Funds ===\n");

    let mut ctx = setup_initialized_marketplace().await?;

    let job_creator = Keypair::new();
    // Only fund with 1 SOL (job costs 2 SOL)
    ctx.fund_account(&job_creator.pubkey(), 1_000_000_000).await?;

    // Get next job ID
    let config_account = ctx.banks_client
        .get_account(ctx.config_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Config account not found"))?;

    let config: zyberlink_sdk::MarketplaceConfig =
        borsh::from_slice(&config_account.data)?;
    let job_id = config.next_job_id;

    let witness_commitment = [42u8; 32];
    let price_lamports = 2_000_000_000u64; // 2 SOL (more than creator has)

    let sdk = ctx.sdk_client();
    let create_job_ix = sdk.create_job_instruction(
        &job_creator.pubkey(),
        job_id,
        CircuitType::ZcashOrchard,
        witness_commitment,
        2048,
        price_lamports,
        600,
        None,
    )?;

    let result = ctx.execute_transaction(&[create_job_ix], &[&job_creator]).await;

    assert!(result.is_err(), "Should fail with insufficient funds");
    println!("Job creation correctly rejected for insufficient funds");

    println!("\nInsufficient funds test passed!");
    Ok(())
}

#[tokio::test]
async fn test_create_multiple_jobs() -> Result<()> {
    println!("\n=== Testing Create Multiple Jobs ===\n");

    let mut ctx = setup_initialized_marketplace().await?;

    let job_creator = Keypair::new();
    ctx.fund_account(&job_creator.pubkey(), 50_000_000_000).await?;

    let sdk = ctx.sdk_client();

    // Create 3 jobs
    for i in 0..3 {
        let config_account = ctx.banks_client
            .get_account(ctx.config_pda)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Config account not found"))?;

        let config: zyberlink_sdk::MarketplaceConfig =
            borsh::from_slice(&config_account.data)?;
        let job_id = config.next_job_id;

        let witness_commitment = [i as u8; 32];
        let price_lamports = 1_000_000_000u64; // 1 SOL each

        let create_job_ix = sdk.create_job_instruction(
            &job_creator.pubkey(),
            job_id,
            CircuitType::ZcashOrchard,
            witness_commitment,
            2048,
            price_lamports,
            600,
            None,
        )?;

        ctx.execute_transaction(&[create_job_ix], &[&job_creator]).await?;
        println!("Created job {} with ID {}", i + 1, job_id);

        // Verify job exists
        let (job_pda, _) = sdk.get_job_pda(&job_creator.pubkey(), job_id);
        let job_account = ctx.banks_client.get_account(job_pda).await?;
        assert!(job_account.is_some(), "Job {} should exist", job_id);
    }

    // Verify config updated
    let config_account = ctx.banks_client
        .get_account(ctx.config_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Config account not found"))?;

    let config: zyberlink_sdk::MarketplaceConfig =
        borsh::from_slice(&config_account.data)?;

    assert_eq!(config.total_jobs_created, 3, "Should have created 3 jobs");
    println!("Total jobs created: {}", config.total_jobs_created);

    println!("\nMultiple jobs test passed!");
    Ok(())
}
