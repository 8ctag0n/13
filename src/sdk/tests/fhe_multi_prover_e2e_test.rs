/// FHE Multi-Prover E2E Test Suite
///
/// Comprehensive tests validating the complete lifecycle of FHE jobs with
/// multi-prover consensus mechanism including:
/// - Multi-prover claiming
/// - FHE result submission
/// - Consensus verification
/// - Payment distribution
/// - Edge cases and failure scenarios
///
/// NOTE: These tests use the new two-account model:
/// - JobAccount: Fixed 203 bytes, stores job metadata
/// - FheConsensusData: Fixed 384 bytes, stores multi-prover consensus state
use anyhow::Result;
use borsh::BorshDeserialize;
use zyberlink_sdk::{FheConsensusData, JobAccount, MarketplaceClient, ProverAccount};
use zyberlink_types::{FheConsensusConfig, FheOperation, JobStatus as TypesJobStatus};
use solana_program_test::{processor, BanksClient, ProgramTest};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};

// ============================================================================
// Test Helpers
// ============================================================================

/// Generate unique program ID for isolated testing
fn program_id() -> Pubkey {
    Pubkey::new_unique()
}

/// Setup program test environment with zyberlink processor
fn setup_program_test(program_id: Pubkey) -> ProgramTest {
    ProgramTest::new(
        "zyberlink",
        program_id,
        processor!(zyberlink::process_instruction),
    )
}

/// Create fake FHE result hash (for testing consensus without real FHE computation)
fn create_fhe_result_hash(value: u8) -> [u8; 32] {
    let mut hash = [0u8; 32];
    hash[0] = value;
    hash
}

/// Get SOL balance of an account
async fn get_account_balance(banks_client: &mut BanksClient, pubkey: &Pubkey) -> u64 {
    banks_client
        .get_account(*pubkey)
        .await
        .unwrap()
        .map(|acc| acc.lamports)
        .unwrap_or(0)
}

/// Fund an account with SOL
async fn fund_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    target: &Pubkey,
    amount: u64,
    recent_blockhash: solana_sdk::hash::Hash,
) -> Result<()> {
    let fund_ix = system_instruction::transfer(&payer.pubkey(), target, amount);
    let mut tx = Transaction::new_with_payer(&[fund_ix], Some(&payer.pubkey()));
    tx.sign(&[payer], recent_blockhash);
    banks_client.process_transaction(tx).await?;
    Ok(())
}

/// Fetch FheConsensusData from the chain
async fn fetch_fhe_consensus(
    banks_client: &mut BanksClient,
    fhe_consensus_pda: &Pubkey,
) -> Result<FheConsensusData> {
    let account = banks_client
        .get_account(*fhe_consensus_pda)
        .await?
        .ok_or_else(|| anyhow::anyhow!("FheConsensusData account not found"))?;
    let data = FheConsensusData::deserialize(&mut &account.data[..])?;
    Ok(data)
}

// ============================================================================
// TEST 1: Complete FHE Multi-Prover Flow (Happy Path)
// ============================================================================

