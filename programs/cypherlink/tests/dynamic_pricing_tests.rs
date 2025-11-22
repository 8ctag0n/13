mod common;

use borsh::BorshDeserialize;
use common::{initialize_marketplace, setup_program_test};
use cypherlink::{
    instruction::MarketplaceInstruction,
    state::{JobAccount, MarketplaceConfig},
};
use cypherlink_types::{CircuitType, FheConsensusConfig, fhe::FheOperation};
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::{signature::Signer, transaction::Transaction};

/// Test that Tier 1 operation (Add) with sufficient pricing succeeds
#[tokio::test]
async fn test_tier1_add_sufficient_price() {
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

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], &program_id);

    // Tier 1 operation: Add
    // Min price: 0.001 SOL per prover × 3 provers = 0.003 SOL = 3_000_000 lamports
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
        price_lamports: 3_000_000, // Exactly minimum
        timeout_seconds: 0, // Use dynamic timeout
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
    let mut create_job_tx =
        Transaction::new_with_payer(&[create_job_ix], Some(&payer.pubkey()));
    create_job_tx.sign(&[&payer], recent_blockhash);

    let result = banks_client.process_transaction(create_job_tx).await;
    assert!(
        result.is_ok(),
        "CreateJob with sufficient price should succeed: {:?}",
        result.err()
    );

    // Verify job was created
    let job_account = banks_client.get_account(job_pda).await.unwrap();
    assert!(job_account.is_some(), "Job account not created");

    let job_account = job_account.unwrap();
    let job: JobAccount = {
        let mut data_slice = &job_account.data[..];
        JobAccount::deserialize(&mut data_slice).unwrap()
    };

    assert_eq!(job.price_lamports, 3_000_000);
    // Note: timeout is not directly accessible in JobAccount struct
}

/// Test that Tier 1 operation with insufficient pricing fails
#[tokio::test]
async fn test_tier1_add_insufficient_price() {
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

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], &program_id);

    // Tier 1 operation: Add
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
        price_lamports: 2_000_000, // INSUFFICIENT! Need 3_000_000
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
    let mut create_job_tx =
        Transaction::new_with_payer(&[create_job_ix], Some(&payer.pubkey()));
    create_job_tx.sign(&[&payer], recent_blockhash);

    let result = banks_client.process_transaction(create_job_tx).await;
    assert!(
        result.is_err(),
        "CreateJob with insufficient price should fail"
    );

    // Verify error is InvalidPrice
    // Note: In program_test, we can't easily extract the exact error,
    // but transaction should fail
}

/// Test that Tier 3 operation (Threshold) requires higher pricing
#[tokio::test]
async fn test_tier3_threshold_higher_price() {
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

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], &program_id);

    // Tier 3 operation: Threshold
    // Min price: 0.005 SOL per prover × 3 provers = 0.015 SOL = 15_000_000 lamports
    let fhe_operation = FheOperation::Threshold {
        threshold: 18,
        greater_or_equal: true,
    };
    let fhe_config = FheConsensusConfig {
        required_provers: 3,
        consensus_threshold: 2,
        submission_timeout_secs: 300,
        operation: fhe_operation.clone(),
    };

    let create_job_instruction = MarketplaceInstruction::CreateJob {
        circuit_type: CircuitType::FheComputation(fhe_operation),
        witness_commitment: [1u8; 32],
        witness_size: 2048,
        price_lamports: 15_000_000, // Sufficient for Tier 3
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
    let mut create_job_tx =
        Transaction::new_with_payer(&[create_job_ix], Some(&payer.pubkey()));
    create_job_tx.sign(&[&payer], recent_blockhash);

    let result = banks_client.process_transaction(create_job_tx).await;
    assert!(
        result.is_ok(),
        "CreateJob Tier 3 with sufficient price should succeed: {:?}",
        result.err()
    );

    // Verify job was created
    let job_account = banks_client.get_account(job_pda).await.unwrap().unwrap();
    let job: JobAccount = {
        let mut data_slice = &job_account.data[..];
        JobAccount::deserialize(&mut data_slice).unwrap()
    };

    assert_eq!(job.price_lamports, 15_000_000);
    // Timeout is set internally based on dynamic cost config
}

/// Test that pricing scales correctly with number of provers
#[tokio::test]
async fn test_pricing_scales_with_provers() {
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

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[b"escrow", job_pda.as_ref()], &program_id);

    // Same operation but with 5 provers instead of 3
    // Should require 5_000_000 lamports instead of 3_000_000
    let fhe_operation = FheOperation::Add(5);
    let fhe_config = FheConsensusConfig {
        required_provers: 5, // 5 provers!
        consensus_threshold: 3,
        submission_timeout_secs: 60,
        operation: fhe_operation.clone(),
    };

    let create_job_instruction = MarketplaceInstruction::CreateJob {
        circuit_type: CircuitType::FheComputation(fhe_operation),
        witness_commitment: [1u8; 32],
        witness_size: 2048,
        price_lamports: 5_000_000, // 5 provers × 0.001 SOL
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
    let mut create_job_tx =
        Transaction::new_with_payer(&[create_job_ix], Some(&payer.pubkey()));
    create_job_tx.sign(&[&payer], recent_blockhash);

    let result = banks_client.process_transaction(create_job_tx).await;
    assert!(
        result.is_ok(),
        "CreateJob with 5 provers should succeed: {:?}",
        result.err()
    );
}
