use cypherlink_sdk::{
    calculate_platform_fee, calculate_prover_payout, get_time_remaining, is_job_timed_out,
    CircuitType, JobStatus, MarketplaceClient,
};
use solana_program_test::{processor, ProgramTest};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};

fn program_id() -> solana_sdk::pubkey::Pubkey {
    solana_sdk::pubkey::Pubkey::new_unique()
}

fn setup_program_test(program_id: solana_sdk::pubkey::Pubkey) -> ProgramTest {
    let mut program_test = ProgramTest::new(
        "cypherlink",
        program_id,
        processor!(cypherlink::process_instruction),
    );
    program_test
}

#[tokio::test]
async fn test_initialize_marketplace() {
    let program_id = program_id();
    let program_test = setup_program_test(program_id);
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let client = MarketplaceClient::new("http://localhost:8899".to_string(), program_id);

    // Build initialize instruction
    let init_ix = client
        .initialize_instruction(
            &payer.pubkey(),
            1000,           // 10% fee
            5_000_000_000,  // 5 SOL stake
            500,            // min reputation
            600,            // 10 min timeout
        )
        .unwrap();

    // Send transaction
    let mut tx = Transaction::new_with_payer(&[init_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Verify config account exists
    let (config_pda, _) = client.get_config_pda();
    let config_account = banks_client.get_account(config_pda).await.unwrap();
    assert!(config_account.is_some(), "Config account should exist");
}

#[tokio::test]
async fn test_register_prover() {
    let program_id = program_id();
    let program_test = setup_program_test(program_id);
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let client = MarketplaceClient::new("http://localhost:8899".to_string(), program_id);

    // Initialize marketplace first
    let init_ix = client
        .initialize_instruction(&payer.pubkey(), 1000, 5_000_000_000, 500, 600)
        .unwrap();

    let mut tx = Transaction::new_with_payer(&[init_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Create and fund prover
    let prover_keypair = Keypair::new();
    let fund_ix =
        system_instruction::transfer(&payer.pubkey(), &prover_keypair.pubkey(), 10_000_000_000);

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[fund_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Register prover
    let register_ix = client
        .register_prover_instruction(&prover_keypair.pubkey(), 5_000_000_000, [1u8; 32])
        .unwrap();

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[register_ix], Some(&prover_keypair.pubkey()));
    tx.sign(&[&prover_keypair], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Verify prover account exists
    let (prover_pda, _) = client.get_prover_pda(&prover_keypair.pubkey());
    let prover_account = banks_client.get_account(prover_pda).await.unwrap();
    assert!(prover_account.is_some(), "Prover account should exist");
}

#[tokio::test]
async fn test_create_job() {
    let program_id = program_id();
    let program_test = setup_program_test(program_id);
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let client = MarketplaceClient::new("http://localhost:8899".to_string(), program_id);

    // Initialize marketplace
    let init_ix = client
        .initialize_instruction(&payer.pubkey(), 1000, 5_000_000_000, 500, 600)
        .unwrap();

    let mut tx = Transaction::new_with_payer(&[init_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Create job
    let create_job_ix = client
        .create_job_instruction(
            &payer.pubkey(),
            0,
            CircuitType::ZcashOrchard,
            [1u8; 32],
            2048,
            1_000_000,
            600,
            None, // No FHE config for ZK jobs
        )
        .unwrap();

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[create_job_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Verify job account exists
    let (job_pda, _) = client.get_job_pda(&payer.pubkey(), 0);
    let job_account = banks_client.get_account(job_pda).await.unwrap();
    assert!(job_account.is_some(), "Job account should exist");

    // Verify escrow account exists
    let (escrow_pda, _) = client.get_escrow_pda(&job_pda);
    let escrow_account = banks_client.get_account(escrow_pda).await.unwrap();
    assert!(escrow_account.is_some(), "Escrow account should exist");
}

#[tokio::test]
async fn test_full_job_lifecycle() {
    let program_id = program_id();
    let program_test = setup_program_test(program_id);
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let client = MarketplaceClient::new("http://localhost:8899".to_string(), program_id);

    // Initialize
    let init_ix = client
        .initialize_instruction(&payer.pubkey(), 1000, 5_000_000_000, 500, 600)
        .unwrap();
    let mut tx = Transaction::new_with_payer(&[init_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Register prover
    let prover_keypair = Keypair::new();
    let fund_ix =
        system_instruction::transfer(&payer.pubkey(), &prover_keypair.pubkey(), 10_000_000_000);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[fund_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    let register_ix = client
        .register_prover_instruction(&prover_keypair.pubkey(), 5_000_000_000, [1u8; 32])
        .unwrap();
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[register_ix], Some(&prover_keypair.pubkey()));
    tx.sign(&[&prover_keypair], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Create job
    let create_job_ix = client
        .create_job_instruction(
            &payer.pubkey(),
            0,
            CircuitType::ZcashOrchard,
            [1u8; 32],
            2048,
            1_000_000,
            600,
            None, // No FHE config for ZK jobs
        )
        .unwrap();
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[create_job_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    let (job_pda, _) = client.get_job_pda(&payer.pubkey(), 0);

    // Claim job
    let claim_ix = client
        .claim_job_instruction(&prover_keypair.pubkey(), &job_pda)
        .unwrap();
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[claim_ix], Some(&prover_keypair.pubkey()));
    tx.sign(&[&prover_keypair], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Submit proof (using version without RPC call for tests)
    let (config_pda, _) = client.get_config_pda();
    let config_account = banks_client.get_account(config_pda).await.unwrap().unwrap();
    // Offset: authority(32) + fee_basis_points(2) + min_stake(8) + min_rep(4) + timeout(8) = 54
    let protocol_fee_recipient = Pubkey::try_from(&config_account.data[54..86]).unwrap();

    let submit_ix = client
        .submit_proof_instruction_with_recipient(
            &prover_keypair.pubkey(),
            &job_pda,
            &payer.pubkey(),
            &protocol_fee_recipient,
            [2u8; 32],
            1536,
        )
        .unwrap();
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[submit_ix], Some(&prover_keypair.pubkey()));
    tx.sign(&[&prover_keypair], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Verify job completed (account still exists)
    let job_account = banks_client.get_account(job_pda).await.unwrap();
    assert!(job_account.is_some(), "Job should be completed");
}

#[tokio::test]
async fn test_cancel_job() {
    let program_id = program_id();
    let program_test = setup_program_test(program_id);
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let client = MarketplaceClient::new("http://localhost:8899".to_string(), program_id);

    // Initialize
    let init_ix = client
        .initialize_instruction(&payer.pubkey(), 1000, 5_000_000_000, 500, 600)
        .unwrap();
    let mut tx = Transaction::new_with_payer(&[init_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Create job
    let create_job_ix = client
        .create_job_instruction(
            &payer.pubkey(),
            0,
            CircuitType::ZcashOrchard,
            [1u8; 32],
            2048,
            1_000_000,
            600,
            None, // No FHE config for ZK jobs
        )
        .unwrap();
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[create_job_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    let (job_pda, _) = client.get_job_pda(&payer.pubkey(), 0);

    // Cancel job
    let cancel_ix = client.cancel_job_instruction(&payer.pubkey(), &job_pda).unwrap();
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[cancel_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Job account should still exist (cancelled, not closed)
    let job_account = banks_client.get_account(job_pda).await.unwrap();
    assert!(job_account.is_some(), "Job account should still exist after cancellation");
}

#[tokio::test]
async fn test_multiple_jobs() {
    let program_id = program_id();
    let program_test = setup_program_test(program_id);
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let client = MarketplaceClient::new("http://localhost:8899".to_string(), program_id);

    // Initialize
    let init_ix = client
        .initialize_instruction(&payer.pubkey(), 1000, 5_000_000_000, 500, 600)
        .unwrap();
    let mut tx = Transaction::new_with_payer(&[init_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Create 5 jobs
    for i in 0..5u64 {
        let create_job_ix = client
            .create_job_instruction(
                &payer.pubkey(),
                i,
                CircuitType::ZcashOrchard,
                [i as u8; 32],
                2048,
                1_000_000,
                600,
                None, // No FHE config for ZK jobs
            )
            .unwrap();

        let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
        let mut tx = Transaction::new_with_payer(&[create_job_ix], Some(&payer.pubkey()));
        tx.sign(&[&payer], recent_blockhash);
        banks_client.process_transaction(tx).await.unwrap();
    }

    // Verify all 5 jobs exist
    for i in 0..5u64 {
        let (job_pda, _) = client.get_job_pda(&payer.pubkey(), i);
        let job_account = banks_client.get_account(job_pda).await.unwrap();
        assert!(job_account.is_some(), "Job {} should exist", i);
    }
}

// ============================================================================
// Utility Function Tests
// ============================================================================

#[test]
fn test_fee_calculations() {
    // 10% fee
    assert_eq!(calculate_platform_fee(1_000_000, 1000), 100_000);
    assert_eq!(calculate_prover_payout(1_000_000, 1000), 900_000);

    // 5% fee
    assert_eq!(calculate_platform_fee(1_000_000, 500), 50_000);
    assert_eq!(calculate_prover_payout(1_000_000, 500), 950_000);

    // 20% fee
    assert_eq!(calculate_platform_fee(1_000_000, 2000), 200_000);
    assert_eq!(calculate_prover_payout(1_000_000, 2000), 800_000);

    // 0% fee
    assert_eq!(calculate_platform_fee(1_000_000, 0), 0);
    assert_eq!(calculate_prover_payout(1_000_000, 0), 1_000_000);

    // 100% fee
    assert_eq!(calculate_platform_fee(1_000_000, 10000), 1_000_000);
    assert_eq!(calculate_prover_payout(1_000_000, 10000), 0);
}

#[test]
fn test_timeout_calculations() {
    use cypherlink_sdk::JobAccount;
    use solana_sdk::pubkey::Pubkey;

    let job = JobAccount {
        id: 0,
        creator: Pubkey::new_unique(),
        prover: Some(Pubkey::new_unique()),
        status: JobStatus::Claimed,
        circuit_type: CircuitType::ZcashOrchard,
        witness_commitment: [0u8; 32],
        witness_size: 1024,
        proof_commitment: None,
        proof_size: None,
        price_lamports: 1_000_000,
        escrow_account: Pubkey::new_unique(),
        created_at: 1000,
        claimed_at: Some(1000),
        completed_at: None,
        timeout_at: 1600, // 600 seconds timeout from claimed_at (1000)
        bump: 0,
        fhe_config: None,
        claimed_provers: vec![],
        fhe_results: vec![],
        fhe_consensus_hash: None,
    };

    // Not timed out yet
    assert!(!is_job_timed_out(&job, 1100));
    assert_eq!(get_time_remaining(&job, 1100), Some(500));

    // Halfway through
    assert!(!is_job_timed_out(&job, 1300));
    assert_eq!(get_time_remaining(&job, 1300), Some(300));

    // Just at timeout
    assert!(is_job_timed_out(&job, 1601));
    assert_eq!(get_time_remaining(&job, 1601), Some(0));

    // Way past timeout
    assert!(is_job_timed_out(&job, 2000));
    assert_eq!(get_time_remaining(&job, 2000), Some(0));
}

#[test]
fn test_pda_derivation_consistency() {
    let program_id = Pubkey::new_unique();
    let client = MarketplaceClient::new("http://localhost:8899".to_string(), program_id);

    // Same inputs should produce same PDAs
    let (config1, bump1) = client.get_config_pda();
    let (config2, bump2) = client.get_config_pda();
    assert_eq!(config1, config2);
    assert_eq!(bump1, bump2);

    // Different job IDs should produce different PDAs
    let creator = Pubkey::new_unique();
    let (job0, _) = client.get_job_pda(&creator, 0);
    let (job1, _) = client.get_job_pda(&creator, 1);
    assert_ne!(job0, job1);

    // Different creators should produce different PDAs
    let creator2 = Pubkey::new_unique();
    let (job_c1, _) = client.get_job_pda(&creator, 0);
    let (job_c2, _) = client.get_job_pda(&creator2, 0);
    assert_ne!(job_c1, job_c2);
}
