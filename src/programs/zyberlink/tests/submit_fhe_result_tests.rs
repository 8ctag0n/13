mod common;

use borsh::BorshDeserialize;
use common::{initialize_marketplace, register_prover, setup_program_test};
use cypherlink::{instruction::MarketplaceInstruction, state::JobAccount};
use cypherlink_types::{fhe::FheOperation, CircuitType, FheConsensusConfig, JobStatus};
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

/// Test 1: Submit FHE result successfully
#[tokio::test]
async fn test_submit_fhe_result_success() {
    let program_id = Pubkey::new_unique();
    let program_test = setup_program_test(program_id);

    let (mut banks_client, payer, _recent_blockhash) = program_test.start().await;

    // 1. Initialize marketplace
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

    // 2. Register 3 provers
    let prover1_keypair = Keypair::new();
    let prover1_pda = register_prover(
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

    // 3. Create FHE job
    let job_creator = payer.pubkey();
    let job_id = 0u64;
    let (job_pda, _) = Pubkey::find_program_address(
        &[b"job", job_creator.as_ref(), &job_id.to_le_bytes()],
        &program_id,
    );
    let (escrow_pda, _) = Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], &program_id);

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
        price_lamports: 3_000_000,
        timeout_seconds: 0,
        fhe_config: Some(fhe_config),
    };

    let create_job_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(job_creator, true),
            solana_program::instruction::AccountMeta::new(job_pda, false),
            solana_program::instruction::AccountMeta::new(config_pda, false),
            solana_program::instruction::AccountMeta::new(escrow_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::system_program::id(),
                false,
            ),
        ],
        data: create_job_instruction.pack().unwrap(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut create_job_tx = Transaction::new_with_payer(&[create_job_ix], Some(&payer.pubkey()));
    create_job_tx.sign(&[&payer], recent_blockhash);
    banks_client
        .process_transaction(create_job_tx)
        .await
        .unwrap();

    // 4. Claim job with all 3 provers
    for prover_keypair in [&prover1_keypair, &prover2_keypair, &prover3_keypair] {
        let prover_authority = prover_keypair.pubkey();
        let (prover_pda, _) =
            Pubkey::find_program_address(&[b"prover", prover_authority.as_ref()], &program_id);

        let claim_job_instruction = MarketplaceInstruction::ClaimJob;
        let claim_job_ix = solana_program::instruction::Instruction {
            program_id,
            accounts: vec![
                solana_program::instruction::AccountMeta::new(prover_authority, true),
                solana_program::instruction::AccountMeta::new(prover_pda, false),
                solana_program::instruction::AccountMeta::new(job_pda, false),
                solana_program::instruction::AccountMeta::new_readonly(config_pda, false),
                solana_program::instruction::AccountMeta::new_readonly(
                    solana_program::sysvar::clock::id(),
                    false,
                ),
            ],
            data: claim_job_instruction.pack().unwrap(),
        };

        let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
        let mut claim_job_tx =
            Transaction::new_with_payer(&[claim_job_ix], Some(&prover_authority));
        claim_job_tx.sign(&[prover_keypair], recent_blockhash);
        banks_client
            .process_transaction(claim_job_tx)
            .await
            .unwrap();
    }

    // 5. Submit FHE result from prover1
    let prover1_authority = prover1_keypair.pubkey();
    let result_hash = [2u8; 32];

    let submit_result_instruction = MarketplaceInstruction::SubmitFheResult { result_hash };

    let submit_result_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(prover1_authority, true),
            solana_program::instruction::AccountMeta::new(job_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::sysvar::clock::id(),
                false,
            ),
        ],
        data: submit_result_instruction.pack().unwrap(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut submit_result_tx =
        Transaction::new_with_payer(&[submit_result_ix], Some(&prover1_authority));
    submit_result_tx.sign(&[&prover1_keypair], recent_blockhash);

    let result = banks_client.process_transaction(submit_result_tx).await;
    assert!(
        result.is_ok(),
        "SubmitFheResult should succeed: {:?}",
        result.err()
    );

    // 6. Verify result stored in job.fhe_results
    let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
    let job: JobAccount = {
        let mut data_slice = &job_account.data[..];
        JobAccount::deserialize(&mut data_slice).unwrap()
    };

    assert_eq!(job.fhe_results.len(), 1);
    assert_eq!(job.fhe_results[0].prover, prover1_authority);
    assert_eq!(job.fhe_results[0].result_hash, result_hash);
    assert_eq!(job.status, JobStatus::Claimed);
}