#[tokio::test]
async fn test_fhe_multi_prover_complete_flow() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST 1: FHE Multi-Prover Complete Flow (Happy Path)");
    println!("{}", "=".repeat(80));

    let program_id = program_id();
    let program_test = setup_program_test(program_id);
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let client = MarketplaceClient::new("http://localhost:8899".to_string(), program_id);

    // Create participants
    let authority = Keypair::new();
    let prover1 = Keypair::new();
    let prover2 = Keypair::new();
    let prover3 = Keypair::new();
    let job_creator = Keypair::new();

    println!("\n1. Funding accounts...");
    for keypair in [&authority, &prover1, &prover2, &prover3, &job_creator] {
        fund_account(
            &mut banks_client,
            &payer,
            &keypair.pubkey(),
            10_000_000_000,
            recent_blockhash,
        )
        .await?;
    }
    println!("   All accounts funded with 10 SOL");

    // Step 1: Initialize Marketplace
    println!("\n2. Initializing marketplace...");
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
    println!("   Marketplace initialized (10% fee)");

    // Step 2: Register 3 Provers
    println!("\n3. Registering 3 provers...");

    for (i, prover) in [&prover1, &prover2, &prover3].iter().enumerate() {
        let enc_key = [(i + 1) as u8; 32];
        let reg_ix = client.register_prover_instruction(&prover.pubkey(), 5_000_000, enc_key)?;

        let recent_blockhash = banks_client.get_latest_blockhash().await?;
        let mut tx = Transaction::new_with_payer(&[reg_ix], Some(&prover.pubkey()));
        tx.sign(&[prover], recent_blockhash);
        banks_client.process_transaction(tx).await?;

        // Verify prover account exists
        let (prover_pda, _) = client.get_prover_pda(&prover.pubkey());
        let account = banks_client.get_account(prover_pda).await?.unwrap();
        let prover_data = ProverAccount::deserialize(&mut &account.data[..])?;
        assert_eq!(prover_data.stake_amount, 5_000_000);

        println!("   Prover {} registered (stake: 5M lamports)", i + 1);
    }

    // Step 3: Create FHE Job
    println!("\n4. Creating FHE job...");
    let job_id = 0u64;
    let (job_pda, _) = client.get_job_pda(&job_creator.pubkey(), job_id);
    let (fhe_consensus_pda, _) = client.get_fhe_consensus_pda(job_id);

    let fhe_config = FheConsensusConfig {
        required_provers: 3,
        consensus_threshold: 2,
        submission_timeout_secs: 300,
        operation: FheOperation::Add(10),
    };

    let create_ix = client.create_job_instruction(
        &job_creator.pubkey(),
        job_id,
        zyberlink_sdk::CircuitType::FheComputation(FheOperation::Add(10)),
        [42u8; 32], // witness commitment
        1024,       // witness size
        3_000_000,  // price: enough for 3 provers
        3600,       // timeout
        Some(fhe_config),
    )?;

    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[create_ix], Some(&job_creator.pubkey()));
    tx.sign(&[&job_creator], recent_blockhash);
    banks_client.process_transaction(tx).await?;

    // Verify job created in Pending state
    let job_account = banks_client.get_account(job_pda).await?.unwrap();
    let job = JobAccount::deserialize(&mut &job_account.data[..])?;
    assert_eq!(job.status, TypesJobStatus::Pending);
    assert_eq!(job.price_lamports, 3_000_000);
    assert!(job.fhe_consensus_bump.is_some(), "FHE job should have consensus bump");

    // Verify FheConsensusData created
    let fhe_data = fetch_fhe_consensus(&mut banks_client, &fhe_consensus_pda).await?;
    assert_eq!(fhe_data.job_id, job_id);
    assert_eq!(fhe_data.required_provers, 3);
    assert_eq!(fhe_data.consensus_threshold, 2);
    println!("   FHE job created (3 provers, 2/3 consensus, 3M lamports)");

    // Step 4: Multi-Prover Claim (using claim_fhe_job_instruction)
    println!("\n5. Provers claiming job...");

    for (i, prover) in [&prover1, &prover2, &prover3].iter().enumerate() {
        let claim_ix = client.claim_fhe_job_instruction(&prover.pubkey(), &job_pda, job_id)?;

        let recent_blockhash = banks_client.get_latest_blockhash().await?;
        let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&prover.pubkey()));
        tx.sign(&[prover], recent_blockhash);
        banks_client.process_transaction(tx).await?;

        println!("   Prover {} claimed job", i + 1);
    }

    // Verify all provers claimed in FheConsensusData
    let fhe_data = fetch_fhe_consensus(&mut banks_client, &fhe_consensus_pda).await?;
    assert_eq!(fhe_data.claimed_count, 3);
    assert!(fhe_data.claimed_provers[0..3].contains(&prover1.pubkey()));
    assert!(fhe_data.claimed_provers[0..3].contains(&prover2.pubkey()));
    assert!(fhe_data.claimed_provers[0..3].contains(&prover3.pubkey()));

    // Verify job status is Claimed
    let job_account = banks_client.get_account(job_pda).await?.unwrap();
    let job = JobAccount::deserialize(&mut &job_account.data[..])?;
    assert_eq!(job.status, TypesJobStatus::Claimed);
    println!("   All 3 provers successfully claimed");

    // Step 5: Submit FHE Results (with consensus)
    println!("\n6. Submitting FHE results...");

    // Prover 1 and 2 submit SAME hash (consensus)
    let consensus_hash = create_fhe_result_hash(100);

    for (i, prover) in [&prover1, &prover2].iter().enumerate() {
        let submit_ix =
            client.submit_fhe_result_instruction(&prover.pubkey(), &job_pda, job_id, consensus_hash)?;

        let recent_blockhash = banks_client.get_latest_blockhash().await?;
        let mut tx = Transaction::new_with_payer(&[submit_ix], Some(&prover.pubkey()));
        tx.sign(&[prover], recent_blockhash);
        banks_client.process_transaction(tx).await?;

        println!(
            "   Prover {} submitted result (hash: {:02x}...)",
            i + 1,
            consensus_hash[0]
        );
    }

    // Prover 3 submits DIFFERENT hash (outlier)
    let outlier_hash = create_fhe_result_hash(200);
    let submit_ix =
        client.submit_fhe_result_instruction(&prover3.pubkey(), &job_pda, job_id, outlier_hash)?;

    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[submit_ix], Some(&prover3.pubkey()));
    tx.sign(&[&prover3], recent_blockhash);
    banks_client.process_transaction(tx).await?;
    println!(
        "   Prover 3 submitted result (hash: {:02x}...) - DIFFERENT",
        outlier_hash[0]
    );

    // Verify all results submitted in FheConsensusData
    let fhe_data = fetch_fhe_consensus(&mut banks_client, &fhe_consensus_pda).await?;
    assert_eq!(fhe_data.results_count, 3);

    // Verify consensus pattern: 2 matching, 1 different
    let matching_count = fhe_data
        .result_hashes[0..fhe_data.results_count as usize]
        .iter()
        .filter(|r| **r == consensus_hash)
        .count();
    assert_eq!(matching_count, 2);
    println!("   Results verified: 2 matching, 1 outlier");

    // Step 6: Record balances before finalization
    println!("\n7. Recording balances before finalization...");
    let prover1_balance_before = get_account_balance(&mut banks_client, &prover1.pubkey()).await;
    let prover2_balance_before = get_account_balance(&mut banks_client, &prover2.pubkey()).await;
    let prover3_balance_before = get_account_balance(&mut banks_client, &prover3.pubkey()).await;
    let escrow_balance_before = get_account_balance(&mut banks_client, &job.escrow_account).await;

    println!("   Prover 1 balance: {} lamports", prover1_balance_before);
    println!("   Prover 2 balance: {} lamports", prover2_balance_before);
    println!("   Prover 3 balance: {} lamports", prover3_balance_before);
    println!("   Escrow balance: {} lamports", escrow_balance_before);

    // Step 7: Finalize Job
    // NOTE: Must pass ALL provers in the order they claimed (not just matching ones)
    // The program will verify prover order matches claimed_provers in FheConsensusData
    println!("\n8. Finalizing FHE job...");
    let finalize_ix = client.finalize_fhe_job_instruction_with_recipient(
        &authority.pubkey(), // finalizer (can be anyone)
        &job_pda,
        job_id,
        &job_creator.pubkey(),
        &authority.pubkey(),                   // protocol fee recipient
        &[prover1.pubkey(), prover2.pubkey(), prover3.pubkey()], // ALL provers in claim order
    )?;

    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[finalize_ix], Some(&authority.pubkey()));
    tx.sign(&[&authority], recent_blockhash);
    banks_client.process_transaction(tx).await?;

    // Verify job completed
    let job_account = banks_client.get_account(job_pda).await?.unwrap();
    let job = JobAccount::deserialize(&mut &job_account.data[..])?;
    assert_eq!(job.status, TypesJobStatus::Completed);

    // Verify consensus hash in FheConsensusData
    let fhe_data = fetch_fhe_consensus(&mut banks_client, &fhe_consensus_pda).await?;
    assert_eq!(fhe_data.consensus_hash, Some(consensus_hash));
    println!("   Job finalized successfully!");
    println!("   Consensus hash: {:02x}...", consensus_hash[0]);

    // Step 8: Verify Payouts
    println!("\n9. Verifying payouts...");

    let prover1_balance_after = get_account_balance(&mut banks_client, &prover1.pubkey()).await;
    let prover2_balance_after = get_account_balance(&mut banks_client, &prover2.pubkey()).await;
    let prover3_balance_after = get_account_balance(&mut banks_client, &prover3.pubkey()).await;
    let escrow_balance_after = get_account_balance(&mut banks_client, &job.escrow_account).await;

    // Calculate expected payout (3M lamports - 10% fee = 2.7M, split between 2 provers)
    let _platform_fee = 300_000; // 10% of 3M
    let _total_prover_payout = 3_000_000 - _platform_fee;
    let _payout_per_prover = _total_prover_payout / 2; // Split among 2 matching provers

    // Verify provers 1 and 2 got paid
    let prover1_gain = prover1_balance_after.saturating_sub(prover1_balance_before);
    let prover2_gain = prover2_balance_after.saturating_sub(prover2_balance_before);
    let prover3_gain = prover3_balance_after.saturating_sub(prover3_balance_before);

    println!("   Prover 1 gained: {} lamports", prover1_gain);
    println!("   Prover 2 gained: {} lamports", prover2_gain);
    println!(
        "   Prover 3 gained: {} lamports (should be 0)",
        prover3_gain
    );

    assert!(prover1_gain > 0, "Prover 1 should receive payment");
    assert!(prover2_gain > 0, "Prover 2 should receive payment");
    assert_eq!(
        prover3_gain, 0,
        "Prover 3 (outlier) should NOT receive payment"
    );

    // Verify escrow is nearly drained (rent-exempt minimum may remain)
    assert!(
        escrow_balance_after < 1_000_000,
        "Escrow should be mostly drained (only rent may remain)"
    );
    println!(
        "   Escrow drained: {} lamports remaining (rent-exempt)",
        escrow_balance_after
    );

    println!("\n{}", "=".repeat(80));
    println!("TEST 1 PASSED!");
    println!("{}", "=".repeat(80));

    Ok(())
}

