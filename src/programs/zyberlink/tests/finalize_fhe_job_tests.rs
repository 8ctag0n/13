mod common;

use borsh::BorshDeserialize;
use common::{initialize_marketplace, register_prover, setup_program_test};
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use zyberlink::{instruction::MarketplaceInstruction, state::JobAccount};
use zyberlink_types::{fhe::FheOperation, CircuitType, FheConsensusConfig, JobStatus};

/// Helper to create an FHE job with proper accounts
async fn create_fhe_job(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    program_id: &Pubkey,
    config_pda: &Pubkey,
    job_id: u64,
    price_lamports: u64,
) -> (Pubkey, Pubkey, Pubkey) {
    let job_creator = payer.pubkey();
    let job_id_bytes = job_id.to_le_bytes();
    let (job_pda, _) =
        Pubkey::find_program_address(&[b"job", job_creator.as_ref(), &job_id_bytes], program_id);
    let (escrow_pda, _) = Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], program_id);
    let (fhe_consensus_pda, _) =
        Pubkey::find_program_address(&[b"fhe_consensus", &job_id_bytes], program_id);

    let fhe_operation = FheOperation::Add(5);
    let fhe_config = FheConsensusConfig {
        required_provers: 3,
        consensus_threshold: 2,
        submission_timeout_secs: 60,
        operation: fhe_operation.clone(),
    };

    let create_job_instruction = MarketplaceInstruction::CreateJob {
        circuit_type: CircuitType::FheComputation(fhe_operation),
        witness_commitment: [1u8; 32],
        witness_size: 2048,
        price_lamports,
        timeout_seconds: 0,
        fhe_config: Some(fhe_config),
    };

    let create_job_ix = solana_program::instruction::Instruction {
        program_id: *program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(job_creator, true),
            solana_program::instruction::AccountMeta::new(job_pda, false),
            solana_program::instruction::AccountMeta::new(*config_pda, false),
            solana_program::instruction::AccountMeta::new(escrow_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::system_program::id(),
                false,
            ),
            solana_program::instruction::AccountMeta::new(fhe_consensus_pda, false),
        ],
        data: create_job_instruction.pack().unwrap(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut create_job_tx = Transaction::new_with_payer(&[create_job_ix], Some(&payer.pubkey()));
    create_job_tx.sign(&[payer], recent_blockhash);
    banks_client
        .process_transaction(create_job_tx)
        .await
        .unwrap();

    (job_pda, escrow_pda, fhe_consensus_pda)
}

