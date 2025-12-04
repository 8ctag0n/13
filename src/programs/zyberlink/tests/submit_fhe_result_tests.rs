mod common;

use borsh::BorshDeserialize;
use common::{initialize_marketplace, register_prover, setup_program_test};
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use zyberlink::{
    instruction::MarketplaceInstruction,
    state::{FheConsensusData, JobAccount},
};
use zyberlink_types::{fhe::FheOperation, CircuitType, FheConsensusConfig, JobStatus};

/// Helper to create an FHE job with proper accounts
async fn create_fhe_job(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    program_id: &Pubkey,
    config_pda: &Pubkey,
    job_id: u64,
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
        price_lamports: 3_000_000,
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
            // FHE jobs need the fhe_consensus account
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
) {
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
            // FHE jobs need the fhe_consensus account for multi-prover claiming
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
}

/// Helper to submit FHE result
async fn submit_fhe_result(
    banks_client: &mut BanksClient,
    prover_keypair: &Keypair,
    program_id: &Pubkey,
    job_pda: &Pubkey,
    fhe_consensus_pda: &Pubkey,
    result_hash: [u8; 32],
) -> Result<(), solana_program_test::BanksClientError> {
    let prover_authority = prover_keypair.pubkey();

    let submit_result_instruction = MarketplaceInstruction::SubmitFheResult { result_hash };

    let submit_result_ix = solana_program::instruction::Instruction {
        program_id: *program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(prover_authority, true),
            solana_program::instruction::AccountMeta::new(*job_pda, false),
            // FHE results are stored in FheConsensusData
            solana_program::instruction::AccountMeta::new(*fhe_consensus_pda, false),
        ],
        data: submit_result_instruction.pack().unwrap(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut submit_result_tx =
        Transaction::new_with_payer(&[submit_result_ix], Some(&prover_authority));
    submit_result_tx.sign(&[prover_keypair], recent_blockhash);

    banks_client.process_transaction(submit_result_tx).await
}

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

    // 3. Create FHE job
    let job_id = 0u64;
    let (job_pda, _escrow_pda, fhe_consensus_pda) =
        create_fhe_job(&mut banks_client, &payer, &program_id, &config_pda, job_id).await;

    // 4. Claim job with all 3 provers
    for prover_keypair in [&prover1_keypair, &prover2_keypair, &prover3_keypair] {
        claim_fhe_job(
            &mut banks_client,
            prover_keypair,
            &program_id,
            &config_pda,
            &job_pda,
            &fhe_consensus_pda,
        )
        .await;
    }

    // 5. Submit FHE result from prover1
    let result_hash = [2u8; 32];
    let result = submit_fhe_result(
        &mut banks_client,
        &prover1_keypair,
        &program_id,
        &job_pda,
        &fhe_consensus_pda,
        result_hash,
    )
    .await;

    assert!(
        result.is_ok(),
        "SubmitFheResult should succeed: {:?}",
        result.err()
    );

    // 6. Verify result stored in FheConsensusData
    let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
    let job: JobAccount = {
        let mut data_slice = &job_account.data[..];
        JobAccount::deserialize(&mut data_slice).unwrap()
    };

    // Job should still be in Claimed status
    assert_eq!(job.status, JobStatus::Claimed);

    // Results are now in FheConsensusData
    let fhe_account = banks_client
        .get_account(fhe_consensus_pda)
        .await
        .unwrap()
        .unwrap();
    let fhe_data: FheConsensusData = {
        let mut data_slice = &fhe_account.data[..];
        FheConsensusData::deserialize(&mut data_slice).unwrap()
    };

    assert_eq!(fhe_data.results_count, 1);
    assert!(fhe_data.result_submitted[0]);
    assert_eq!(fhe_data.result_hashes[0], result_hash);
}

