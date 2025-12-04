mod common;

use borsh::BorshDeserialize;
use common::{initialize_marketplace, register_prover, setup_program_test};
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use zyberlink::{
    instruction::MarketplaceInstruction, state::JobAccount, state::MarketplaceConfig,
    state::ProverAccount,
};
use zyberlink_types::{CircuitType, JobStatus};

/// Helper to create a job
async fn create_job(
    banks_client: &mut solana_program_test::BanksClient,
    payer: &Keypair,
    program_id: &Pubkey,
    config_pda: &Pubkey,
    job_id: u64,
    price_lamports: u64,
) -> Result<(Pubkey, Pubkey), Box<dyn std::error::Error>> {
    let job_creator = payer.pubkey();
    let job_id_bytes = job_id.to_le_bytes();
    let (job_pda, _) =
        Pubkey::find_program_address(&[b"job", job_creator.as_ref(), &job_id_bytes], program_id);
    let (escrow_pda, _) = Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], program_id);

    let create_job_instruction = MarketplaceInstruction::CreateJob {
        circuit_type: CircuitType::ZcashOrchard,
        witness_commitment: [1u8; 32],
        witness_size: 2048,
        price_lamports,
        timeout_seconds: 600,
        fhe_config: None, // ZK job, no FHE config
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
        ],
        data: create_job_instruction.pack()?,
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut create_job_tx = Transaction::new_with_payer(&[create_job_ix], Some(&payer.pubkey()));
    create_job_tx.sign(&[payer], recent_blockhash);
    banks_client.process_transaction(create_job_tx).await?;

    Ok((job_pda, escrow_pda))
}

/// Helper to claim a job
async fn claim_job(
    banks_client: &mut solana_program_test::BanksClient,
    prover_keypair: &Keypair,
    program_id: &Pubkey,
    prover_pda: &Pubkey,
    job_pda: &Pubkey,
    config_pda: &Pubkey,
) -> Result<(), Box<dyn std::error::Error>> {
    let claim_job_instruction = MarketplaceInstruction::ClaimJob;

    let claim_job_ix = solana_program::instruction::Instruction {
        program_id: *program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(prover_keypair.pubkey(), true),
            solana_program::instruction::AccountMeta::new(*prover_pda, false),
            solana_program::instruction::AccountMeta::new(*job_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(*config_pda, false),
        ],
        data: claim_job_instruction.pack()?,
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let mut claim_job_tx =
        Transaction::new_with_payer(&[claim_job_ix], Some(&prover_keypair.pubkey()));
    claim_job_tx.sign(&[prover_keypair], recent_blockhash);
    banks_client.process_transaction(claim_job_tx).await?;

    Ok(())
}

#[tokio::test]
async fn test_submit_proof() {
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

    // Create a job
    let price_lamports = 1_000_000u64;
    let (job_pda, escrow_pda) = create_job(
        &mut banks_client,
        &payer,
        &program_id,
        &config_pda,
        0,
        price_lamports,
    )
    .await
    .unwrap();

    // Claim the job
    claim_job(
        &mut banks_client,
        &prover_keypair,
        &program_id,
        &prover_pda,
        &job_pda,
        &config_pda,
    )
    .await
    .unwrap();

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

    // Submit proof
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

    let result = banks_client.process_transaction(submit_proof_tx).await;
    assert!(result.is_ok(), "SubmitProof failed: {:?}", result.err());

    // Verify job status
    let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
    let job: JobAccount = {
        let mut data_slice = &job_account.data[..];
        JobAccount::deserialize(&mut data_slice).unwrap()
    };

    assert_eq!(job.status, JobStatus::Completed);
    // proof_hash stores the proof commitment in the new structure
    assert_eq!(job.proof_hash, Some([2u8; 32]));

    // Verify prover got paid (should receive 90% of price)
    let expected_prover_payout = 900_000u64; // 90% of 1_000_000
    let prover_balance_after = banks_client
        .get_account(prover_keypair.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Account for transaction fees
    let prover_gain = prover_balance_after.saturating_sub(prover_balance_before);
    assert!(
        prover_gain >= expected_prover_payout - 10_000,
        "Prover should receive ~900k lamports, got {}",
        prover_gain
    );

    // Verify protocol got fee (10% of price)
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
    let config_data: MarketplaceConfig = borsh::from_slice(&config_account.data).unwrap();
    assert_eq!(config_data.total_jobs_completed, 1);
}

#[tokio::test]
async fn test_submit_proof_job_not_claimed() {
    let program_id = Pubkey::new_unique();
    let program_test = setup_program_test(program_id);

    let (mut banks_client, payer, _recent_blockhash) = program_test.start().await;

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

    // Create a job but DON'T claim it
    let (job_pda, escrow_pda) = create_job(
        &mut banks_client,
        &payer,
        &program_id,
        &config_pda,
        0,
        1_000_000,
    )
    .await
    .unwrap();

    let config = banks_client.get_account(config_pda).await.unwrap().unwrap();
    let config_data: MarketplaceConfig = borsh::from_slice(&config.data).unwrap();

    // Try to submit proof without claiming
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

    let result = banks_client.process_transaction(submit_proof_tx).await;
    assert!(result.is_err(), "SubmitProof should fail for unclaimed job");
}

#[tokio::test]
async fn test_submit_proof_wrong_prover() {
    let program_id = Pubkey::new_unique();
    let program_test = setup_program_test(program_id);

    let (mut banks_client, payer, _recent_blockhash) = program_test.start().await;

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

    // Register two provers
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
    let prover2_pda = register_prover(
        &mut banks_client,
        &payer,
        &prover2_keypair,
        &program_id,
        &config_pda,
        5_000_000_000,
    )
    .await
    .unwrap();

    // Create and claim job with prover1
    let (job_pda, escrow_pda) = create_job(
        &mut banks_client,
        &payer,
        &program_id,
        &config_pda,
        0,
        1_000_000,
    )
    .await
    .unwrap();

    claim_job(
        &mut banks_client,
        &prover1_keypair,
        &program_id,
        &prover1_pda,
        &job_pda,
        &config_pda,
    )
    .await
    .unwrap();

    let config = banks_client.get_account(config_pda).await.unwrap().unwrap();
    let config_data: MarketplaceConfig = borsh::from_slice(&config.data).unwrap();

    // Try to submit proof with prover2 (wrong prover)
    let submit_proof_instruction = MarketplaceInstruction::SubmitProof {
        proof_commitment: [2u8; 32],
        proof_size: 1536,
    };

    let submit_proof_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(prover2_keypair.pubkey(), true),
            solana_program::instruction::AccountMeta::new(prover2_pda, false),
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
        Transaction::new_with_payer(&[submit_proof_ix], Some(&prover2_keypair.pubkey()));
    submit_proof_tx.sign(&[&prover2_keypair], recent_blockhash);

    let result = banks_client.process_transaction(submit_proof_tx).await;
    assert!(
        result.is_err(),
        "SubmitProof should fail when wrong prover tries"
    );
}
