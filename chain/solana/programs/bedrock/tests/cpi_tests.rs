//! CPI Integration Tests for Bedrock
//!
//! These tests verify that:
//! 1. CPI instruction builders create correct instructions
//! 2. Bedrock validates caller properly
//! 3. Prover stats are updated correctly

use bedrock::{
    derive_config_pda, derive_prover_pda, slash_prover_instruction,
    update_prover_stats_instruction, BedrockConfig, BedrockInstruction, ProverAccount,
    SlashReason,
};
use borsh::BorshDeserialize;
use solana_program::pubkey::Pubkey;

/// Test that update_prover_stats_instruction creates correct instruction
#[test]
fn test_update_prover_stats_instruction_builder() {
    let bedrock_id = Pubkey::new_unique();
    let generator_id = Pubkey::new_unique();
    let prover_pda = Pubkey::new_unique();
    let config_pda = Pubkey::new_unique();

    let ix = update_prover_stats_instruction(
        &bedrock_id,
        &generator_id,
        &prover_pda,
        &config_pda,
        true,  // job_completed
        false, // job_failed
    );

    // Verify instruction program_id
    assert_eq!(ix.program_id, bedrock_id);

    // Verify accounts
    assert_eq!(ix.accounts.len(), 3);
    assert_eq!(ix.accounts[0].pubkey, generator_id);
    assert!(ix.accounts[0].is_signer);
    assert!(!ix.accounts[0].is_writable);

    assert_eq!(ix.accounts[1].pubkey, prover_pda);
    assert!(!ix.accounts[1].is_signer);
    assert!(ix.accounts[1].is_writable);

    assert_eq!(ix.accounts[2].pubkey, config_pda);
    assert!(!ix.accounts[2].is_signer);
    assert!(!ix.accounts[2].is_writable);

    // Verify instruction data deserializes correctly
    let decoded: BedrockInstruction = BorshDeserialize::try_from_slice(&ix.data).unwrap();
    match decoded {
        BedrockInstruction::UpdateProverStats {
            job_completed,
            job_failed,
        } => {
            assert!(job_completed);
            assert!(!job_failed);
        }
        _ => panic!("Wrong instruction variant"),
    }
}

/// Test that slash_prover_instruction creates correct instruction
#[test]
fn test_slash_prover_instruction_builder() {
    let bedrock_id = Pubkey::new_unique();
    let generator_id = Pubkey::new_unique();
    let prover_pda = Pubkey::new_unique();
    let config_pda = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    let ix = slash_prover_instruction(
        &bedrock_id,
        &generator_id,
        &prover_pda,
        &config_pda,
        &recipient,
        1_000_000,
        SlashReason::ConsensusMismatch,
    );

    // Verify instruction program_id
    assert_eq!(ix.program_id, bedrock_id);

    // Verify accounts
    assert_eq!(ix.accounts.len(), 4);
    assert_eq!(ix.accounts[0].pubkey, generator_id);
    assert!(ix.accounts[0].is_signer);

    assert_eq!(ix.accounts[3].pubkey, recipient);
    assert!(ix.accounts[3].is_writable);

    // Verify instruction data
    let decoded: BedrockInstruction = BorshDeserialize::try_from_slice(&ix.data).unwrap();
    match decoded {
        BedrockInstruction::SlashProver { amount, reason } => {
            assert_eq!(amount, 1_000_000);
            assert!(matches!(reason, SlashReason::ConsensusMismatch));
        }
        _ => panic!("Wrong instruction variant"),
    }
}

/// Test PDA derivation helpers
#[test]
fn test_pda_derivation() {
    let bedrock_id = Pubkey::new_unique();
    let prover_wallet = Pubkey::new_unique();

    // Config PDA
    let (config_pda, config_bump) = derive_config_pda(&bedrock_id);
    let (expected_config, expected_bump) =
        Pubkey::find_program_address(&[b"config"], &bedrock_id);
    assert_eq!(config_pda, expected_config);
    assert_eq!(config_bump, expected_bump);

    // Prover PDA
    let (prover_pda, prover_bump) = derive_prover_pda(&bedrock_id, &prover_wallet);
    let (expected_prover, expected_prover_bump) =
        Pubkey::find_program_address(&[b"prover", prover_wallet.as_ref()], &bedrock_id);
    assert_eq!(prover_pda, expected_prover);
    assert_eq!(prover_bump, expected_prover_bump);
}