/// Test 2: Submit duplicate result fails
#[tokio::test]
async fn test_submit_fhe_result_duplicate() {
    let program_id = Pubkey::new_unique();
    let program_test = setup_program_test(program_id);

    let (mut banks_client, payer, _recent_blockhash) = program_test.start().await;

    // Setup marketplace
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
    let job_id = 0u64;
    let (job_pda, _escrow_pda, fhe_consensus_pda) =
        create_fhe_job(&mut banks_client, &payer, &program_id, &config_pda, job_id).await;

    // Claim job with all 3 provers
    for pk in [&prover_keypair, &prover2_keypair, &prover3_keypair] {
        claim_fhe_job(
            &mut banks_client,
            pk,
            &program_id,
            &config_pda,
            &job_pda,
            &fhe_consensus_pda,
        )
        .await;
    }

    // Submit result once (should succeed)
    let result_hash = [2u8; 32];

    // Build first transaction
    let prover_authority = prover_keypair.pubkey();
    let submit_result_instruction = MarketplaceInstruction::SubmitFheResult { result_hash };
    let submit_result_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(prover_authority, true),
            solana_program::instruction::AccountMeta::new(job_pda, false),
            solana_program::instruction::AccountMeta::new(fhe_consensus_pda, false),
        ],
        data: submit_result_instruction.pack().unwrap(),
    };

    let blockhash1 = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx1 = Transaction::new_with_payer(&[submit_result_ix.clone()], Some(&prover_authority));
    tx1.sign(&[&prover_keypair], blockhash1);

    let result1 = banks_client.process_transaction(tx1).await;
    assert!(result1.is_ok(), "First submission should succeed");

    // Verify the result was stored in FheConsensusData
    let fhe_account = banks_client
        .get_account(fhe_consensus_pda)
        .await
        .unwrap()
        .unwrap();
    let fhe_data: FheConsensusData = {
        let mut data_slice = &fhe_account.data[..];
        FheConsensusData::deserialize(&mut data_slice).unwrap()
    };
    assert!(
        fhe_data.result_submitted[0],
        "Result should be marked as submitted"
    );
    assert_eq!(fhe_data.results_count, 1, "Results count should be 1");

    // Get NEW blockhash for second transaction and retry a few times
    // BanksClient can be flaky with state propagation
    let mut result2_is_err = false;
    for _ in 0..5 {
        let blockhash2 = banks_client.get_latest_blockhash().await.unwrap();
        let mut tx2 =
            Transaction::new_with_payer(&[submit_result_ix.clone()], Some(&prover_authority));
        tx2.sign(&[&prover_keypair], blockhash2);

        let result2 = banks_client.process_transaction(tx2).await;
        if result2.is_err() {
            result2_is_err = true;
            break;
        }
        // Small delay to let state propagate
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    assert!(
        result2_is_err,
        "Duplicate submission should fail (program has ResultAlreadySubmitted validation)"
    );
}

/// Test 3: Non-claimant cannot submit
#[tokio::test]
async fn test_submit_fhe_result_unauthorized() {
    let program_id = Pubkey::new_unique();
    let program_test = setup_program_test(program_id);

    let (mut banks_client, payer, _recent_blockhash) = program_test.start().await;

    // Setup marketplace
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
    let job_id = 0u64;
    let (job_pda, _escrow_pda, fhe_consensus_pda) =
        create_fhe_job(&mut banks_client, &payer, &program_id, &config_pda, job_id).await;

    // Only Prover A, prover2, and prover3 claim (NOT Prover B)
    for pk in [&prover_a_keypair, &prover2_keypair, &prover3_keypair] {
        claim_fhe_job(
            &mut banks_client,
            pk,
            &program_id,
            &config_pda,
            &job_pda,
            &fhe_consensus_pda,
        )
        .await;
    }

    // Prover B tries to submit result (should fail - did not claim)
    let result_hash = [2u8; 32];
    let result = submit_fhe_result(
        &mut banks_client,
        &prover_b_keypair,
        &program_id,
        &job_pda,
        &fhe_consensus_pda,
        result_hash,
    )
    .await;

    assert!(
        result.is_err(),
        "Non-claimant should not be able to submit result"
    );
}
