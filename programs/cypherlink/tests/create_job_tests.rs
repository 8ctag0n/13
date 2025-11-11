mod common;

use borsh::BorshDeserialize;
use common::{initialize_marketplace, setup_program_test};
use cypherlink::{
    instruction::MarketplaceInstruction, state::JobAccount, state::MarketplaceConfig,
};
use cypherlink_types::CircuitType;
use solana_program::pubkey::Pubkey;
use solana_sdk::{signature::Signer, transaction::Transaction};

#[tokio::test]
async fn test_create_job() {
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

    let job_creator = payer.pubkey();
    let job_id = 0u64;
    let job_id_bytes = job_id.to_le_bytes();
    let (job_pda, _) = Pubkey::find_program_address(
        &[b"job", job_creator.as_ref(), &job_id_bytes],
        &program_id,
    );

    let (escrow_pda, _) = Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], &program_id);

    let create_job_instruction = MarketplaceInstruction::CreateJob {
        circuit_type: CircuitType::ZcashOrchard,
        witness_commitment: [1u8; 32],
        witness_size: 2048,
        price_lamports: 1_000_000,
        timeout_seconds: 300,
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

    let result = banks_client.process_transaction(create_job_tx).await;
    assert!(result.is_ok(), "CreateJob failed: {:?}", result.err());

    let job_account = banks_client.get_account(job_pda).await.unwrap();
    assert!(job_account.is_some(), "Job account not created");

    let job_account = job_account.unwrap();
    let job: JobAccount = {
        let mut data_slice = &job_account.data[..];
        JobAccount::deserialize(&mut data_slice).unwrap()
    };

    assert_eq!(job.id, 0);
    assert_eq!(job.creator, job_creator);
    assert_eq!(job.price_lamports, 1_000_000);
    assert_eq!(job.witness_size, 2048);
    assert_eq!(job.status, cypherlink_types::JobStatus::Pending);

    let escrow_account = banks_client.get_account(escrow_pda).await.unwrap();
    assert!(escrow_account.is_some(), "Escrow account not created");

    let escrow_account = escrow_account.unwrap();
    assert!(escrow_account.lamports >= 1_000_000, "Escrow should hold payment");
}

#[tokio::test]
async fn test_create_job_invalid_price() {
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

    let job_creator = payer.pubkey();
    let job_id = 0u64;
    let job_id_bytes = job_id.to_le_bytes();
    let (job_pda, _) = Pubkey::find_program_address(
        &[b"job", job_creator.as_ref(), &job_id_bytes],
        &program_id,
    );
    let (escrow_pda, _) = Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], &program_id);

    let create_job_instruction = MarketplaceInstruction::CreateJob {
        circuit_type: CircuitType::ZcashOrchard,
        witness_commitment: [1u8; 32],
        witness_size: 2048,
        price_lamports: 0,
        timeout_seconds: 300,
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

    let result = banks_client.process_transaction(create_job_tx).await;
    assert!(result.is_err(), "CreateJob should fail with zero price");
}

#[tokio::test]
async fn test_create_multiple_jobs() {
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

    for i in 0..3u64 {
        let job_creator = payer.pubkey();
        let job_id = i;
        let job_id_bytes = job_id.to_le_bytes();
        let (job_pda, _) = Pubkey::find_program_address(
            &[b"job", job_creator.as_ref(), &job_id_bytes],
            &program_id,
        );
        let (escrow_pda, _) = Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], &program_id);

        let create_job_instruction = MarketplaceInstruction::CreateJob {
            circuit_type: CircuitType::ZcashOrchard,
            witness_commitment: [i as u8; 32],
            witness_size: 2048 + (i as u32 * 100),
            price_lamports: 1_000_000 + (i * 100_000),
            timeout_seconds: 300,
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

        let result = banks_client.process_transaction(create_job_tx).await;
        assert!(result.is_ok(), "CreateJob {} failed: {:?}", i, result.err());

        let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
        let job: JobAccount = {
            let mut data_slice = &job_account.data[..];
            JobAccount::deserialize(&mut data_slice).unwrap()
        };
        assert_eq!(job.id, i);
    }

    let config_account = banks_client.get_account(config_pda).await.unwrap().unwrap();
    let config: MarketplaceConfig = borsh::from_slice(&config_account.data).unwrap();
    assert_eq!(config.next_job_id, 3);
    assert_eq!(config.total_jobs_created, 3);
}