/// Test BedrockConfig is_registered_generator
#[test]
fn test_config_is_registered_generator() {
    let admin = Pubkey::new_unique();
    let zk_gen = Pubkey::new_unique();
    let fhe_gen = Pubkey::new_unique();
    let random = Pubkey::new_unique();

    let config = BedrockConfig::new(admin, zk_gen, fhe_gen, 255);

    assert!(config.is_registered_generator(&zk_gen));
    assert!(config.is_registered_generator(&fhe_gen));
    assert!(!config.is_registered_generator(&admin)); // admin is not a generator
    assert!(!config.is_registered_generator(&random));
}

/// Test ProverAccount record_completion
#[test]
fn test_prover_record_completion() {
    let authority = Pubkey::new_unique();
    let mut prover = ProverAccount::new(authority, 100_000_000, 1000, 255);

    assert_eq!(prover.jobs_completed, 0);
    assert_eq!(prover.total_earnings, 0);

    prover.record_completion(50_000_000, 2000);

    assert_eq!(prover.jobs_completed, 1);
    assert_eq!(prover.total_earnings, 50_000_000);
    assert_eq!(prover.last_active_at, 2000);
}

/// Test ProverAccount record_failure
#[test]
fn test_prover_record_failure() {
    let authority = Pubkey::new_unique();
    let mut prover = ProverAccount::new(authority, 100_000_000, 1000, 255);

    assert_eq!(prover.jobs_failed, 0);

    prover.record_failure(2000);

    assert_eq!(prover.jobs_failed, 1);
    assert_eq!(prover.last_active_at, 2000);
}

/// Test ProverAccount slash
#[test]
fn test_prover_slash() {
    let authority = Pubkey::new_unique();
    let mut prover = ProverAccount::new(authority, 150_000_000, 1000, 255); // 0.15 SOL

    assert!(prover.is_active);

    // Slash 50M lamports (should still be active)
    let slashed = prover.slash(50_000_000);
    assert_eq!(slashed, 50_000_000);
    assert_eq!(prover.stake_lamports, 100_000_000);
    assert!(prover.is_active);

    // Slash 60M lamports (drops below MIN_STAKE, becomes inactive)
    let slashed = prover.slash(60_000_000);
    assert_eq!(slashed, 60_000_000);
    assert_eq!(prover.stake_lamports, 40_000_000);
    assert!(!prover.is_active);

    // Slash more than remaining
    let slashed = prover.slash(100_000_000);
    assert_eq!(slashed, 40_000_000); // only slashes what's available
    assert_eq!(prover.stake_lamports, 0);
}

/// Test ProverAccount success_rate
#[test]
fn test_prover_success_rate() {
    let authority = Pubkey::new_unique();
    let mut prover = ProverAccount::new(authority, 100_000_000, 1000, 255);

    // No jobs = 100% success rate
    assert_eq!(prover.success_rate(), 100);

    // 1 completion, 0 failures = 100%
    prover.jobs_completed = 1;
    assert_eq!(prover.success_rate(), 100);

    // 1 completion, 1 failure = 50%
    prover.jobs_failed = 1;
    assert_eq!(prover.success_rate(), 50);

    // 3 completions, 1 failure = 75%
    prover.jobs_completed = 3;
    assert_eq!(prover.success_rate(), 75);
}

/// Test ProverAccount can_take_jobs
#[test]
fn test_prover_can_take_jobs() {
    let authority = Pubkey::new_unique();
    let mut prover = ProverAccount::new(authority, 100_000_000, 1000, 255);

    assert!(prover.can_take_jobs());

    // Deactivate
    prover.is_active = false;
    assert!(!prover.can_take_jobs());

    // Reactivate but low stake
    prover.is_active = true;
    prover.stake_lamports = 50_000_000; // below MIN_STAKE
    assert!(!prover.can_take_jobs());
}