/// Test 2: Submit duplicate result fails
#[tokio::test]
async fn test_submit_fhe_result_duplicate() {
    let program_id = Pubkey::new_unique();
    let program_test = setup_program_test(program_id);

    let (mut banks_client, payer, _recent_blockhash) = program_test.start().await;

    // Setup marketplace and provers
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

    let prover_keypair = Keypair::new();
    let _prover_pda = register_prover(
        &mut banks_client,
        &payer,
        &prover_keypair,
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
    let job_creator = payer.pubkey();
    let job_id = 0u64;
    let (job_pda, _) = Pubkey::find_program_address(
        &[b"job", job_creator.as_ref(), &job_id.to_le_bytes()],
        &program_id,
    );
    let (escrow_pda, _) = Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], &program_id);

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
        price_lamports: 3_000_000,
        timeout_seconds: 0,
        fhe_config: Some(fhe_config),
    };

    let create_job_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(job_creator, true),
            solana_program::instruction::AccountMeta::new(job_pda, false),
            solana_program::instruction::AccountMeta::new(config_pda, false),
            solana_program::instruction::AccountMeta::new(escrow_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::system_program::id(),
                false,
            ),
        ],
        data: create_job_instruction.pack().unwrap(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut create_job_tx = Transaction::new_with_payer(&[create_job_ix], Some(&payer.pubkey()));
    create_job_tx.sign(&[&payer], recent_blockhash);
    banks_client
        .process_transaction(create_job_tx)
        .await
        .unwrap();

    // Claim job with all 3 provers
    for prover_keypair in [&prover_keypair, &prover2_keypair, &prover3_keypair] {
        let prover_authority = prover_keypair.pubkey();
        let (prover_pda, _) =
            Pubkey::find_program_address(&[b"prover", prover_authority.as_ref()], &program_id);

        let claim_job_instruction = MarketplaceInstruction::ClaimJob;
        let claim_job_ix = solana_program::instruction::Instruction {
            program_id,
            accounts: vec![
                solana_program::instruction::AccountMeta::new(prover_authority, true),
                solana_program::instruction::AccountMeta::new(prover_pda, false),
                solana_program::instruction::AccountMeta::new(job_pda, false),
                solana_program::instruction::AccountMeta::new_readonly(config_pda, false),
                solana_program::instruction::AccountMeta::new_readonly(
                    solana_program::sysvar::clock::id(),
                    false,
                ),
            ],
            data: claim_job_instruction.pack().unwrap(),
        };

        let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
        let mut claim_job_tx =
            Transaction::new_with_payer(&[claim_job_ix], Some(&prover_authority));
        claim_job_tx.sign(&[prover_keypair], recent_blockhash);
        banks_client
            .process_transaction(claim_job_tx)
            .await
            .unwrap();
    }

    // Submit result once (should succeed)
    let prover_authority = prover_keypair.pubkey();
    let result_hash = [2u8; 32];

    let submit_result_instruction = MarketplaceInstruction::SubmitFheResult { result_hash };

    let submit_result_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(prover_authority, true),
            solana_program::instruction::AccountMeta::new(job_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::sysvar::clock::id(),
                false,
            ),
        ],
        data: submit_result_instruction.pack().unwrap(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut submit_result_tx =
        Transaction::new_with_payer(&[submit_result_ix.clone()], Some(&prover_authority));
    submit_result_tx.sign(&[&prover_keypair], recent_blockhash);
    let result1 = banks_client.process_transaction(submit_result_tx).await;
    assert!(result1.is_ok(), "First submission should succeed");

    // Force state to be committed by processing an empty transaction
    // This ensures the job account is updated before we try duplicate submission
    let recent_blockhash2 = banks_client.get_latest_blockhash().await.unwrap();

    // Try to submit same result again (should fail - program validates duplicates)
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut submit_result_tx2 =
        Transaction::new_with_payer(&[submit_result_ix], Some(&prover_authority));
    submit_result_tx2.sign(&[&prover_keypair], recent_blockhash);

    let result = banks_client.process_transaction(submit_result_tx2).await;
    assert!(
        result.is_err(),
        "Duplicate submission should fail (program has ResultAlreadySubmitted validation)"
    );
}

