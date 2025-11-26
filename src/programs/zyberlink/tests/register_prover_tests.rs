mod common;

use common::{initialize_marketplace, setup_program_test};
use zyberlink::{instruction::MarketplaceInstruction, state::ProverAccount};
use solana_program::{pubkey::Pubkey, system_instruction};
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

#[tokio::test]
async fn test_register_prover() {
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
    let prover_authority = prover_keypair.pubkey();

    let fund_instruction =
        system_instruction::transfer(&payer.pubkey(), &prover_authority, 10_000_000_000);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut fund_tx = Transaction::new_with_payer(&[fund_instruction], Some(&payer.pubkey()));
    fund_tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(fund_tx).await.unwrap();

    let (prover_pda, _bump) =
        Pubkey::find_program_address(&[b"prover", prover_authority.as_ref()], &program_id);

    let register_instruction = MarketplaceInstruction::RegisterProver {
        stake_amount: 5_000_000_000,
        encryption_pubkey: [1u8; 32],
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

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut register_tx = Transaction::new_with_payer(&[register_ix], Some(&prover_authority));
    register_tx.sign(&[&prover_keypair], recent_blockhash);

    let result = banks_client.process_transaction(register_tx).await;
    assert!(result.is_ok(), "RegisterProver failed: {:?}", result.err());

    let prover_account = banks_client.get_account(prover_pda).await.unwrap();
    assert!(prover_account.is_some(), "Prover account not created");

    let prover_account = prover_account.unwrap();
    let prover: ProverAccount = borsh::from_slice(&prover_account.data).unwrap();

    assert_eq!(prover.authority, prover_authority);
    assert_eq!(prover.stake_amount, 5_000_000_000);
    assert_eq!(prover.reputation_score, 1000);
    assert_eq!(prover.total_jobs_completed, 0);
    assert_eq!(prover.total_jobs_failed, 0);
    assert!(prover.is_active);
}

#[tokio::test]
async fn test_register_prover_insufficient_stake() {
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
    let prover_authority = prover_keypair.pubkey();

    let fund_instruction =
        system_instruction::transfer(&payer.pubkey(), &prover_authority, 3_000_000_000);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut fund_tx = Transaction::new_with_payer(&[fund_instruction], Some(&payer.pubkey()));
    fund_tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(fund_tx).await.unwrap();

    let (prover_pda, _) =
        Pubkey::find_program_address(&[b"prover", prover_authority.as_ref()], &program_id);

    let register_instruction = MarketplaceInstruction::RegisterProver {
        stake_amount: 3_000_000_000,
        encryption_pubkey: [1u8; 32],
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

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut register_tx = Transaction::new_with_payer(&[register_ix], Some(&prover_authority));
    register_tx.sign(&[&prover_keypair], recent_blockhash);

    let result = banks_client.process_transaction(register_tx).await;
    assert!(
        result.is_err(),
        "RegisterProver should fail with insufficient stake"
    );
}

#[tokio::test]
async fn test_register_prover_marketplace_not_initialized() {
    let program_id = Pubkey::new_unique();
    let program_test = setup_program_test(program_id);

    let (banks_client, payer, _recent_blockhash) = program_test.start().await;

    let prover_keypair = Keypair::new();
    let prover_authority = prover_keypair.pubkey();

    let fund_instruction =
        system_instruction::transfer(&payer.pubkey(), &prover_authority, 10_000_000_000);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut fund_tx = Transaction::new_with_payer(&[fund_instruction], Some(&payer.pubkey()));
    fund_tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(fund_tx).await.unwrap();

    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &program_id);
    let (prover_pda, _) =
        Pubkey::find_program_address(&[b"prover", prover_authority.as_ref()], &program_id);

    let register_instruction = MarketplaceInstruction::RegisterProver {
        stake_amount: 5_000_000_000,
        encryption_pubkey: [1u8; 32],
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

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut register_tx = Transaction::new_with_payer(&[register_ix], Some(&prover_authority));
    register_tx.sign(&[&prover_keypair], recent_blockhash);

    let result = banks_client.process_transaction(register_tx).await;
    assert!(
        result.is_err(),
        "RegisterProver should fail when marketplace not initialized"
    );
}
