use anyhow::Result;
use borsh::BorshDeserialize;
use zyberlink_sdk::{
    CircuitType, JobAccount, JobStatus, MarketplaceClient, MarketplaceConfig, ProverAccount,
};
use solana_program_test::{processor, ProgramTest};
use solana_sdk::{
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};

fn program_id() -> solana_sdk::pubkey::Pubkey {
    solana_sdk::pubkey::Pubkey::new_unique()
}

fn setup_program_test(program_id: solana_sdk::pubkey::Pubkey) -> ProgramTest {
    ProgramTest::new(
        "zyberlink",
        program_id,
        processor!(zyberlink::process_instruction),
    )
}

/// Comprehensive end-to-end test simulating complete marketplace workflow
/// Tests: Initialize → Register Provers → Create Jobs → Claim → Submit → Verify
#[tokio::test]
async fn test_e2e_complete_marketplace_workflow() -> Result<()> {
    println!("\n=== Starting E2E Marketplace Workflow Test ===\n");

    let program_id = program_id();
    let program_test = setup_program_test(program_id);
    let (banks_client, payer, recent_blockhash) = program_test.start().await;

    let client = MarketplaceClient::new("http://localhost:8899".to_string(), program_id);

    // Create test participants
    let authority = Keypair::new();
    let prover1 = Keypair::new();
    let prover2 = Keypair::new();
    let job_creator = Keypair::new();

    // Fund all accounts
    println!("Phase 1: Funding accounts...");
    for keypair in [&authority, &prover1, &prover2, &job_creator] {
        let fund_ix =
            system_instruction::transfer(&payer.pubkey(), &keypair.pubkey(), 10_000_000_000);
        let mut tx = Transaction::new_with_payer(&[fund_ix], Some(&payer.pubkey()));
        tx.sign(&[&payer], recent_blockhash);
        banks_client.process_transaction(tx).await?;
    }
    println!("✓ All accounts funded\n");

    // ============================================================================
    // Phase 2: Initialize Marketplace
    // ============================================================================
    println!("Phase 2: Initializing marketplace...");

    let init_ix = client.initialize_instruction(
        &authority.pubkey(),
        1000,      // 10% fee
        1_000_000, // 1 SOL min stake
        100,       // min reputation
        3600,      // 1 hour default timeout
    )?;

    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[init_ix], Some(&authority.pubkey()));
    tx.sign(&[&authority], recent_blockhash);
    banks_client.process_transaction(tx).await?;

    let (config_pda, _) = client.get_config_pda();
    let config_account = banks_client.get_account(config_pda).await?.unwrap();
    let config = MarketplaceConfig::deserialize(&mut &config_account.data[..])?;

    assert_eq!(config.authority, authority.pubkey());
    assert_eq!(config.fee_basis_points, 1000);
    assert_eq!(config.total_provers, 0);
    assert_eq!(config.total_jobs_created, 0);
    println!("✓ Marketplace initialized");
    println!("  - Authority: {}", authority.pubkey());
    println!("  - Fee: 10%");
    println!("  - Min stake: 1M lamports\n");

    // ============================================================================
    // Phase 3: Register Provers
    // ============================================================================
    println!("Phase 3: Registering provers...");

    // Register prover 1
    let reg_p1_ix = client.register_prover_instruction(&prover1.pubkey(), 5_000_000, [1u8; 32])?;
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[reg_p1_ix], Some(&prover1.pubkey()));
    tx.sign(&[&prover1], recent_blockhash);
    banks_client.process_transaction(tx).await?;

    let (prover1_pda, _) = client.get_prover_pda(&prover1.pubkey());
    let p1_account = banks_client.get_account(prover1_pda).await?.unwrap();
    let p1_data = ProverAccount::deserialize(&mut &p1_account.data[..])?;
    assert_eq!(p1_data.stake_amount, 5_000_000);
    assert_eq!(p1_data.total_jobs_completed, 0);
    println!("✓ Prover 1 registered (stake: 5M lamports)");

    // Register prover 2
    let reg_p2_ix = client.register_prover_instruction(&prover2.pubkey(), 10_000_000, [2u8; 32])?;
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[reg_p2_ix], Some(&prover2.pubkey()));
    tx.sign(&[&prover2], recent_blockhash);
    banks_client.process_transaction(tx).await?;

    let (prover2_pda, _) = client.get_prover_pda(&prover2.pubkey());
    let p2_account = banks_client.get_account(prover2_pda).await?.unwrap();
    let p2_data = ProverAccount::deserialize(&mut &p2_account.data[..])?;
    assert_eq!(p2_data.stake_amount, 10_000_000);
    println!("✓ Prover 2 registered (stake: 10M lamports)\n");

    // Verify marketplace stats updated
    let config_account = banks_client.get_account(config_pda).await?.unwrap();
    let config = MarketplaceConfig::deserialize(&mut &config_account.data[..])?;
    assert_eq!(config.total_provers, 2);

    // ============================================================================
    // Phase 4: Create Jobs
    // ============================================================================
    println!("Phase 4: Creating jobs...");

    let jobs_to_create = [
        (CircuitType::ZcashOrchard, 5_000_000, "ZcashOrchard"),
        (CircuitType::AnonymousVote, 2_000_000, "AnonymousVote"),
        (CircuitType::Credential, 3_000_000, "Credential"),
    ];

    let mut job_pdas = Vec::new();

    for (idx, (circuit, price, name)) in jobs_to_create.iter().enumerate() {
        let job_id = idx as u64;
        let (job_pda, _) = client.get_job_pda(&job_creator.pubkey(), job_id);

        let create_ix = client.create_job_instruction(
            &job_creator.pubkey(),
            job_id,
            circuit.clone(),
            [idx as u8; 32],
            1024,
            *price,
            3600,
            None, // No FHE config for ZK jobs
        )?;

        let recent_blockhash = banks_client.get_latest_blockhash().await?;
        let mut tx = Transaction::new_with_payer(&[create_ix], Some(&job_creator.pubkey()));
        tx.sign(&[&job_creator], recent_blockhash);
        banks_client.process_transaction(tx).await?;

        let job_account = banks_client.get_account(job_pda).await?.unwrap();
        let job = JobAccount::deserialize(&mut &job_account.data[..])?;

        assert_eq!(job.status, JobStatus::Pending);
        assert_eq!(job.price_lamports, *price);
        assert_eq!(job.creator, job_creator.pubkey());

        job_pdas.push(job_pda);
        println!("✓ Job {} created: {} ({} lamports)", job_id, name, price);
    }
    println!();

    // Verify marketplace stats
    let config_account = banks_client.get_account(config_pda).await?.unwrap();
    let config = MarketplaceConfig::deserialize(&mut &config_account.data[..])?;
    assert_eq!(config.total_jobs_created, 3);
    assert_eq!(config.total_jobs_completed, 0);

    // ============================================================================
    // Phase 5: Process Jobs (Claim + Submit)
    // ============================================================================
    println!("Phase 5: Processing jobs...");

    // Prover 1 processes job 0
    println!("  Job 0: Prover 1 claiming...");
    let claim_ix = client.claim_job_instruction(&prover1.pubkey(), &job_pdas[0])?;
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&prover1.pubkey()));
    tx.sign(&[&prover1], recent_blockhash);
    banks_client.process_transaction(tx).await?;

    let job_account = banks_client.get_account(job_pdas[0]).await?.unwrap();
    let job = JobAccount::deserialize(&mut &job_account.data[..])?;
    assert_eq!(job.status, JobStatus::Claimed);
    assert_eq!(job.prover, Some(prover1.pubkey()));
    println!("  ✓ Job 0 claimed by Prover 1");

    println!("  Job 0: Prover 1 submitting proof...");
    let submit_ix = client.submit_proof_instruction_with_recipient(
        &prover1.pubkey(),
        &job_pdas[0],
        &job_creator.pubkey(),
        &authority.pubkey(),
        [42u8; 32],
        2048,
    )?;
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[submit_ix], Some(&prover1.pubkey()));
    tx.sign(&[&prover1], recent_blockhash);
    banks_client.process_transaction(tx).await?;

    let job_account = banks_client.get_account(job_pdas[0]).await?.unwrap();
    let job = JobAccount::deserialize(&mut &job_account.data[..])?;
    assert_eq!(job.status, JobStatus::Completed);
    println!("  ✓ Job 0 completed by Prover 1\n");

    // Prover 2 processes job 1
    println!("  Job 1: Prover 2 claiming...");
    let claim_ix = client.claim_job_instruction(&prover2.pubkey(), &job_pdas[1])?;
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&prover2.pubkey()));
    tx.sign(&[&prover2], recent_blockhash);
    banks_client.process_transaction(tx).await?;

    let submit_ix = client.submit_proof_instruction_with_recipient(
        &prover2.pubkey(),
        &job_pdas[1],
        &job_creator.pubkey(),
        &authority.pubkey(),
        [43u8; 32],
        2048,
    )?;
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[submit_ix], Some(&prover2.pubkey()));
    tx.sign(&[&prover2], recent_blockhash);
    banks_client.process_transaction(tx).await?;
    println!("  ✓ Job 1 completed by Prover 2\n");

    // Prover 1 processes job 2
    println!("  Job 2: Prover 1 claiming...");
    let claim_ix = client.claim_job_instruction(&prover1.pubkey(), &job_pdas[2])?;
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&prover1.pubkey()));
    tx.sign(&[&prover1], recent_blockhash);
    banks_client.process_transaction(tx).await?;

    let submit_ix = client.submit_proof_instruction_with_recipient(
        &prover1.pubkey(),
        &job_pdas[2],
        &job_creator.pubkey(),
        &authority.pubkey(),
        [44u8; 32],
        2048,
    )?;
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[submit_ix], Some(&prover1.pubkey()));
    tx.sign(&[&prover1], recent_blockhash);
    banks_client.process_transaction(tx).await?;
    println!("  ✓ Job 2 completed by Prover 1\n");

    // ============================================================================
    // Phase 6: Verify Final State
    // ============================================================================
    println!("Phase 6: Verifying final state...");

    // Check marketplace statistics
    let config_account = banks_client.get_account(config_pda).await?.unwrap();
    let final_config = MarketplaceConfig::deserialize(&mut &config_account.data[..])?;

    assert_eq!(final_config.total_provers, 2);
    assert_eq!(final_config.total_jobs_created, 3);
    assert_eq!(final_config.total_jobs_completed, 3);
    println!("✓ Marketplace stats verified:");
    println!("  - Total provers: {}", final_config.total_provers);
    println!(
        "  - Total jobs created: {}",
        final_config.total_jobs_created
    );
    println!(
        "  - Total jobs completed: {}",
        final_config.total_jobs_completed
    );

    // Check prover statistics
    let p1_account = banks_client.get_account(prover1_pda).await?.unwrap();
    let p1_final = ProverAccount::deserialize(&mut &p1_account.data[..])?;
    assert_eq!(p1_final.total_jobs_completed, 2);
    println!("\n✓ Prover 1 stats:");
    println!("  - Jobs completed: {}", p1_final.total_jobs_completed);

    let p2_account = banks_client.get_account(prover2_pda).await?.unwrap();
    let p2_final = ProverAccount::deserialize(&mut &p2_account.data[..])?;
    assert_eq!(p2_final.total_jobs_completed, 1);
    println!("\n✓ Prover 2 stats:");
    println!("  - Jobs completed: {}", p2_final.total_jobs_completed);

    // Verify all jobs are completed
    for (idx, job_pda) in job_pdas.iter().enumerate() {
        let job_account = banks_client.get_account(*job_pda).await?.unwrap();
        let job = JobAccount::deserialize(&mut &job_account.data[..])?;
        assert_eq!(job.status, JobStatus::Completed);
        assert!(job.proof_commitment.is_some());
        println!("\n✓ Job {} final state verified (Completed)", idx);
    }

    println!("\n=== ✅ E2E Marketplace Workflow Test PASSED ===\n");

    Ok(())
}

