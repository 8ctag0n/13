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

    let caller_info = next_account_info(account_info_iter)?;
    let prover_account_info = next_account_info(account_info_iter)?;
    let config_info = next_account_info(account_info_iter)?;

    // Verify caller is signer
    if !caller_info.is_signer {
        msg!("Caller must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load config and verify caller is a registered generator
    let config = BedrockConfig::try_from_slice(&config_info.data.borrow())?;

    if !config.is_registered_generator(caller_info.key) {
        msg!("Caller is not a registered generator program");
        return Err(BedrockError::InvalidAuthority.into());
    }

    // Verify prover account is owned by this program
    if prover_account_info.owner != program_id {
        msg!("Invalid prover account owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    // Load prover
    let mut prover = ProverAccount::try_from_slice(&prover_account_info.data.borrow())?;

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
