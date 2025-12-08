//! Integration tests using solana-program-test
//!
//! These tests simulate the full Bedrock program on a local validator

use bedrock::{
    process_instruction, BedrockConfig, BedrockInstruction, ProverAccount, CONFIG_SEED,
    PROVER_SEED,
};
use borsh::BorshDeserialize;
use solana_program::pubkey::Pubkey;
use solana_program_test::{processor, ProgramTest};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};

/// Create a ProgramTest instance with Bedrock loaded
fn program_test() -> ProgramTest {
    let mut pt =
        ProgramTest::new("bedrock", bedrock_program_id(), processor!(process_instruction));
    pt.prefer_bpf(false); // Use native for faster tests
    pt
}

/// Get bedrock program ID (deterministic for tests)
fn bedrock_program_id() -> Pubkey {
    Pubkey::new_from_array([
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
        0x1f, 0x20,
    ])
}

/// Helper to create Initialize instruction
fn create_initialize_ix(
    admin: &Pubkey,
    config_pda: &Pubkey,
    zk_gen: Pubkey,
    fhe_gen: Pubkey,
) -> Instruction {
    let data = borsh::to_vec(&BedrockInstruction::Initialize {
        zk_generator_program: zk_gen,
        fhe_generator_program: fhe_gen,
    })
    .unwrap();

    Instruction {
        program_id: bedrock_program_id(),
        accounts: vec![
            AccountMeta::new(*admin, true),
            AccountMeta::new(*config_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data,
    }
}

/// Helper to create RegisterProver instruction
fn create_register_prover_ix(
    prover_wallet: &Pubkey,
    prover_pda: &Pubkey,
    config_pda: &Pubkey,
    stake_lamports: u64,
) -> Instruction {
    let data = borsh::to_vec(&BedrockInstruction::RegisterProver { stake_lamports }).unwrap();

    Instruction {
        program_id: bedrock_program_id(),
        accounts: vec![
            AccountMeta::new(*prover_wallet, true),
            AccountMeta::new(*prover_pda, false),
            AccountMeta::new_readonly(*config_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data,
    }
}

#[tokio::test]
async fn test_initialize() {
    let mut pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = bedrock_program_id();
    let zk_gen = Pubkey::new_unique();
    let fhe_gen = Pubkey::new_unique();

    // Derive config PDA
    let (config_pda, _bump) = Pubkey::find_program_address(&[CONFIG_SEED], &program_id);

    // Create initialize instruction
    let ix = create_initialize_ix(&payer.pubkey(), &config_pda, zk_gen, fhe_gen);

    // Send transaction
    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);

    banks_client.process_transaction(tx).await.unwrap();

    // Verify config was created
    let config_account = banks_client.get_account(config_pda).await.unwrap().unwrap();
    let config = BedrockConfig::try_from_slice(&config_account.data).unwrap();

    assert_eq!(config.admin, payer.pubkey());
    assert_eq!(config.zk_generator_program, zk_gen);
    assert_eq!(config.fhe_generator_program, fhe_gen);
    assert!(config.is_initialized);
    assert_eq!(config.next_job_id, 1);
    assert_eq!(config.total_provers, 0);
}

#[tokio::test]
async fn test_register_prover() {
    let mut pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = bedrock_program_id();
    let zk_gen = Pubkey::new_unique();
    let fhe_gen = Pubkey::new_unique();

    // Derive PDAs
    let (config_pda, _) = Pubkey::find_program_address(&[CONFIG_SEED], &program_id);
    let (prover_pda, _) =
        Pubkey::find_program_address(&[PROVER_SEED, payer.pubkey().as_ref()], &program_id);

    // Initialize first
    let init_ix = create_initialize_ix(&payer.pubkey(), &config_pda, zk_gen, fhe_gen);
    let mut tx = Transaction::new_with_payer(&[init_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Register prover
    let stake = 100_000_000u64; // 0.1 SOL
    let register_ix = create_register_prover_ix(&payer.pubkey(), &prover_pda, &config_pda, stake);

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[register_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);

    banks_client.process_transaction(tx).await.unwrap();

    // Verify prover was created
    let prover_account = banks_client.get_account(prover_pda).await.unwrap().unwrap();
    // Use deserialize_reader to handle accounts with extra padding bytes
    let prover: ProverAccount =
        BorshDeserialize::deserialize(&mut &prover_account.data[..]).unwrap();

    assert_eq!(prover.authority, payer.pubkey());
    assert_eq!(prover.stake_lamports, stake);
    assert!(prover.is_active);
    assert_eq!(prover.jobs_completed, 0);
    assert_eq!(prover.jobs_failed, 0);
}

#[tokio::test]
async fn test_register_prover_insufficient_stake() {
    let mut pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = bedrock_program_id();
    let zk_gen = Pubkey::new_unique();
    let fhe_gen = Pubkey::new_unique();

    // Derive PDAs
    let (config_pda, _) = Pubkey::find_program_address(&[CONFIG_SEED], &program_id);
    let (prover_pda, _) =
        Pubkey::find_program_address(&[PROVER_SEED, payer.pubkey().as_ref()], &program_id);

    // Initialize first
    let init_ix = create_initialize_ix(&payer.pubkey(), &config_pda, zk_gen, fhe_gen);
    let mut tx = Transaction::new_with_payer(&[init_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Try to register with insufficient stake
    let stake = 50_000_000u64; // 0.05 SOL - below minimum
    let register_ix = create_register_prover_ix(&payer.pubkey(), &prover_pda, &config_pda, stake);

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[register_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);

    // Should fail
    let result = banks_client.process_transaction(tx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_update_prover_stats_unauthorized() {
    let mut pt = program_test();
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let program_id = bedrock_program_id();
    let zk_gen = Pubkey::new_unique();
    let fhe_gen = Pubkey::new_unique();

    // Derive PDAs
    let (config_pda, _) = Pubkey::find_program_address(&[CONFIG_SEED], &program_id);
    let (prover_pda, _) =
        Pubkey::find_program_address(&[PROVER_SEED, payer.pubkey().as_ref()], &program_id);

    // Initialize
    let init_ix = create_initialize_ix(&payer.pubkey(), &config_pda, zk_gen, fhe_gen);
    let mut tx = Transaction::new_with_payer(&[init_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Register prover
    let stake = 100_000_000u64;
    let register_ix = create_register_prover_ix(&payer.pubkey(), &prover_pda, &config_pda, stake);
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[register_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Try to update stats from unauthorized caller (payer, not a generator)
    let update_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new_readonly(payer.pubkey(), true), // unauthorized
            AccountMeta::new(prover_pda, false),
            AccountMeta::new_readonly(config_pda, false),
        ],
        data: borsh::to_vec(&BedrockInstruction::UpdateProverStats {
            job_completed: true,
            job_failed: false,
        })
        .unwrap(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[update_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);

    // Should fail - unauthorized caller
    let result = banks_client.process_transaction(tx).await;
    assert!(result.is_err());
}