/// Test job cancellation workflow
#[tokio::test]
async fn test_e2e_job_cancellation() -> Result<()> {
    println!("\n=== Testing Job Cancellation ===\n");

    let program_id = program_id();
    let program_test = setup_program_test(program_id);
    let (banks_client, payer, recent_blockhash) = program_test.start().await;

    let client = MarketplaceClient::new("http://localhost:8899".to_string(), program_id);
    let authority = Keypair::new();
    let job_creator = Keypair::new();

    // Fund accounts
    for keypair in [&authority, &job_creator] {
        let fund_ix =
            system_instruction::transfer(&payer.pubkey(), &keypair.pubkey(), 10_000_000_000);
        let mut tx = Transaction::new_with_payer(&[fund_ix], Some(&payer.pubkey()));
        tx.sign(&[&payer], recent_blockhash);
        banks_client.process_transaction(tx).await?;
    }

    // Initialize
    let init_ix = client.initialize_instruction(&authority.pubkey(), 1000, 1_000_000, 100, 3600)?;
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[init_ix], Some(&authority.pubkey()));
    tx.sign(&[&authority], recent_blockhash);
    banks_client.process_transaction(tx).await?;
    println!("✓ Marketplace initialized");

    // Create job
    let (job_pda, _) = client.get_job_pda(&job_creator.pubkey(), 0);
    let create_ix = client.create_job_instruction(
        &job_creator.pubkey(),
        0,
        CircuitType::AnonymousVote,
        [1u8; 32],
        1024,
        5_000_000,
        3600,
        None, // No FHE config for ZK jobs
    )?;
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[create_ix], Some(&job_creator.pubkey()));
    tx.sign(&[&job_creator], recent_blockhash);
    banks_client.process_transaction(tx).await?;
    println!("✓ Job created");

    // Cancel job
    let cancel_ix = client.cancel_job_instruction(&job_creator.pubkey(), &job_pda)?;
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[cancel_ix], Some(&job_creator.pubkey()));
    tx.sign(&[&job_creator], recent_blockhash);
    banks_client.process_transaction(tx).await?;

    // Verify job is cancelled
    let job_account = banks_client.get_account(job_pda).await?.unwrap();
    let job = JobAccount::deserialize(&mut &job_account.data[..])?;
    assert_eq!(job.status, JobStatus::Cancelled);
    println!("✓ Job cancelled successfully");

    println!("\n=== ✅ Job Cancellation Test PASSED ===\n");

    Ok(())
}

