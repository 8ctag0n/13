mod common;

use borsh::BorshDeserialize;
use common::{initialize_marketplace, register_prover, setup_program_test};
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use zyberlink::{instruction::MarketplaceInstruction, state::JobAccount};
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
async fn test_cancel_job() {
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

    // Get creator balance before cancel
    let creator_balance_before = banks_client
        .get_account(payer.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Get escrow balance
    let escrow_balance = banks_client
        .get_account(escrow_pda)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Cancel the job
    let cancel_job_instruction = MarketplaceInstruction::CancelJob;

    let cancel_job_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(payer.pubkey(), true),
            solana_program::instruction::AccountMeta::new(job_pda, false),
            solana_program::instruction::AccountMeta::new(escrow_pda, false),
        ],
        data: cancel_job_instruction.pack().unwrap(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut cancel_job_tx = Transaction::new_with_payer(&[cancel_job_ix], Some(&payer.pubkey()));
    cancel_job_tx.sign(&[&payer], recent_blockhash);

    let result = banks_client.process_transaction(cancel_job_tx).await;
    assert!(result.is_ok(), "CancelJob failed: {:?}", result.err());

    // Verify job status
    let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
    let job: JobAccount = {
        let mut data_slice = &job_account.data[..];
        JobAccount::deserialize(&mut data_slice).unwrap()
    };

    assert_eq!(job.status, JobStatus::Cancelled);

    // Verify creator got refund
    let creator_balance_after = banks_client
        .get_account(payer.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Account for transaction fees (should get back escrow balance minus fees)
    let refund = creator_balance_after.saturating_sub(creator_balance_before);
    assert!(
        refund >= escrow_balance - 10_000,
        "Creator should receive refund, expected ~{}, got {}",
        escrow_balance,
        refund
    );

    // Verify escrow is empty (account should be deleted when lamports = 0)
    let escrow_account_after = banks_client.get_account(escrow_pda).await.unwrap();
    assert!(
        escrow_account_after.is_none() || escrow_account_after.unwrap().lamports == 0,
        "Escrow should be empty or deleted"
    );
}

#[tokio::test]
async fn test_cancel_job_already_claimed() {
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
    banks_client
        .process_transaction(claim_job_tx)
        .await
        .unwrap();

    // Try to cancel claimed job - should fail
    let cancel_job_instruction = MarketplaceInstruction::CancelJob;

    let cancel_job_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(payer.pubkey(), true),
            solana_program::instruction::AccountMeta::new(job_pda, false),
            solana_program::instruction::AccountMeta::new(escrow_pda, false),
        ],
        data: cancel_job_instruction.pack().unwrap(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut cancel_job_tx = Transaction::new_with_payer(&[cancel_job_ix], Some(&payer.pubkey()));
    cancel_job_tx.sign(&[&payer], recent_blockhash);

    let result = banks_client.process_transaction(cancel_job_tx).await;
    assert!(result.is_err(), "Should not be able to cancel claimed job");
}

#[tokio::test]
async fn test_cancel_job_wrong_creator() {
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

    // Create a job
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

    // Try to cancel with different account (not creator)
    let fake_creator = Keypair::new();

    // Fund fake creator
    let fund_instruction = solana_program::system_instruction::transfer(
        &payer.pubkey(),
        &fake_creator.pubkey(),
        10_000_000,
    );
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut fund_tx = Transaction::new_with_payer(&[fund_instruction], Some(&payer.pubkey()));
    fund_tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(fund_tx).await.unwrap();

    let cancel_job_instruction = MarketplaceInstruction::CancelJob;

    let cancel_job_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(fake_creator.pubkey(), true),
            solana_program::instruction::AccountMeta::new(job_pda, false),
            solana_program::instruction::AccountMeta::new(escrow_pda, false),
        ],
        data: cancel_job_instruction.pack().unwrap(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut cancel_job_tx =
        Transaction::new_with_payer(&[cancel_job_ix], Some(&fake_creator.pubkey()));
    cancel_job_tx.sign(&[&fake_creator], recent_blockhash);

    let result = banks_client.process_transaction(cancel_job_tx).await;
    assert!(result.is_err(), "Only creator should be able to cancel");
}
