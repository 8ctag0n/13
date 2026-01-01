//! SubmitResult instruction processor for FHE jobs

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
use zyberlink_types::JobStatus;

use crate::{
    error::FheGeneratorError,
    state::{FheConsensusData, FheJob, FHE_CONSENSUS_SEED},
};

/// Process SubmitResult instruction for FHE jobs
///
/// Provers submit their FHE computation results to the FheConsensusData account.
/// Results are hashed for comparison during finalization.
///
/// Accounts:
/// 0. `[signer]` Prover wallet (must have claimed the job)
/// 1. `[]` FheJob account (read-only)
/// 2. `[writable]` FheConsensusData account
pub fn process_submit_result(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    result_hash: [u8; 32],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let prover_info = next_account_info(account_info_iter)?;
    let job_info = next_account_info(account_info_iter)?;
    let consensus_info = next_account_info(account_info_iter)?;

    // Verify prover is signer
    if !prover_info.is_signer {
        msg!("Prover must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Verify job account is owned by this program
    if job_info.owner != program_id {
        msg!("Invalid job account owner");
        return Err(FheGeneratorError::JobNotFound.into());
    }

    // Verify consensus account is owned by this program
    if consensus_info.owner != program_id {
        msg!("Invalid consensus account owner");
        return Err(FheGeneratorError::InvalidConsensus.into());
    }

    // Load FHE job (read-only, use deserialize_reader to handle account padding)
    let job_data = job_info.data.borrow();
    let fhe_job: FheJob = BorshDeserialize::deserialize_reader(&mut &job_data[..])?;
    drop(job_data);

    // Verify consensus PDA
    let job_id_bytes = fhe_job.common.id.to_le_bytes();
    let (consensus_pda, _) =
        Pubkey::find_program_address(&[FHE_CONSENSUS_SEED, &job_id_bytes], program_id);

    if consensus_info.key != &consensus_pda {
        msg!("Invalid FHE consensus account");
        return Err(FheGeneratorError::InvalidConsensus.into());
    }

    // Load consensus data (use deserialize_reader to handle account padding)
    let consensus_data_ref = consensus_info.data.borrow();
    let mut consensus_data: FheConsensusData =
        BorshDeserialize::deserialize_reader(&mut &consensus_data_ref[..])?;
    drop(consensus_data_ref);

    // Verify job is in valid status
    // Allow both Pending (partially claimed) and Claimed (fully claimed)
    // Provers can submit as soon as they finish processing
    if fhe_job.common.status != JobStatus::Pending && fhe_job.common.status != JobStatus::Claimed {
        msg!(
            "Job must be in Pending or Claimed status, current: {:?}",
            fhe_job.common.status
        );
        return Err(FheGeneratorError::InvalidJobStatus.into());
    }

    // Get current time
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    // Check if job timed out
    if current_time >= consensus_data.submission_timeout {
        msg!("Job submission period has expired");
        return Err(FheGeneratorError::JobExpired.into());
    }

    // Submit result to consensus data
    match consensus_data.submit_result(prover_info.key, result_hash) {
        Ok(()) => {
            msg!("FHE result submitted successfully");
        }
        Err(e) => {
            msg!("Failed to submit FHE result: {}", e);
            if e.contains("not claimed") {
                return Err(FheGeneratorError::ProverNotInJob.into());
            } else if e.contains("already submitted") {
                return Err(FheGeneratorError::AlreadySubmitted.into());
            }
            return Err(FheGeneratorError::InvalidInstruction.into());
        }
    }

    // Save updated consensus data
    let mut consensus_account_data = consensus_info.try_borrow_mut_data()?;
    consensus_data.serialize(&mut &mut consensus_account_data[..])?;

    msg!("FHE result submitted");
    msg!("  Job ID: {}", fhe_job.common.id);
    msg!("  Prover: {}", prover_info.key);
    msg!("  Result hash: {:?}", &result_hash[..8]);
    msg!(
        "  Total results: {}/{}",
        consensus_data.results_count,
        consensus_data.required_provers
    );

    Ok(())
}
