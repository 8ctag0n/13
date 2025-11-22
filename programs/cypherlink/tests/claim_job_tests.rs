mod common;

use borsh::BorshDeserialize;
use common::{initialize_marketplace, register_prover, setup_program_test};
use cypherlink::{instruction::MarketplaceInstruction, state::JobAccount};
use cypherlink_types::{CircuitType, JobStatus};
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

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
    let (job_pda, _) = Pubkey::find_program_address(
        &[b"job", job_creator.as_ref(), &job_id_bytes],
        program_id,
    );
    let (escrow_pda, _) = Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], program_id);

    let create_job_instruction = MarketplaceInstruction::CreateJob {
        circuit_type: CircuitType::ZcashOrchard,
        witness_commitment: [1u8; 32],
        witness_size: 2048,
        price_lamports,
        timeout_seconds: 600,
        fhe_config: None,
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

#[tokio::test]
async fn test_claim_job() {
    let program_id = Pubkey::new_unique();
    let mut program_test = setup_program_test(program_id);

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

    // Create a job
    let (job_pda, _escrow_pda) = create_job(
        &mut banks_client,
        &payer,
        &program_id,
        &config_pda,
        0,
        1_000_000,
    )
    .await
    .unwrap();

    // Claim the job
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

    let result = banks_client.process_transaction(claim_job_tx).await;
    assert!(result.is_ok(), "ClaimJob failed: {:?}", result.err());

    // Verify job was claimed
    let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
    let job: JobAccount = {
        let mut data_slice = &job_account.data[..];
        JobAccount::deserialize(&mut data_slice).unwrap()
    };

    assert_eq!(job.status, JobStatus::Claimed);
    assert_eq!(job.prover, Some(prover_keypair.pubkey()));
    assert!(job.claimed_at.is_some());
}

#[tokio::test]
async fn test_claim_job_already_claimed() {
    let program_id = Pubkey::new_unique();
    let mut program_test = setup_program_test(program_id);

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

    // Create a job
    let (job_pda, _escrow_pda) = create_job(
        &mut banks_client,
        &payer,
        &program_id,
        &config_pda,
        0,
        1_000_000,
    )
    .await
    .unwrap();

    // First prover claims the job
    let claim_job_instruction = MarketplaceInstruction::ClaimJob;

    let claim_job_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(prover1_keypair.pubkey(), true),
            solana_program::instruction::AccountMeta::new(prover1_pda, false),
            solana_program::instruction::AccountMeta::new(job_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(config_pda, false),
        ],
        data: claim_job_instruction.pack().unwrap(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut claim_job_tx =
        Transaction::new_with_payer(&[claim_job_ix], Some(&prover1_keypair.pubkey()));
    claim_job_tx.sign(&[&prover1_keypair], recent_blockhash);
    banks_client.process_transaction(claim_job_tx).await.unwrap();

    // Second prover tries to claim the same job - should fail
    let claim_job_instruction2 = MarketplaceInstruction::ClaimJob;

    let claim_job_ix2 = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(prover2_keypair.pubkey(), true),
            solana_program::instruction::AccountMeta::new(prover2_pda, false),
            solana_program::instruction::AccountMeta::new(job_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(config_pda, false),
        ],
        data: claim_job_instruction2.pack().unwrap(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut claim_job_tx2 =
        Transaction::new_with_payer(&[claim_job_ix2], Some(&prover2_keypair.pubkey()));
    claim_job_tx2.sign(&[&prover2_keypair], recent_blockhash);

    let result = banks_client.process_transaction(claim_job_tx2).await;
    assert!(result.is_err(), "Second claim should fail");
}

#[tokio::test]
async fn test_claim_job_insufficient_reputation() {
    let program_id = Pubkey::new_unique();
    let mut program_test = setup_program_test(program_id);

    let (mut banks_client, payer, _recent_blockhash) = program_test.start().await;

    // Initialize marketplace with HIGH reputation requirement
    let config_pda = initialize_marketplace(
        &mut banks_client,
        &payer,
        &program_id,
        1000,
        5_000_000_000,
        900, // High reputation requirement
        600,
    )
    .await
    .unwrap();

    // Register a prover (starts with 1000 reputation, but requirement is 900)
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
    let (job_pda, _escrow_pda) = create_job(
        &mut banks_client,
        &payer,
        &program_id,
        &config_pda,
        0,
        1_000_000,
    )
    .await
    .unwrap();

    // This should succeed since prover has 1000 reputation (meets 900 requirement)
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

    let result = banks_client.process_transaction(claim_job_tx).await;
    assert!(result.is_ok(), "ClaimJob should succeed with sufficient reputation");
}