// ============================================================================
// TEST 2: Consensus Threshold Not Met
// ============================================================================

#[tokio::test]
async fn test_fhe_consensus_threshold_not_met() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST 2: Consensus Threshold Not Met");
    println!("{}", "=".repeat(80));

    let program_id = program_id();
    let program_test = setup_program_test(program_id);
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let client = MarketplaceClient::new("http://localhost:8899".to_string(), program_id);

    let authority = Keypair::new();
    let prover1 = Keypair::new();
    let prover2 = Keypair::new();
    let prover3 = Keypair::new();
    let job_creator = Keypair::new();

    println!("\n1. Setting up marketplace and provers...");

    // Fund accounts
    for keypair in [&authority, &prover1, &prover2, &prover3, &job_creator] {
        fund_account(
            &mut banks_client,
            &payer,
            &keypair.pubkey(),
            10_000_000_000,
            recent_blockhash,
        )
        .await?;
    }

    // Initialize marketplace
    let init_ix = client.initialize_instruction(&authority.pubkey(), 1000, 1_000_000, 100, 3600)?;
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[init_ix], Some(&authority.pubkey()));
    tx.sign(&[&authority], recent_blockhash);
    banks_client.process_transaction(tx).await?;

    // Register 3 provers
    for (i, prover) in [&prover1, &prover2, &prover3].iter().enumerate() {
        let reg_ix =
            client.register_prover_instruction(&prover.pubkey(), 5_000_000, [(i + 1) as u8; 32])?;
        let recent_blockhash = banks_client.get_latest_blockhash().await?;
        let mut tx = Transaction::new_with_payer(&[reg_ix], Some(&prover.pubkey()));
        tx.sign(&[prover], recent_blockhash);
        banks_client.process_transaction(tx).await?;
    }
    println!("   Setup complete");

    // Step 2: Create FHE job with 100% consensus requirement
    println!("\n2. Creating FHE job with consensus_threshold: 3 (100% agreement)...");
    let job_id = 0u64;
    let (job_pda, _) = client.get_job_pda(&job_creator.pubkey(), job_id);
    let (fhe_consensus_pda, _) = client.get_fhe_consensus_pda(job_id);

    let fhe_config = FheConsensusConfig {
        required_provers: 3,
        consensus_threshold: 3, // All 3 must agree!
        submission_timeout_secs: 300,
        operation: FheOperation::Add(10),
    };

    let create_ix = client.create_job_instruction(
        &job_creator.pubkey(),
        job_id,
        zyberlink_sdk::CircuitType::FheComputation(FheOperation::Add(10)),
        [42u8; 32],
        1024,
        3_000_000,
        3600,
        Some(fhe_config),
    )?;

    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[create_ix], Some(&job_creator.pubkey()));
    tx.sign(&[&job_creator], recent_blockhash);
    banks_client.process_transaction(tx).await?;
    println!("   Job created with strict consensus");

    // Step 3: Provers claim (using FHE-specific claim)
    println!("\n3. Provers claiming job...");
    for prover in [&prover1, &prover2, &prover3] {
        let claim_ix = client.claim_fhe_job_instruction(&prover.pubkey(), &job_pda, job_id)?;
        let recent_blockhash = banks_client.get_latest_blockhash().await?;
        let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&prover.pubkey()));
        tx.sign(&[prover], recent_blockhash);
        banks_client.process_transaction(tx).await?;
    }
    println!("   All provers claimed");

    // Step 4: Submit DIFFERENT results (no consensus)
    println!("\n4. Submitting different results (no consensus)...");

    for (i, prover) in [&prover1, &prover2, &prover3].iter().enumerate() {
        let different_hash = create_fhe_result_hash((i + 1) as u8 * 10);
        let submit_ix =
            client.submit_fhe_result_instruction(&prover.pubkey(), &job_pda, job_id, different_hash)?;

        let recent_blockhash = banks_client.get_latest_blockhash().await?;
        let mut tx = Transaction::new_with_payer(&[submit_ix], Some(&prover.pubkey()));
        tx.sign(&[prover], recent_blockhash);
        banks_client.process_transaction(tx).await?;

        println!(
            "   Prover {} submitted hash: {:02x}...",
            i + 1,
            different_hash[0]
        );
    }

    // Verify results in FheConsensusData
    let fhe_data = fetch_fhe_consensus(&mut banks_client, &fhe_consensus_pda).await?;
    assert_eq!(fhe_data.results_count, 3);

    // Step 5: Record creator balance before finalize
    let creator_balance_before =
        get_account_balance(&mut banks_client, &job_creator.pubkey()).await;

    // Step 6: Attempt to finalize (should fail or mark as failed)
    // NOTE: Must pass ALL provers in the order they claimed (not just matching ones)
    println!("\n5. Attempting to finalize (consensus should fail)...");
    let finalize_ix = client.finalize_fhe_job_instruction_with_recipient(
        &authority.pubkey(),
        &job_pda,
        job_id,
        &job_creator.pubkey(),
        &authority.pubkey(),
        &[prover1.pubkey(), prover2.pubkey(), prover3.pubkey()], // ALL provers in claim order
    )?;

    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[finalize_ix], Some(&authority.pubkey()));
    tx.sign(&[&authority], recent_blockhash);

    let result = banks_client.process_transaction(tx).await;

    // Finalize should succeed but mark job as Failed
    assert!(
        result.is_ok(),
        "Finalize should process but mark job as failed"
    );

    // Verify job status
    let job_account = banks_client.get_account(job_pda).await?.unwrap();
    let job = JobAccount::deserialize(&mut &job_account.data[..])?;
    assert_eq!(
        job.status,
        TypesJobStatus::Failed,
        "Job should be marked as Failed"
    );

    // Verify no consensus hash in FheConsensusData
    let fhe_data = fetch_fhe_consensus(&mut banks_client, &fhe_consensus_pda).await?;
    assert!(
        fhe_data.consensus_hash.is_none(),
        "No consensus hash should be set"
    );
    println!("   Job correctly marked as Failed (no consensus)");

    // Verify creator got refund
    let creator_balance_after = get_account_balance(&mut banks_client, &job_creator.pubkey()).await;
    let creator_gain = creator_balance_after.saturating_sub(creator_balance_before);
    assert!(creator_gain > 0, "Creator should receive refund");
    println!("   Creator refunded: {} lamports", creator_gain);

    println!("\n{}", "=".repeat(80));
    println!("TEST 2 PASSED!");
    println!("{}", "=".repeat(80));

    Ok(())
}