/// Helper to claim FHE job with a prover
async fn claim_fhe_job(
    banks_client: &mut BanksClient,
    prover_keypair: &Keypair,
    program_id: &Pubkey,
    config_pda: &Pubkey,
    job_pda: &Pubkey,
    fhe_consensus_pda: &Pubkey,
) -> Pubkey {
    let prover_authority = prover_keypair.pubkey();
    let (prover_pda, _) =
        Pubkey::find_program_address(&[b"prover", prover_authority.as_ref()], program_id);

    let claim_job_instruction = MarketplaceInstruction::ClaimJob;
    let claim_job_ix = solana_program::instruction::Instruction {
        program_id: *program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(prover_authority, true),
            solana_program::instruction::AccountMeta::new(prover_pda, false),
            solana_program::instruction::AccountMeta::new(*job_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(*config_pda, false),
            solana_program::instruction::AccountMeta::new(*fhe_consensus_pda, false),
        ],
        data: claim_job_instruction.pack().unwrap(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut claim_job_tx = Transaction::new_with_payer(&[claim_job_ix], Some(&prover_authority));
    claim_job_tx.sign(&[prover_keypair], recent_blockhash);
    banks_client
        .process_transaction(claim_job_tx)
        .await
        .unwrap();

    prover_pda
}

/// Helper to submit FHE result
async fn submit_fhe_result(
    banks_client: &mut BanksClient,
    prover_keypair: &Keypair,
    program_id: &Pubkey,
    job_pda: &Pubkey,
    fhe_consensus_pda: &Pubkey,
    result_hash: [u8; 32],
) {
    let prover_authority = prover_keypair.pubkey();

    let submit_result_instruction = MarketplaceInstruction::SubmitFheResult { result_hash };

    let submit_result_ix = solana_program::instruction::Instruction {
        program_id: *program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(prover_authority, true),
            solana_program::instruction::AccountMeta::new(*job_pda, false),
            solana_program::instruction::AccountMeta::new(*fhe_consensus_pda, false),
        ],
        data: submit_result_instruction.pack().unwrap(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut submit_result_tx =
        Transaction::new_with_payer(&[submit_result_ix], Some(&prover_authority));
    submit_result_tx.sign(&[prover_keypair], recent_blockhash);
    banks_client
        .process_transaction(submit_result_tx)
        .await
        .unwrap();
}

/// Test 1: Finalize with consensus reached (2/3 matching)
#[tokio::test]
async fn test_finalize_fhe_job_consensus_reached() {
    let program_id = Pubkey::new_unique();
    let program_test = setup_program_test(program_id);

    let (mut banks_client, payer, _recent_blockhash) = program_test.start().await;

    // Initialize marketplace
    let config_pda = initialize_marketplace(
        &mut banks_client,
        &payer,
        &program_id,
        1000, // 10% fee
        5_000_000_000,
        500,
        600,
    )
    .await
    .unwrap();

    // Register 3 provers
    let prover1_keypair = Keypair::new();
    let _prover1_pda = register_prover(
        &mut banks_client,
        &payer,
        &prover1_keypair,
        &program_id,
        &config_pda,
        5_000_000_000,
    )
    .await
    .unwrap();

    let prover2_keypair = Keypair::new();
    let _prover2_pda = register_prover(
        &mut banks_client,
        &payer,
        &prover2_keypair,
        &program_id,
        &config_pda,
        5_000_000_000,
    )
    .await
    .unwrap();

    let prover3_keypair = Keypair::new();
    let _prover3_pda = register_prover(
        &mut banks_client,
        &payer,
        &prover3_keypair,
        &program_id,
        &config_pda,
        5_000_000_000,
    )
    .await
    .unwrap();

    // Create FHE job
    let job_id = 0u64;
    let (job_pda, escrow_pda, fhe_consensus_pda) = create_fhe_job(
        &mut banks_client,
        &payer,
        &program_id,
        &config_pda,
        job_id,
        3_000_000,
    )
    .await;

    // Claim job with all 3 provers and collect their PDAs
    let mut prover_pdas = Vec::new();
    for prover_keypair in [&prover1_keypair, &prover2_keypair, &prover3_keypair] {
        let prover_pda = claim_fhe_job(
            &mut banks_client,
            prover_keypair,
            &program_id,
            &config_pda,
            &job_pda,
            &fhe_consensus_pda,
        )
        .await;
        prover_pdas.push(prover_pda);
    }

    // Submit results: 2 matching [5u8; 32], 1 different [9u8; 32]
    let matching_hash = [5u8; 32];
    let different_hash = [9u8; 32];

    // Prover 1 and 2 submit matching result
    for prover_keypair in [&prover1_keypair, &prover2_keypair] {
        submit_fhe_result(
            &mut banks_client,
            prover_keypair,
            &program_id,
            &job_pda,
            &fhe_consensus_pda,
            matching_hash,
        )
        .await;
    }

    // Prover 3 submits different result
    submit_fhe_result(
        &mut banks_client,
        &prover3_keypair,
        &program_id,
        &job_pda,
        &fhe_consensus_pda,
        different_hash,
    )
    .await;

    // Get protocol fee recipient from config
    let config_account = banks_client.get_account(config_pda).await.unwrap().unwrap();
    let config: zyberlink::state::MarketplaceConfig =
        borsh::from_slice(&config_account.data).unwrap();

    // Finalize FHE job
    let finalizer = payer.pubkey();
    let finalize_instruction = MarketplaceInstruction::FinalizeFheJob;

    let mut finalize_accounts = vec![
        solana_program::instruction::AccountMeta::new(finalizer, true),
        solana_program::instruction::AccountMeta::new(job_pda, false),
        solana_program::instruction::AccountMeta::new(fhe_consensus_pda, false),
        solana_program::instruction::AccountMeta::new(escrow_pda, false),
        solana_program::instruction::AccountMeta::new(payer.pubkey(), false),
        solana_program::instruction::AccountMeta::new(config.protocol_fee_recipient, false),
        solana_program::instruction::AccountMeta::new_readonly(config_pda, false),
        solana_program::instruction::AccountMeta::new_readonly(
            solana_program::system_program::id(),
            false,
        ),
        solana_program::instruction::AccountMeta::new_readonly(
            solana_program::sysvar::clock::id(),
            false,
        ),
    ];

    // Add prover accounts (authority + PDA for each)
    for (i, prover_keypair) in [&prover1_keypair, &prover2_keypair, &prover3_keypair]
        .iter()
        .enumerate()
    {
        finalize_accounts.push(solana_program::instruction::AccountMeta::new(
            prover_keypair.pubkey(),
            false,
        ));
        finalize_accounts.push(solana_program::instruction::AccountMeta::new(
            prover_pdas[i],
            false,
        ));
    }

    let finalize_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: finalize_accounts,
        data: finalize_instruction.pack().unwrap(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut finalize_tx = Transaction::new_with_payer(&[finalize_ix], Some(&finalizer));
    finalize_tx.sign(&[&payer], recent_blockhash);

    let result = banks_client.process_transaction(finalize_tx).await;
    assert!(
        result.is_ok(),
        "FinalizeFheJob should succeed with consensus: {:?}",
        result.err()
    );

    // Verify job status
    let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
    let job: JobAccount = {
        let mut data_slice = &job_account.data[..];
        JobAccount::deserialize(&mut data_slice).unwrap()
    };

    assert_eq!(job.status, JobStatus::Completed);
    // proof_hash stores the consensus result
    assert_eq!(job.proof_hash, Some(matching_hash));
}

/// Test 2: Finalize before consensus fails
#[tokio::test]
async fn test_finalize_fhe_job_no_consensus() {
    let program_id = Pubkey::new_unique();
    let program_test = setup_program_test(program_id);

    let (mut banks_client, payer, _recent_blockhash) = program_test.start().await;

    // Initialize marketplace
    let config_pda = initialize_marketplace(
        &mut banks_client,
        &payer,
        &program_id,
        1000,
        5_000_000_000,
        500,
        600,
    )
    .await
    .unwrap();

    // Register 3 provers
    let prover1_keypair = Keypair::new();
    let _prover1_pda = register_prover(
        &mut banks_client,
        &payer,
        &prover1_keypair,
        &program_id,
        &config_pda,
        5_000_000_000,
    )
    .await
    .unwrap();

    let prover2_keypair = Keypair::new();
    let _prover2_pda = register_prover(
        &mut banks_client,
        &payer,
        &prover2_keypair,
        &program_id,
        &config_pda,
        5_000_000_000,
    )
    .await
    .unwrap();

    let prover3_keypair = Keypair::new();
    let _prover3_pda = register_prover(
        &mut banks_client,
        &payer,
        &prover3_keypair,
        &program_id,
        &config_pda,
        5_000_000_000,
    )
    .await
    .unwrap();

    // Create FHE job
    let job_id = 0u64;
    let (job_pda, escrow_pda, fhe_consensus_pda) = create_fhe_job(
        &mut banks_client,
        &payer,
        &program_id,
        &config_pda,
        job_id,
        3_000_000,
    )
    .await;

    // Claim job with all 3 provers
    let mut prover_pdas = Vec::new();
    for prover_keypair in [&prover1_keypair, &prover2_keypair, &prover3_keypair] {
        let prover_pda = claim_fhe_job(
            &mut banks_client,
            prover_keypair,
            &program_id,
            &config_pda,
            &job_pda,
            &fhe_consensus_pda,
        )
        .await;
        prover_pdas.push(prover_pda);
    }

    // Only 1 prover submits result (not enough for finalization - need all 3)
    let result_hash = [5u8; 32];
    submit_fhe_result(
        &mut banks_client,
        &prover1_keypair,
        &program_id,
        &job_pda,
        &fhe_consensus_pda,
        result_hash,
    )
    .await;

    // Get protocol fee recipient from config
    let config_account = banks_client.get_account(config_pda).await.unwrap().unwrap();
    let config: zyberlink::state::MarketplaceConfig =
        borsh::from_slice(&config_account.data).unwrap();

    // Try to finalize (should fail - not enough submissions)
    let finalizer = payer.pubkey();
    let finalize_instruction = MarketplaceInstruction::FinalizeFheJob;

    let mut finalize_accounts = vec![
        solana_program::instruction::AccountMeta::new(finalizer, true),
        solana_program::instruction::AccountMeta::new(job_pda, false),
        solana_program::instruction::AccountMeta::new(fhe_consensus_pda, false),
        solana_program::instruction::AccountMeta::new(escrow_pda, false),
        solana_program::instruction::AccountMeta::new(payer.pubkey(), false),
        solana_program::instruction::AccountMeta::new(config.protocol_fee_recipient, false),
        solana_program::instruction::AccountMeta::new_readonly(config_pda, false),
        solana_program::instruction::AccountMeta::new_readonly(
            solana_program::system_program::id(),
            false,
        ),
        solana_program::instruction::AccountMeta::new_readonly(
            solana_program::sysvar::clock::id(),
            false,
        ),
    ];

    // Add only prover1 (since only 1 result submitted)
    finalize_accounts.push(solana_program::instruction::AccountMeta::new(
        prover1_keypair.pubkey(),
        false,
    ));
    finalize_accounts.push(solana_program::instruction::AccountMeta::new(
        prover_pdas[0],
        false,
    ));

    let finalize_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: finalize_accounts,
        data: finalize_instruction.pack().unwrap(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut finalize_tx = Transaction::new_with_payer(&[finalize_ix], Some(&finalizer));
    finalize_tx.sign(&[&payer], recent_blockhash);

    let result = banks_client.process_transaction(finalize_tx).await;
    assert!(
        result.is_err(),
        "Finalize should fail with insufficient results"
    );
}

/// Test 3: Payment distribution correct
#[tokio::test]
async fn test_finalize_fhe_job_payment_split() {
    let program_id = Pubkey::new_unique();
    let program_test = setup_program_test(program_id);

    let (mut banks_client, payer, _recent_blockhash) = program_test.start().await;

    // Initialize marketplace
    let config_pda = initialize_marketplace(
        &mut banks_client,
        &payer,
        &program_id,
        1000, // 10% fee
        5_000_000_000,
        500,
        600,
    )
    .await
    .unwrap();

    // Register 3 provers
    let prover1_keypair = Keypair::new();
    let _prover1_pda = register_prover(
        &mut banks_client,
        &payer,
        &prover1_keypair,
        &program_id,
        &config_pda,
        5_000_000_000,
    )
    .await
    .unwrap();

    let prover2_keypair = Keypair::new();
    let _prover2_pda = register_prover(
        &mut banks_client,
        &payer,
        &prover2_keypair,
        &program_id,
        &config_pda,
        5_000_000_000,
    )
    .await
    .unwrap();

    let prover3_keypair = Keypair::new();
    let _prover3_pda = register_prover(
        &mut banks_client,
        &payer,
        &prover3_keypair,
        &program_id,
        &config_pda,
        5_000_000_000,
    )
    .await
    .unwrap();

    // Create FHE job
    let job_id = 0u64;
    let (job_pda, escrow_pda, fhe_consensus_pda) = create_fhe_job(
        &mut banks_client,
        &payer,
        &program_id,
        &config_pda,
        job_id,
        3_000_000,
    )
    .await;

    // Claim job with all 3 provers
    let mut prover_pdas = Vec::new();
    for prover_keypair in [&prover1_keypair, &prover2_keypair, &prover3_keypair] {
        let prover_pda = claim_fhe_job(
            &mut banks_client,
            prover_keypair,
            &program_id,
            &config_pda,
            &job_pda,
            &fhe_consensus_pda,
        )
        .await;
        prover_pdas.push(prover_pda);
    }

    // All 3 submit matching result
    let matching_hash = [5u8; 32];
    for prover_keypair in [&prover1_keypair, &prover2_keypair, &prover3_keypair] {
        submit_fhe_result(
            &mut banks_client,
            prover_keypair,
            &program_id,
            &job_pda,
            &fhe_consensus_pda,
            matching_hash,
        )
        .await;
    }

    // Get initial balances
    let prover1_balance_before = banks_client
        .get_account(prover1_keypair.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let prover2_balance_before = banks_client
        .get_account(prover2_keypair.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let prover3_balance_before = banks_client
        .get_account(prover3_keypair.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Get protocol fee recipient from config
    let config_account = banks_client.get_account(config_pda).await.unwrap().unwrap();
    let config: zyberlink::state::MarketplaceConfig =
        borsh::from_slice(&config_account.data).unwrap();

    // Finalize FHE job
    let finalizer = payer.pubkey();
    let finalize_instruction = MarketplaceInstruction::FinalizeFheJob;

    let mut finalize_accounts = vec![
        solana_program::instruction::AccountMeta::new(finalizer, true),
        solana_program::instruction::AccountMeta::new(job_pda, false),
        solana_program::instruction::AccountMeta::new(fhe_consensus_pda, false),
        solana_program::instruction::AccountMeta::new(escrow_pda, false),
        solana_program::instruction::AccountMeta::new(payer.pubkey(), false),
        solana_program::instruction::AccountMeta::new(config.protocol_fee_recipient, false),
        solana_program::instruction::AccountMeta::new_readonly(config_pda, false),
        solana_program::instruction::AccountMeta::new_readonly(
            solana_program::system_program::id(),
            false,
        ),
        solana_program::instruction::AccountMeta::new_readonly(
            solana_program::sysvar::clock::id(),
            false,
        ),
    ];

    // Add prover accounts
    for (i, prover_keypair) in [&prover1_keypair, &prover2_keypair, &prover3_keypair]
        .iter()
        .enumerate()
    {
        finalize_accounts.push(solana_program::instruction::AccountMeta::new(
            prover_keypair.pubkey(),
            false,
        ));
        finalize_accounts.push(solana_program::instruction::AccountMeta::new(
            prover_pdas[i],
            false,
        ));
    }

    let finalize_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: finalize_accounts,
        data: finalize_instruction.pack().unwrap(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut finalize_tx = Transaction::new_with_payer(&[finalize_ix], Some(&finalizer));
    finalize_tx.sign(&[&payer], recent_blockhash);

    banks_client.process_transaction(finalize_tx).await.unwrap();

    // Verify payment distribution
    let prover1_balance_after = banks_client
        .get_account(prover1_keypair.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let prover2_balance_after = banks_client
        .get_account(prover2_keypair.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let prover3_balance_after = banks_client
        .get_account(prover3_keypair.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Calculate expected payments
    // Total: 3_000_000 lamports
    // Fee: 10% = 300_000 lamports
    // Prover payout total: 2_700_000 lamports
    // Per prover: 2_700_000 / 3 = 900_000 lamports
    let expected_per_prover = 900_000;

    let prover1_received = prover1_balance_after - prover1_balance_before;
    let prover2_received = prover2_balance_after - prover2_balance_before;
    let prover3_received = prover3_balance_after - prover3_balance_before;

    assert_eq!(prover1_received, expected_per_prover);
    assert_eq!(prover2_received, expected_per_prover);
    assert_eq!(prover3_received, expected_per_prover);
}
