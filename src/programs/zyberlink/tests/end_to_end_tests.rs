mod common;

use borsh::BorshDeserialize;
use common::{initialize_marketplace, register_prover, setup_program_test};
use zyberlink::{
    instruction::MarketplaceInstruction, state::JobAccount, state::MarketplaceConfig,
    state::ProverAccount,
};
use zyberlink_types::{CircuitType, JobStatus};
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

#[tokio::test]
async fn test_full_job_lifecycle() {
    let program_id = Pubkey::new_unique();
    let program_test = setup_program_test(program_id);

    let (mut banks_client, payer, _recent_blockhash) = program_test.start().await;

    // Step 1: Initialize marketplace
    let config_pda = initialize_marketplace(
        &mut banks_client,
        &payer,
        &program_id,
        1000,          // 10% fee
        5_000_000_000, // 5 SOL minimum stake
        500,           // minimum reputation score
        600,           // default timeout: 10 minutes
    )
    .await
    .unwrap();

    // Verify marketplace initialized correctly
    let config_account = banks_client.get_account(config_pda).await.unwrap().unwrap();
    let config: MarketplaceConfig = borsh::from_slice(&config_account.data).unwrap();
    assert_eq!(config.authority, payer.pubkey());
    assert_eq!(config.fee_basis_points, 1000);
    assert_eq!(config.total_provers, 0);
    assert_eq!(config.total_jobs_created, 0);

    // Step 2: Register a prover
    let prover_keypair = Keypair::new();
    let prover_pda = register_prover(
        &mut banks_client,
        &payer,
        &prover_keypair,
        &program_id,
        &config_pda,
        5_000_000_000,
    )
    .await
    .unwrap();

    // Verify prover registered correctly
    let prover_account = banks_client.get_account(prover_pda).await.unwrap().unwrap();
    let prover_data: ProverAccount = borsh::from_slice(&prover_account.data).unwrap();
    assert_eq!(prover_data.authority, prover_keypair.pubkey());
    assert_eq!(prover_data.stake_amount, 5_000_000_000);
    assert_eq!(prover_data.total_jobs_completed, 0);
    assert_eq!(prover_data.reputation_score, 1000);

    // Step 3: Create a job
    let job_creator = payer.pubkey();
    let job_id = 0u64;
    let job_id_bytes = job_id.to_le_bytes();
    let (job_pda, _) =
        Pubkey::find_program_address(&[b"job", job_creator.as_ref(), &job_id_bytes], &program_id);
    let (escrow_pda, _) = Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], &program_id);

    let price_lamports = 1_000_000u64;
    let create_job_instruction = MarketplaceInstruction::CreateJob {
        circuit_type: CircuitType::ZcashOrchard,
        witness_commitment: [1u8; 32],
        witness_size: 2048,
        price_lamports,
        timeout_seconds: 600,
        fhe_config: None, // ZK job, no FHE config
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

    // Verify job created correctly
    let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
    let job: JobAccount = {
        let mut data_slice = &job_account.data[..];
        JobAccount::deserialize(&mut data_slice).unwrap()
    };
    assert_eq!(job.id, 0);
    assert_eq!(job.creator, job_creator);
    assert_eq!(job.price_lamports, price_lamports);
    assert_eq!(job.status, JobStatus::Pending);
    assert_eq!(job.prover, None);

    // Step 4: Prover claims the job
    let claim_job_instruction = MarketplaceInstruction::ClaimJob;

    let claim_job_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(prover_keypair.pubkey(), true),
            solana_program::instruction::AccountMeta::new(prover_pda, false),
            solana_program::instruction::AccountMeta::new(job_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(config_pda, false),
        ],
        data: claim_job_instruction.pack().unwrap(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut claim_job_tx =
        Transaction::new_with_payer(&[claim_job_ix], Some(&prover_keypair.pubkey()));
    claim_job_tx.sign(&[&prover_keypair], recent_blockhash);
    banks_client
        .process_transaction(claim_job_tx)
        .await
        .unwrap();

    // Verify job was claimed
    let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
    let job: JobAccount = {
        let mut data_slice = &job_account.data[..];
        JobAccount::deserialize(&mut data_slice).unwrap()
    };
    assert_eq!(job.status, JobStatus::Claimed);
    assert_eq!(job.prover, Some(prover_keypair.pubkey()));
    // claimed_at no longer exists in JobAccount (structure optimized)

    // Step 5: Prover submits proof
    // Get initial balances
    let prover_balance_before = banks_client
        .get_account(prover_keypair.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    let config = banks_client.get_account(config_pda).await.unwrap().unwrap();
    let config_data: MarketplaceConfig = borsh::from_slice(&config.data).unwrap();
    let protocol_fee_recipient = config_data.protocol_fee_recipient;

    let protocol_balance_before = banks_client
        .get_account(protocol_fee_recipient)
        .await
        .unwrap()
        .map(|a| a.lamports)
        .unwrap_or(0);

    let submit_proof_instruction = MarketplaceInstruction::SubmitProof {
        proof_commitment: [2u8; 32],
        proof_size: 1536,
    };

    let submit_proof_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(prover_keypair.pubkey(), true),
            solana_program::instruction::AccountMeta::new(prover_pda, false),
            solana_program::instruction::AccountMeta::new(job_pda, false),
            solana_program::instruction::AccountMeta::new(escrow_pda, false),
            solana_program::instruction::AccountMeta::new(payer.pubkey(), false),
            solana_program::instruction::AccountMeta::new(protocol_fee_recipient, false),
            solana_program::instruction::AccountMeta::new(config_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::system_program::id(),
                false,
            ),
        ],
        data: submit_proof_instruction.pack().unwrap(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut submit_proof_tx =
        Transaction::new_with_payer(&[submit_proof_ix], Some(&prover_keypair.pubkey()));
    submit_proof_tx.sign(&[&prover_keypair], recent_blockhash);
    banks_client
        .process_transaction(submit_proof_tx)
        .await
        .unwrap();

    // Verify job completed successfully
    let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
    let job: JobAccount = {
        let mut data_slice = &job_account.data[..];
        JobAccount::deserialize(&mut data_slice).unwrap()
    };
    assert_eq!(job.status, JobStatus::Completed);
    // proof_hash stores the proof commitment in the new structure
    assert_eq!(job.proof_hash, Some([2u8; 32]));

    // Verify payments were processed correctly
    // Prover should receive 90% of price (after 10% protocol fee)
    let expected_prover_payout = 900_000u64;
    let prover_balance_after = banks_client
        .get_account(prover_keypair.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    let prover_gain = prover_balance_after.saturating_sub(prover_balance_before);
    assert!(
        prover_gain >= expected_prover_payout - 10_000,
        "Prover should receive ~900k lamports, got {}",
        prover_gain
    );

    // Protocol should receive 10% fee
    let expected_protocol_fee = 100_000u64;
    let protocol_balance_after = banks_client
        .get_account(protocol_fee_recipient)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    let protocol_gain = protocol_balance_after - protocol_balance_before;
    assert_eq!(
        protocol_gain, expected_protocol_fee,
        "Protocol should receive exactly 100k lamports"
    );

    // Verify prover statistics updated
    let prover_account = banks_client.get_account(prover_pda).await.unwrap().unwrap();
    let prover_data: ProverAccount = borsh::from_slice(&prover_account.data).unwrap();
    assert_eq!(prover_data.total_jobs_completed, 1);

    // Verify marketplace statistics updated
    let config_account = banks_client.get_account(config_pda).await.unwrap().unwrap();
    let final_config: MarketplaceConfig = borsh::from_slice(&config_account.data).unwrap();
    assert_eq!(final_config.total_jobs_created, 1);
    assert_eq!(final_config.total_jobs_completed, 1);
}

#[tokio::test]
async fn test_multiple_jobs_lifecycle() {
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

    // Register a prover
    let prover_keypair = Keypair::new();
    let prover_pda = register_prover(
        &mut banks_client,
        &payer,
        &prover_keypair,
        &program_id,
        &config_pda,
        5_000_000_000,
    )
    .await
    .unwrap();

    // Process 3 jobs through complete lifecycle
    for i in 0..3u64 {
        // Create job
        let job_creator = payer.pubkey();
        let job_id_bytes = i.to_le_bytes();
        let (job_pda, _) = Pubkey::find_program_address(
            &[b"job", job_creator.as_ref(), &job_id_bytes],
            &program_id,
        );
        let (escrow_pda, _) =
            Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], &program_id);

        let create_job_instruction = MarketplaceInstruction::CreateJob {
            circuit_type: CircuitType::ZcashOrchard,
            witness_commitment: [i as u8; 32],
            witness_size: 2048,
            price_lamports: 1_000_000,
            timeout_seconds: 600,
            fhe_config: None, // ZK job, no FHE config
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
        let mut create_job_tx =
            Transaction::new_with_payer(&[create_job_ix], Some(&payer.pubkey()));
        create_job_tx.sign(&[&payer], recent_blockhash);
        banks_client
            .process_transaction(create_job_tx)
            .await
            .unwrap();

        // Claim job
        let claim_job_instruction = MarketplaceInstruction::ClaimJob;
        let claim_job_ix = solana_program::instruction::Instruction {
            program_id,
            accounts: vec![
                solana_program::instruction::AccountMeta::new(prover_keypair.pubkey(), true),
                solana_program::instruction::AccountMeta::new(prover_pda, false),
                solana_program::instruction::AccountMeta::new(job_pda, false),
                solana_program::instruction::AccountMeta::new_readonly(config_pda, false),
            ],
            data: claim_job_instruction.pack().unwrap(),
        };

        let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
        let mut claim_job_tx =
            Transaction::new_with_payer(&[claim_job_ix], Some(&prover_keypair.pubkey()));
        claim_job_tx.sign(&[&prover_keypair], recent_blockhash);
        banks_client
            .process_transaction(claim_job_tx)
            .await
            .unwrap();

        // Submit proof
        let config = banks_client.get_account(config_pda).await.unwrap().unwrap();
        let config_data: MarketplaceConfig = borsh::from_slice(&config.data).unwrap();

        let submit_proof_instruction = MarketplaceInstruction::SubmitProof {
            proof_commitment: [(i + 10) as u8; 32],
            proof_size: 1536,
        };

        let submit_proof_ix = solana_program::instruction::Instruction {
            program_id,
            accounts: vec![
                solana_program::instruction::AccountMeta::new(prover_keypair.pubkey(), true),
                solana_program::instruction::AccountMeta::new(prover_pda, false),
                solana_program::instruction::AccountMeta::new(job_pda, false),
                solana_program::instruction::AccountMeta::new(escrow_pda, false),
                solana_program::instruction::AccountMeta::new(payer.pubkey(), false),
                solana_program::instruction::AccountMeta::new(
                    config_data.protocol_fee_recipient,
                    false,
                ),
                solana_program::instruction::AccountMeta::new(config_pda, false),
                solana_program::instruction::AccountMeta::new_readonly(
                    solana_program::system_program::id(),
                    false,
                ),
            ],
            data: submit_proof_instruction.pack().unwrap(),
        };

        let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
        let mut submit_proof_tx =
            Transaction::new_with_payer(&[submit_proof_ix], Some(&prover_keypair.pubkey()));
        submit_proof_tx.sign(&[&prover_keypair], recent_blockhash);
        banks_client
            .process_transaction(submit_proof_tx)
            .await
            .unwrap();

        // Verify job completed
        let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
        let job: JobAccount = {
            let mut data_slice = &job_account.data[..];
            JobAccount::deserialize(&mut data_slice).unwrap()
        };
        assert_eq!(job.status, JobStatus::Completed);
    }

    // Verify final statistics
    let prover_account = banks_client.get_account(prover_pda).await.unwrap().unwrap();
    let prover_data: ProverAccount = borsh::from_slice(&prover_account.data).unwrap();
    assert_eq!(prover_data.total_jobs_completed, 3);

    let config_account = banks_client.get_account(config_pda).await.unwrap().unwrap();
    let final_config: MarketplaceConfig = borsh::from_slice(&config_account.data).unwrap();
    assert_eq!(final_config.total_jobs_created, 3);
    assert_eq!(final_config.total_jobs_completed, 3);
}
