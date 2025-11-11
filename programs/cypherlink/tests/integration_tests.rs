use borsh::BorshSerialize;
use cypherlink::{instruction::MarketplaceInstruction, state::MarketplaceConfig, state::ProverAccount};
use solana_program::{pubkey::Pubkey, system_instruction};
use solana_program_test::{processor, ProgramTest};
use solana_sdk::{
    account::Account, signature::Keypair, signer::Signer, transaction::Transaction,
};

#[tokio::test]
async fn test_initialize_marketplace() {
    let program_id = Pubkey::new_unique();
    let mut program_test = ProgramTest::new(
        "cypherlink",
        program_id,
        processor!(cypherlink::process_instruction),
    );

    // Start test environment
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // Derive config PDA
    let (config_pda, _bump) = Pubkey::find_program_address(&[b"config"], &program_id);

    // Build Initialize instruction
    let instruction_data = MarketplaceInstruction::Initialize {
        fee_basis_points: 1000,      // 10%
        min_stake_amount: 5_000_000_000, // 5 SOL
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

    // Execute transaction
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);

    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_ok(), "Initialize failed: {:?}", result.err());

    // Verify config account was created
    let config_account = banks_client.get_account(config_pda).await.unwrap();
    assert!(config_account.is_some(), "Config account not created");

    let config_account = config_account.unwrap();
    let config: MarketplaceConfig = borsh::from_slice(&config_account.data).unwrap();

    // Verify config values
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
    let mut program_test = ProgramTest::new(
        "cypherlink",
        program_id,
        processor!(cypherlink::process_instruction),
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    let (config_pda, _bump) = Pubkey::find_program_address(&[b"config"], &program_id);

    // First initialization
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

    let mut transaction = Transaction::new_with_payer(&[instruction.clone()], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Verify config account exists and has correct state
    let config_account = banks_client.get_account(config_pda).await.unwrap();
    assert!(config_account.is_some(), "Config account should exist after first init");

    // Get a fresh blockhash for the second transaction
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();

    // Try to initialize again with different parameters - should still fail
    let instruction_data2 = MarketplaceInstruction::Initialize {
        fee_basis_points: 2000,  // Different parameters
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
async fn test_register_prover() {
    let program_id = Pubkey::new_unique();
    let mut program_test = ProgramTest::new(
        "cypherlink",
        program_id,
        processor!(cypherlink::process_instruction),
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // First, initialize marketplace
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &program_id);

    let init_instruction = MarketplaceInstruction::Initialize {
        fee_basis_points: 1000,
        min_stake_amount: 5_000_000_000,
        min_reputation_score: 500,
        default_job_timeout_seconds: 600,
    };

    let init_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(payer.pubkey(), true),
            solana_program::instruction::AccountMeta::new(config_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::system_program::id(),
                false,
            ),
        ],
        data: init_instruction.pack().unwrap(),
    };

    let mut init_tx = Transaction::new_with_payer(&[init_ix], Some(&payer.pubkey()));
    init_tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(init_tx).await.unwrap();

    // Now register a prover
    let prover_keypair = Keypair::new();
    let prover_authority = prover_keypair.pubkey();

    // Fund the prover authority
    let fund_instruction = system_instruction::transfer(
        &payer.pubkey(),
        &prover_authority,
        10_000_000_000, // 10 SOL for stake + fees
    );
    let mut fund_tx = Transaction::new_with_payer(&[fund_instruction], Some(&payer.pubkey()));
    fund_tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(fund_tx).await.unwrap();

    let (prover_pda, _bump) = Pubkey::find_program_address(
        &[b"prover", prover_authority.as_ref()],
        &program_id,
    );

    let register_instruction = MarketplaceInstruction::RegisterProver {
        stake_amount: 5_000_000_000, // 5 SOL
    };

    let register_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(prover_authority, true),
            solana_program::instruction::AccountMeta::new(prover_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(config_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::system_program::id(),
                false,
            ),
        ],
        data: register_instruction.pack().unwrap(),
    };

    let mut register_tx = Transaction::new_with_payer(&[register_ix], Some(&prover_authority));
    register_tx.sign(&[&prover_keypair], recent_blockhash);

    let result = banks_client.process_transaction(register_tx).await;
    assert!(result.is_ok(), "RegisterProver failed: {:?}", result.err());

    // Verify prover account was created
    let prover_account = banks_client.get_account(prover_pda).await.unwrap();
    assert!(prover_account.is_some(), "Prover account not created");

    let prover_account = prover_account.unwrap();
    let prover: ProverAccount = borsh::from_slice(&prover_account.data).unwrap();

    // Verify prover values
    assert_eq!(prover.authority, prover_authority);
    assert_eq!(prover.stake_amount, 5_000_000_000);
    assert_eq!(prover.reputation_score, 1000); // Initial reputation
    assert_eq!(prover.total_jobs_completed, 0);
    assert_eq!(prover.total_jobs_failed, 0);
    assert!(prover.is_active);
}