// ============================================================================
// TEST 3: Insufficient Provers
// ============================================================================

#[tokio::test]
async fn test_fhe_insufficient_provers() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST 3: Insufficient Provers");
    println!("{}", "=".repeat(80));

    let program_id = program_id();
    let program_test = setup_program_test(program_id);
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let client = MarketplaceClient::new("http://localhost:8899".to_string(), program_id);

    let authority = Keypair::new();
    let prover1 = Keypair::new();
    let prover2 = Keypair::new();
    let prover3 = Keypair::new();
    let job_creator = Keypair::new();

    println!("\n1. Setting up marketplace and provers...");

    // Fund accounts
    for keypair in [&authority, &prover1, &prover2, &prover3, &job_creator] {
        fund_account(
            &mut banks_client,
            &payer,
            &keypair.pubkey(),
            10_000_000_000,
            recent_blockhash,
        )
        .await?;
    }

    // Initialize marketplace
    let init_ix = client.initialize_instruction(&authority.pubkey(), 1000, 1_000_000, 100, 3600)?;
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[init_ix], Some(&authority.pubkey()));
    tx.sign(&[&authority], recent_blockhash);
    banks_client.process_transaction(tx).await?;

    // Register 3 provers
    for (i, prover) in [&prover1, &prover2, &prover3].iter().enumerate() {
        let reg_ix =
            client.register_prover_instruction(&prover.pubkey(), 5_000_000, [(i + 1) as u8; 32])?;
        let recent_blockhash = banks_client.get_latest_blockhash().await?;
        let mut tx = Transaction::new_with_payer(&[reg_ix], Some(&prover.pubkey()));
        tx.sign(&[prover], recent_blockhash);
        banks_client.process_transaction(tx).await?;
    }
    println!("   Setup complete");

    // Step 2: Create FHE job requiring 3 provers
    println!("\n2. Creating FHE job requiring 3 provers...");
    let job_id = 0u64;
    let (job_pda, _) = client.get_job_pda(&job_creator.pubkey(), job_id);
    let (fhe_consensus_pda, _) = client.get_fhe_consensus_pda(job_id);

    let fhe_config = FheConsensusConfig {
        required_provers: 3,
        consensus_threshold: 2,
        submission_timeout_secs: 300,
        operation: FheOperation::Add(10),
    };

    let create_ix = client.create_job_instruction(
        &job_creator.pubkey(),
        job_id,
        zyberlink_sdk::CircuitType::FheComputation(FheOperation::Add(10)),
        [42u8; 32],
        1024,
        3_000_000,
        3600,
        Some(fhe_config),
    )?;

    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[create_ix], Some(&job_creator.pubkey()));
    tx.sign(&[&job_creator], recent_blockhash);
    banks_client.process_transaction(tx).await?;
    println!("   Job created");

    // Step 3: Only 2 provers claim (insufficient)
    println!("\n3. Only 2 provers claiming (insufficient)...");
    for prover in [&prover1, &prover2] {
        let claim_ix = client.claim_fhe_job_instruction(&prover.pubkey(), &job_pda, job_id)?;
        let recent_blockhash = banks_client.get_latest_blockhash().await?;
        let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&prover.pubkey()));
        tx.sign(&[prover], recent_blockhash);
        banks_client.process_transaction(tx).await?;
    }

    // Verify job still in Pending (not fully claimed - needs all 3)
    let job_account = banks_client.get_account(job_pda).await?.unwrap();
    let job = JobAccount::deserialize(&mut &job_account.data[..])?;
    assert_eq!(
        job.status,
        TypesJobStatus::Pending,
        "Job should remain Pending until all provers claim"
    );

    // Verify FheConsensusData has only 2 claimed
    let fhe_data = fetch_fhe_consensus(&mut banks_client, &fhe_consensus_pda).await?;
    assert_eq!(fhe_data.claimed_count, 2);
    println!("   Only 2 provers claimed (job still Pending)");

    // Step 4: Have 3rd prover claim to transition to Claimed
    println!("\n4. Third prover claiming to complete claims...");
    let claim_ix = client.claim_fhe_job_instruction(&prover3.pubkey(), &job_pda, job_id)?;
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&prover3.pubkey()));
    tx.sign(&[&prover3], recent_blockhash);
    banks_client.process_transaction(tx).await?;
    println!("   Third prover claimed");

    // Verify job now in Claimed state
    let job_account = banks_client.get_account(job_pda).await?.unwrap();
    let job = JobAccount::deserialize(&mut &job_account.data[..])?;
    assert_eq!(
        job.status,
        TypesJobStatus::Claimed,
        "Job should be Claimed after all provers claim"
    );

    // Verify FheConsensusData has all 3 claimed
    let fhe_data = fetch_fhe_consensus(&mut banks_client, &fhe_consensus_pda).await?;
    assert_eq!(fhe_data.claimed_count, 3);

    // Step 5: Only 2 provers submit results (insufficient)
    println!("\n5. Only 2 provers submitting results (insufficient for finalization)...");
    let consensus_hash = create_fhe_result_hash(100);

    for (i, prover) in [&prover1, &prover2].iter().enumerate() {
        let submit_ix =
            client.submit_fhe_result_instruction(&prover.pubkey(), &job_pda, job_id, consensus_hash)?;
        let recent_blockhash = banks_client.get_latest_blockhash().await?;
        let mut tx = Transaction::new_with_payer(&[submit_ix], Some(&prover.pubkey()));
        tx.sign(&[prover], recent_blockhash);
        banks_client.process_transaction(tx).await?;
        println!("   Prover {} submitted result", i + 1);
    }

    // Verify FheConsensusData has only 2 results
    let fhe_data = fetch_fhe_consensus(&mut banks_client, &fhe_consensus_pda).await?;
    assert_eq!(fhe_data.results_count, 2);

    // Step 6: Attempt to finalize (should fail - insufficient results)
    println!("\n6. Attempting to finalize with insufficient results...");
    let finalize_ix = client.finalize_fhe_job_instruction_with_recipient(
        &authority.pubkey(),
        &job_pda,
        job_id,
        &job_creator.pubkey(),
        &authority.pubkey(),
        &[prover1.pubkey(), prover2.pubkey()],
    )?;

    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[finalize_ix], Some(&authority.pubkey()));
    tx.sign(&[&authority], recent_blockhash);

    let result = banks_client.process_transaction(tx).await;

    // Should fail with InsufficientFheResults error
    assert!(
        result.is_err(),
        "Finalize should fail with insufficient results"
    );
    println!("   Finalize correctly rejected (insufficient results: 2/3)");

    // Verify job still in Claimed state
    let job_account = banks_client.get_account(job_pda).await?.unwrap();
    let job = JobAccount::deserialize(&mut &job_account.data[..])?;
    assert_eq!(job.status, TypesJobStatus::Claimed);

    // Verify FheConsensusData still has 2 results
    let fhe_data = fetch_fhe_consensus(&mut banks_client, &fhe_consensus_pda).await?;
    assert_eq!(fhe_data.results_count, 2);
    println!("   Job remains in Claimed state");

    println!("\n{}", "=".repeat(80));
    println!("TEST 3 PASSED!");
    println!("{}", "=".repeat(80));

    Ok(())
}