/// Test race condition: multiple provers trying to claim same job
#[tokio::test]
async fn test_e2e_concurrent_job_claims() -> Result<()> {
    println!("\n=== Testing Concurrent Job Claims ===\n");

    let program_id = program_id();
    let program_test = setup_program_test(program_id);
    let (banks_client, payer, recent_blockhash) = program_test.start().await;

    let client = MarketplaceClient::new("http://localhost:8899".to_string(), program_id);
    let authority = Keypair::new();
    let prover1 = Keypair::new();
    let prover2 = Keypair::new();
    let job_creator = Keypair::new();

    // Fund all accounts
    for keypair in [&authority, &prover1, &prover2, &job_creator] {
        let fund_ix =
            system_instruction::transfer(&payer.pubkey(), &keypair.pubkey(), 10_000_000_000);
        let mut tx = Transaction::new_with_payer(&[fund_ix], Some(&payer.pubkey()));
        tx.sign(&[&payer], recent_blockhash);
        banks_client.process_transaction(tx).await?;
    }

    // Initialize marketplace
    let init_ix = client.initialize_instruction(&authority.pubkey(), 1000, 1_000_000, 100, 3600)?;
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[init_ix], Some(&authority.pubkey()));
    tx.sign(&[&authority], recent_blockhash);
    banks_client.process_transaction(tx).await?;

    // Register both provers
    for (i, prover) in [&prover1, &prover2].iter().enumerate() {
        let enc_key = [(i + 1) as u8; 32];
        let reg_ix = client.register_prover_instruction(&prover.pubkey(), 5_000_000, enc_key)?;
        let recent_blockhash = banks_client.get_latest_blockhash().await?;
        let mut tx = Transaction::new_with_payer(&[reg_ix], Some(&prover.pubkey()));
        tx.sign(&[prover], recent_blockhash);
        banks_client.process_transaction(tx).await?;
    }
    println!("✓ Both provers registered");

    // Create job
    let (job_pda, _) = client.get_job_pda(&job_creator.pubkey(), 0);
    let create_ix = client.create_job_instruction(
        &job_creator.pubkey(),
        0,
        CircuitType::ZcashOrchard,
        [1u8; 32],
        1024,
        2_000_000,
        3600,
        None, // No FHE config for ZK jobs
    )?;
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[create_ix], Some(&job_creator.pubkey()));
    tx.sign(&[&job_creator], recent_blockhash);
    banks_client.process_transaction(tx).await?;
    println!("✓ Job created");

    // Prover 1 claims the job
    let claim_ix = client.claim_job_instruction(&prover1.pubkey(), &job_pda)?;
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&prover1.pubkey()));
    tx.sign(&[&prover1], recent_blockhash);
    banks_client.process_transaction(tx).await?;

    let job_account = banks_client.get_account(job_pda).await?.unwrap();
    let job = JobAccount::deserialize(&mut &job_account.data[..])?;
    assert_eq!(job.status, JobStatus::Claimed);
    assert_eq!(job.prover, Some(prover1.pubkey()));
    println!("✓ Prover 1 successfully claimed job");

    // Prover 2 tries to claim the same job (should fail)
    let claim_ix2 = client.claim_job_instruction(&prover2.pubkey(), &job_pda)?;
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx2 = Transaction::new_with_payer(&[claim_ix2], Some(&prover2.pubkey()));
    tx2.sign(&[&prover2], recent_blockhash);

    let result = banks_client.process_transaction(tx2).await;
    assert!(result.is_err(), "Second claim should fail");
    println!("✓ Prover 2 correctly prevented from claiming already-claimed job");

    // Verify job still belongs to prover 1
    let job_account = banks_client.get_account(job_pda).await?.unwrap();
    let job_final = JobAccount::deserialize(&mut &job_account.data[..])?;
    assert_eq!(job_final.prover, Some(prover1.pubkey()));

    println!("\n=== ✅ Concurrent Claims Test PASSED ===\n");

    Ok(())
}