#[tokio::test]
async fn test_register_prover_insufficient_stake() {
    let program_id = Pubkey::new_unique();
    let mut program_test = ProgramTest::new(
        "cypherlink",
        program_id,
        processor!(cypherlink::process_instruction),
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // Initialize marketplace
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &program_id);

    let init_instruction = MarketplaceInstruction::Initialize {
        fee_basis_points: 1000,
        min_stake_amount: 5_000_000_000, // Require 5 SOL
        min_reputation_score: 500,
        default_job_timeout_seconds: 600,
    };

    let init_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(payer.pubkey(), true),
            solana_program::instruction::AccountMeta::new(config_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::system_program::id(),
                false,
            ),
        ],
        data: init_instruction.pack().unwrap(),
    };

    let mut init_tx = Transaction::new_with_payer(&[init_ix], Some(&payer.pubkey()));
    init_tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(init_tx).await.unwrap();

    // Try to register with insufficient stake
    let prover_keypair = Keypair::new();
    let prover_authority = prover_keypair.pubkey();

    let fund_instruction = system_instruction::transfer(
        &payer.pubkey(),
        &prover_authority,
        3_000_000_000, // Only 3 SOL
    );
    let mut fund_tx = Transaction::new_with_payer(&[fund_instruction], Some(&payer.pubkey()));
    fund_tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(fund_tx).await.unwrap();

    let (prover_pda, _) = Pubkey::find_program_address(
        &[b"prover", prover_authority.as_ref()],
        &program_id,
    );

    let register_instruction = MarketplaceInstruction::RegisterProver {
        stake_amount: 3_000_000_000, // Less than required 5 SOL
    };

    let register_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(prover_authority, true),
            solana_program::instruction::AccountMeta::new(prover_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(config_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::system_program::id(),
                false,
            ),
        ],
        data: register_instruction.pack().unwrap(),
    };

    let mut register_tx = Transaction::new_with_payer(&[register_ix], Some(&prover_authority));
    register_tx.sign(&[&prover_keypair], recent_blockhash);

    let result = banks_client.process_transaction(register_tx).await;
    assert!(result.is_err(), "RegisterProver should fail with insufficient stake");
}

#[tokio::test]
async fn test_initialize_missing_signer() {
    let program_id = Pubkey::new_unique();
    let mut program_test = ProgramTest::new(
        "cypherlink",
        program_id,
        processor!(cypherlink::process_instruction),
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    let (config_pda, _bump) = Pubkey::find_program_address(&[b"config"], &program_id);

    // Use a different keypair as authority (not the payer)
    let non_signer = Keypair::new();

    let instruction_data = MarketplaceInstruction::Initialize {
        fee_basis_points: 1000,
        min_stake_amount: 5_000_000_000,
        min_reputation_score: 500,
        default_job_timeout_seconds: 600,
    };

    // Create instruction with non-signer as authority
    let instruction = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(non_signer.pubkey(), false), // Not a signer!
            solana_program::instruction::AccountMeta::new(config_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::system_program::id(),
                false,
            ),
        ],
        data: instruction_data.pack().unwrap(),
    };

    // Sign with payer only (not with non_signer)
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);

    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_err(), "Initialize should fail without signer");
}

#[tokio::test]
async fn test_initialize_invalid_config_pda() {
    let program_id = Pubkey::new_unique();
    let mut program_test = ProgramTest::new(
        "cypherlink",
        program_id,
        processor!(cypherlink::process_instruction),
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // Use wrong PDA (not derived from "config" seed)
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

#[tokio::test]
async fn test_register_prover_marketplace_not_initialized() {
    let program_id = Pubkey::new_unique();
    let mut program_test = ProgramTest::new(
        "cypherlink",
        program_id,
        processor!(cypherlink::process_instruction),
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // Try to register prover WITHOUT initializing marketplace first
    let prover_keypair = Keypair::new();
    let prover_authority = prover_keypair.pubkey();

    let fund_instruction = system_instruction::transfer(
        &payer.pubkey(),
        &prover_authority,
        10_000_000_000,
    );
    let mut fund_tx = Transaction::new_with_payer(&[fund_instruction], Some(&payer.pubkey()));
    fund_tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(fund_tx).await.unwrap();

    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &program_id);
    let (prover_pda, _) = Pubkey::find_program_address(
        &[b"prover", prover_authority.as_ref()],
        &program_id,
    );

    let register_instruction = MarketplaceInstruction::RegisterProver {
        stake_amount: 5_000_000_000,
    };

    let register_ix = solana_program::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(prover_authority, true),
            solana_program::instruction::AccountMeta::new(prover_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(config_pda, false),
            solana_program::instruction::AccountMeta::new_readonly(
                solana_program::system_program::id(),
                false,
            ),
        ],
        data: register_instruction.pack().unwrap(),
    };

    let mut register_tx = Transaction::new_with_payer(&[register_ix], Some(&prover_authority));
    register_tx.sign(&[&prover_keypair], recent_blockhash);

    let result = banks_client.process_transaction(register_tx).await;
    assert!(result.is_err(), "RegisterProver should fail when marketplace not initialized");
}