// ============================================================================
// TEST 4: All Provers Agree (Perfect Consensus)
// ============================================================================

#[tokio::test]
async fn test_fhe_all_provers_agree() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("TEST 4: All Provers Agree (Perfect Consensus)");
    println!("{}", "=".repeat(80));

    let program_id = program_id();
    let program_test = setup_program_test(program_id);
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let client = MarketplaceClient::new("http://localhost:8899".to_string(), program_id);

    let authority = Keypair::new();
    let prover1 = Keypair::new();
    let prover2 = Keypair::new();
    let prover3 = Keypair::new();
    let job_creator = Keypair::new();

    println!("\n1. Setting up marketplace and provers...");

    // Fund accounts
    for keypair in [&authority, &prover1, &prover2, &prover3, &job_creator] {
        fund_account(
            &mut banks_client,
            &payer,
            &keypair.pubkey(),
            10_000_000_000,
            recent_blockhash,
        )
        .await?;
    }

    // Initialize marketplace
    let init_ix = client.initialize_instruction(&authority.pubkey(), 1000, 1_000_000, 100, 3600)?;
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[init_ix], Some(&authority.pubkey()));
    tx.sign(&[&authority], recent_blockhash);
    banks_client.process_transaction(tx).await?;

    // Register 3 provers
    for (i, prover) in [&prover1, &prover2, &prover3].iter().enumerate() {
        let reg_ix =
            client.register_prover_instruction(&prover.pubkey(), 5_000_000, [(i + 1) as u8; 32])?;
        let recent_blockhash = banks_client.get_latest_blockhash().await?;
        let mut tx = Transaction::new_with_payer(&[reg_ix], Some(&prover.pubkey()));
        tx.sign(&[prover], recent_blockhash);
        banks_client.process_transaction(tx).await?;
    }
    println!("   Setup complete");

    // Step 2: Create FHE job
    println!("\n2. Creating FHE job...");
    let job_id = 0u64;
    let (job_pda, _) = client.get_job_pda(&job_creator.pubkey(), job_id);
    let (fhe_consensus_pda, _) = client.get_fhe_consensus_pda(job_id);

    let fhe_config = FheConsensusConfig {
        required_provers: 3,
        consensus_threshold: 2,
        submission_timeout_secs: 300,
        operation: FheOperation::Add(10),
    };

    let create_ix = client.create_job_instruction(
        &job_creator.pubkey(),
        job_id,
        zyberlink_sdk::CircuitType::FheComputation(FheOperation::Add(10)),
        [42u8; 32],
        1024,
        3_000_000,
        3600,
        Some(fhe_config),
    )?;

    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[create_ix], Some(&job_creator.pubkey()));
    tx.sign(&[&job_creator], recent_blockhash);
    banks_client.process_transaction(tx).await?;
    println!("   Job created");

    // Step 3: All provers claim
    println!("\n3. All 3 provers claiming...");
    for prover in [&prover1, &prover2, &prover3] {
        let claim_ix = client.claim_fhe_job_instruction(&prover.pubkey(), &job_pda, job_id)?;
        let recent_blockhash = banks_client.get_latest_blockhash().await?;
        let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&prover.pubkey()));
        tx.sign(&[prover], recent_blockhash);
        banks_client.process_transaction(tx).await?;
    }
    println!("   All provers claimed");

    // Step 4: All submit SAME hash (100% agreement)
    println!("\n4. All provers submitting SAME result (perfect consensus)...");
    let consensus_hash = create_fhe_result_hash(100);

    for (i, prover) in [&prover1, &prover2, &prover3].iter().enumerate() {
        let submit_ix =
            client.submit_fhe_result_instruction(&prover.pubkey(), &job_pda, job_id, consensus_hash)?;
        let recent_blockhash = banks_client.get_latest_blockhash().await?;
        let mut tx = Transaction::new_with_payer(&[submit_ix], Some(&prover.pubkey()));
        tx.sign(&[prover], recent_blockhash);
        banks_client.process_transaction(tx).await?;
        println!(
            "   Prover {} submitted (hash: {:02x}...)",
            i + 1,
            consensus_hash[0]
        );
    }

    // Step 5: Record balances
    let prover1_balance_before = get_account_balance(&mut banks_client, &prover1.pubkey()).await;
    let prover2_balance_before = get_account_balance(&mut banks_client, &prover2.pubkey()).await;
    let prover3_balance_before = get_account_balance(&mut banks_client, &prover3.pubkey()).await;

    // Step 6: Finalize
    println!("\n5. Finalizing job...");
    let finalize_ix = client.finalize_fhe_job_instruction_with_recipient(
        &authority.pubkey(),
        &job_pda,
        job_id,
        &job_creator.pubkey(),
        &authority.pubkey(),
        &[prover1.pubkey(), prover2.pubkey(), prover3.pubkey()], // All 3 match
    )?;

    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(&[finalize_ix], Some(&authority.pubkey()));
    tx.sign(&[&authority], recent_blockhash);
    banks_client.process_transaction(tx).await?;

    // Verify job completed
    let job_account = banks_client.get_account(job_pda).await?.unwrap();
    let job = JobAccount::deserialize(&mut &job_account.data[..])?;
    assert_eq!(job.status, TypesJobStatus::Completed);

    // Verify consensus hash in FheConsensusData
    let fhe_data = fetch_fhe_consensus(&mut banks_client, &fhe_consensus_pda).await?;
    assert_eq!(fhe_data.consensus_hash, Some(consensus_hash));
    println!("   Job finalized successfully!");

    // Step 7: Verify equal payouts to all 3 provers
    println!("\n6. Verifying payouts (all 3 should receive equal payment)...");

    let prover1_balance_after = get_account_balance(&mut banks_client, &prover1.pubkey()).await;
    let prover2_balance_after = get_account_balance(&mut banks_client, &prover2.pubkey()).await;
    let prover3_balance_after = get_account_balance(&mut banks_client, &prover3.pubkey()).await;

    let prover1_gain = prover1_balance_after.saturating_sub(prover1_balance_before);
    let prover2_gain = prover2_balance_after.saturating_sub(prover2_balance_before);
    let prover3_gain = prover3_balance_after.saturating_sub(prover3_balance_before);

    println!("   Prover 1 gained: {} lamports", prover1_gain);
    println!("   Prover 2 gained: {} lamports", prover2_gain);
    println!("   Prover 3 gained: {} lamports", prover3_gain);

    // All should receive equal payment (3M - 10% fee) / 3
    assert!(prover1_gain > 0, "Prover 1 should receive payment");
    assert!(prover2_gain > 0, "Prover 2 should receive payment");
    assert!(prover3_gain > 0, "Prover 3 should receive payment");

    // Verify equal distribution (within rounding)
    assert_eq!(
        prover1_gain, prover2_gain,
        "Provers 1 and 2 should get equal payout"
    );
    assert_eq!(
        prover2_gain, prover3_gain,
        "Provers 2 and 3 should get equal payout"
    );

    println!("   All 3 provers received equal payout!");

    println!("\n{}", "=".repeat(80));
    println!("TEST 4 PASSED!");
    println!("{}", "=".repeat(80));

    Ok(())
}
