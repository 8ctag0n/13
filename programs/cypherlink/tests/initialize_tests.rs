mod common;

use common::setup_program_test;
use cypherlink::{instruction::MarketplaceInstruction, state::MarketplaceConfig};
use solana_program::pubkey::Pubkey;
use solana_sdk::{signature::Signer, transaction::Transaction};

#[tokio::test]
async fn test_initialize_marketplace() {
    let program_id = Pubkey::new_unique();
    let program_test = setup_program_test(program_id);

    let (banks_client, payer, recent_blockhash) = program_test.start().await;

    let (config_pda, _bump) = Pubkey::find_program_address(&[b"config"], &program_id);

    let instruction_data = MarketplaceInstruction::Initialize {
        fee_basis_points: 1000,
        min_stake_amount: 5_000_000_000,
        min_reputation_score: 500,
        default_job_timeout_seconds: 600,
    };

    let instruction = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(payer.pubkey(), true),
            solana_program::instruction::AccountMeta::new(config_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::system_program::id(),
                false,
            ),
        ],
        data: instruction_data.pack().unwrap(),
    };

    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);

    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_ok(), "Initialize failed: {:?}", result.err());

    let config_account = banks_client.get_account(config_pda).await.unwrap();
    assert!(config_account.is_some(), "Config account not created");

    let config_account = config_account.unwrap();
    let config: MarketplaceConfig = borsh::from_slice(&config_account.data).unwrap();

    assert_eq!(config.authority, payer.pubkey());
    assert_eq!(config.fee_basis_points, 1000);
    assert_eq!(config.min_stake_amount, 5_000_000_000);
    assert_eq!(config.min_reputation_score, 500);
    assert_eq!(config.default_job_timeout_seconds, 600);
    assert_eq!(config.total_provers, 0);
    assert_eq!(config.total_jobs_created, 0);
}

#[tokio::test]
async fn test_initialize_already_initialized() {
    let program_id = Pubkey::new_unique();
    let program_test = setup_program_test(program_id);

    let (banks_client, payer, recent_blockhash) = program_test.start().await;
    let (config_pda, _bump) = Pubkey::find_program_address(&[b"config"], &program_id);

    let instruction_data = MarketplaceInstruction::Initialize {
        fee_basis_points: 1000,
        min_stake_amount: 5_000_000_000,
        min_reputation_score: 500,
        default_job_timeout_seconds: 600,
    };

    let instruction = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(payer.pubkey(), true),
            solana_program::instruction::AccountMeta::new(config_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::system_program::id(),
                false,
            ),
        ],
        data: instruction_data.pack().unwrap(),
    };

    let mut transaction =
        Transaction::new_with_payer(&[instruction.clone()], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    let config_account = banks_client.get_account(config_pda).await.unwrap();
    assert!(
        config_account.is_some(),
        "Config account should exist after first init"
    );

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();

    let instruction_data2 = MarketplaceInstruction::Initialize {
        fee_basis_points: 2000,
        min_stake_amount: 10_000_000_000,
        min_reputation_score: 600,
        default_job_timeout_seconds: 300,
    };

    let instruction2 = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(payer.pubkey(), true),
            solana_program::instruction::AccountMeta::new(config_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::system_program::id(),
                false,
            ),
        ],
        data: instruction_data2.pack().unwrap(),
    };

    let mut transaction2 = Transaction::new_with_payer(&[instruction2], Some(&payer.pubkey()));
    transaction2.sign(&[&payer], recent_blockhash);

    let result = banks_client.process_transaction(transaction2).await;
    assert!(result.is_err(), "Second initialization should fail");
}

#[tokio::test]
async fn test_initialize_missing_signer() {
    let program_id = Pubkey::new_unique();
    let program_test = setup_program_test(program_id);

    let (banks_client, payer, recent_blockhash) = program_test.start().await;
    let (config_pda, _bump) = Pubkey::find_program_address(&[b"config"], &program_id);

    let non_signer = solana_sdk::signature::Keypair::new();

    let instruction_data = MarketplaceInstruction::Initialize {
        fee_basis_points: 1000,
        min_stake_amount: 5_000_000_000,
        min_reputation_score: 500,
        default_job_timeout_seconds: 600,
    };

    let instruction = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(non_signer.pubkey(), false),
            solana_program::instruction::AccountMeta::new(config_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::system_program::id(),
                false,
            ),
        ],
        data: instruction_data.pack().unwrap(),
    };

    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);

    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_err(), "Initialize should fail without signer");
}

#[tokio::test]
async fn test_initialize_invalid_config_pda() {
    let program_id = Pubkey::new_unique();
    let program_test = setup_program_test(program_id);

    let (banks_client, payer, recent_blockhash) = program_test.start().await;

    let wrong_pda = Pubkey::new_unique();

    let instruction_data = MarketplaceInstruction::Initialize {
        fee_basis_points: 1000,
        min_stake_amount: 5_000_000_000,
        min_reputation_score: 500,
        default_job_timeout_seconds: 600,
    };

    let instruction = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(payer.pubkey(), true),
            solana_program::instruction::AccountMeta::new(wrong_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::system_program::id(),
                false,
            ),
        ],
        data: instruction_data.pack().unwrap(),
    };

    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);

    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_err(), "Initialize should fail with invalid PDA");
}
