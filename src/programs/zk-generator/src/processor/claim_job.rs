//! ClaimJob instruction processor

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
use zyberlink_jobs::validation;

use crate::{error::ZkGeneratorError, state::ZkJob};

/// Process ClaimJob instruction
///
/// Accounts:
/// 0. `[signer]` Prover wallet
/// 1. `[writable]` ZkJob account
pub fn process_claim_job(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let prover_info = next_account_info(account_info_iter)?;
    let job_info = next_account_info(account_info_iter)?;

    // Verify prover is signer
    if !prover_info.is_signer {
        msg!("Prover must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Verify job account is owned by this program
    if job_info.owner != program_id {
        msg!("Invalid job account owner");
        return Err(ZkGeneratorError::JobNotFound.into());
    }

    // Load job (use deserialize_reader to handle account padding)
    let data = job_info.data.borrow();
    let mut zk_job: ZkJob = BorshDeserialize::deserialize_reader(&mut &data[..])?;
    drop(data); // Release borrow before write

    // Get current time
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    // Validate claim using shared library
    validation::validate_claim(&zk_job.common, prover_info.key, current_time)
        .map_err(|e| {
            msg!("Claim validation failed: {:?}", e);
            ZkGeneratorError::JobAlreadyClaimed
        })?;

    // Claim the job
    zk_job.common.claim(*prover_info.key);

    // Save updated job
    let mut job_data = job_info.try_borrow_mut_data()?;
    zk_job.serialize(&mut &mut job_data[..])?;

    msg!("ZK job claimed successfully");
    msg!("  Job ID: {}", zk_job.common.id);
    msg!("  Prover: {}", prover_info.key);
    msg!("  Timeout at: {}", zk_job.common.timeout_at);

    Ok(())
}
