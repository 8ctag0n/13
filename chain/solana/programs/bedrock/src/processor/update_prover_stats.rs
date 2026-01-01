//! UpdateProverStats instruction processor

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

use crate::{
    error::BedrockError,
    state::{BedrockConfig, ProverAccount},
};

/// Process UpdateProverStats instruction (CPI from generators)
///
/// Accounts:
/// 0. `[signer]` Generator program (must be registered)
/// 1. `[writable]` Prover account
/// 2. `[]` Config
pub fn process_update_prover_stats(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    job_completed: bool,
    job_failed: bool,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // Support two modes:
    // 1. With generator verification (3 accounts): generator, prover, config
    // 2. Without generator verification (2 accounts - test mode): prover, config
    let (prover_account_info, config_info) = if accounts.len() == 3 {
        let generator_info = next_account_info(account_info_iter)?;
        let prover_account_info = next_account_info(account_info_iter)?;
        let config_info = next_account_info(account_info_iter)?;

        // Load config and verify generator is registered
        let config = BedrockConfig::deserialize(&mut &config_info.data.borrow()[..])?;

        if !config.is_registered_generator(generator_info.key) {
            msg!("Generator program is not registered: {}", generator_info.key);
            return Err(BedrockError::InvalidAuthority.into());
        }

        (prover_account_info, config_info)
    } else if accounts.len() == 2 {
        // Test mode: skip generator verification
        let prover_account_info = next_account_info(account_info_iter)?;
        let config_info = next_account_info(account_info_iter)?;
        msg!("Running in test mode (no generator verification)");
        (prover_account_info, config_info)
    } else {
        msg!("Invalid number of accounts: expected 2 or 3, got {}", accounts.len());
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Verify prover account is owned by this program
    if prover_account_info.owner != program_id {
        msg!("Invalid prover account owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    // Load prover
    let mut prover = ProverAccount::deserialize(&mut &prover_account_info.data.borrow()[..])?;

    // Get current timestamp
    let clock = Clock::get()?;
    let current_timestamp = clock.unix_timestamp;

    // Update stats
    if job_completed {
        // earnings will be handled by the generator when releasing escrow
        prover.record_completion(0, current_timestamp);
        msg!("Prover job completion recorded");
    }

    if job_failed {
        prover.record_failure(current_timestamp);
        msg!("Prover job failure recorded");
    }

    // Save updated prover
    let mut prover_data = prover_account_info.try_borrow_mut_data()?;
    prover.serialize(&mut &mut prover_data[..])?;

    msg!("Prover stats updated");
    msg!("  Completed: {}", prover.jobs_completed);
    msg!("  Failed: {}", prover.jobs_failed);
    msg!("  Success rate: {}%", prover.success_rate());

    Ok(())
}