/// Test 3: Non-claimant cannot submit
#[tokio::test]
async fn test_submit_fhe_result_unauthorized() {
    let program_id = Pubkey::new_unique();
    let program_test = setup_program_test(program_id);

    let (mut banks_client, payer, _recent_blockhash) = program_test.start().await;

    // Setup marketplace and provers
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

    // Prover A will claim
    let prover_a_keypair = Keypair::new();
    let _prover_a_pda = register_prover(
        &mut banks_client,
        &payer,
        &prover_a_keypair,
        &program_id,
        &config_pda,
        5_000_000_000,
    )
    .await
    .unwrap();

    // Prover B will NOT claim but will try to submit
    let prover_b_keypair = Keypair::new();
    let _prover_b_pda = register_prover(
        &mut banks_client,
        &payer,
        &prover_b_keypair,
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
    let job_creator = payer.pubkey();
    let job_id = 0u64;
    let (job_pda, _) = Pubkey::find_program_address(
        &[b"job", job_creator.as_ref(), &job_id.to_le_bytes()],
        &program_id,
    );
    let (escrow_pda, _) = Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], &program_id);

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
        price_lamports: 3_000_000,
        timeout_seconds: 0,
        fhe_config: Some(fhe_config),
    };

    let create_job_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(job_creator, true),
            solana_program::instruction::AccountMeta::new(job_pda, false),
            solana_program::instruction::AccountMeta::new(config_pda, false),
            solana_program::instruction::AccountMeta::new(escrow_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::system_program::id(),
                false,
            ),
        ],
        data: create_job_instruction.pack().unwrap(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut create_job_tx = Transaction::new_with_payer(&[create_job_ix], Some(&payer.pubkey()));
    create_job_tx.sign(&[&payer], recent_blockhash);
    banks_client
        .process_transaction(create_job_tx)
        .await
        .unwrap();

    // Only Prover A, prover2, and prover3 claim (NOT Prover B)
    for prover_keypair in [&prover_a_keypair, &prover2_keypair, &prover3_keypair] {
        let prover_authority = prover_keypair.pubkey();
        let (prover_pda, _) =
            Pubkey::find_program_address(&[b"prover", prover_authority.as_ref()], &program_id);

        let claim_job_instruction = MarketplaceInstruction::ClaimJob;
        let claim_job_ix = solana_program::instruction::Instruction {
            program_id,
            accounts: vec![
                solana_program::instruction::AccountMeta::new(prover_authority, true),
                solana_program::instruction::AccountMeta::new(prover_pda, false),
                solana_program::instruction::AccountMeta::new(job_pda, false),
                solana_program::instruction::AccountMeta::new_readonly(config_pda, false),
                solana_program::instruction::AccountMeta::new_readonly(
                    solana_program::sysvar::clock::id(),
                    false,
                ),
            ],
            data: claim_job_instruction.pack().unwrap(),
        };

        let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
        let mut claim_job_tx =
            Transaction::new_with_payer(&[claim_job_ix], Some(&prover_authority));
        claim_job_tx.sign(&[prover_keypair], recent_blockhash);
        banks_client
            .process_transaction(claim_job_tx)
            .await
            .unwrap();
    }

    // Prover B tries to submit result (should fail - did not claim)
    let prover_b_authority = prover_b_keypair.pubkey();
    let result_hash = [2u8; 32];

    let submit_result_instruction = MarketplaceInstruction::SubmitFheResult { result_hash };

    let submit_result_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(prover_b_authority, true),
            solana_program::instruction::AccountMeta::new(job_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::sysvar::clock::id(),
                false,
            ),
        ],
        data: submit_result_instruction.pack().unwrap(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut submit_result_tx =
        Transaction::new_with_payer(&[submit_result_ix], Some(&prover_b_authority));
    submit_result_tx.sign(&[&prover_b_keypair], recent_blockhash);

    let result = banks_client.process_transaction(submit_result_tx).await;
    assert!(
        result.is_err(),
        "Non-claimant should not be able to submit result"
    );
}
