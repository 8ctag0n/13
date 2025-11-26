use borsh::BorshDeserialize;
use zyberlink_types::JobStatus;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
};

use crate::{
    error::ZyberLinkProgramError,
    state::{FheConsensusData, JobAccount},
};

/// Process SubmitFheResult instruction
///
/// Provers submit their FHE computation results to the FheConsensusData account.
/// The JobAccount itself is only read to verify the job is valid.
pub fn process_submit_fhe_result(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    result_hash: [u8; 32],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let prover_authority_info = next_account_info(account_info_iter)?;
    let job_info = next_account_info(account_info_iter)?;
    let fhe_consensus_info = next_account_info(account_info_iter)?;

    // Prover must sign
    if !prover_authority_info.is_signer {
        msg!("Prover authority must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Verify job account is owned by program
    if job_info.owner != program_id {
        msg!("Invalid job account owner");
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Deserialize job
    let job: JobAccount = {
        let mut data_slice = &job_info.data.borrow()[..];
        JobAccount::deserialize(&mut data_slice)?
    };

    // Validate: must be FHE job
    if !job.is_fhe() {
        msg!("Job is not an FHE job (circuit_type={})", job.circuit_type);
        return Err(ZyberLinkProgramError::NotFheJob.into());
    }

    // Verify FHE consensus PDA
    let job_id_bytes = job.id.to_le_bytes();
    let (fhe_pda, _) = Pubkey::find_program_address(
        &[b"fhe_consensus", &job_id_bytes],
        program_id,
    );

    if fhe_consensus_info.key != &fhe_pda {
        msg!("Invalid FHE consensus account");
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Load FHE consensus data (use deserialize to handle variable-size Option)
    let mut fhe_data: FheConsensusData = {
        let mut data_slice = &fhe_consensus_info.data.borrow()[..];
        FheConsensusData::deserialize(&mut data_slice)?
    };

    // Validate: job must be Claimed (all provers have claimed)
    if job.status != JobStatus::Claimed {
        msg!("Job must be in Claimed status, current: {:?}", job.status);
        return Err(ZyberLinkProgramError::InvalidJobStatus.into());
    }

    // Get current time
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    // Validate: not timed out
    if job.is_timed_out(current_time) {
        msg!("Job has timed out");
        return Err(ZyberLinkProgramError::JobTimedOut.into());
    }

    // Submit result to FHE consensus data
    match fhe_data.submit_result(prover_authority_info.key, result_hash) {
        Ok(()) => {
            msg!("FHE result submitted successfully");
        }
        Err(e) => {
            msg!("Failed to submit FHE result: {}", e);
            if e.contains("not claimed") {
                return Err(ZyberLinkProgramError::ProverNotClaimed.into());
            } else if e.contains("already submitted") {
                return Err(ZyberLinkProgramError::ResultAlreadySubmitted.into());
            }
            return Err(ZyberLinkProgramError::InvalidAccount.into());
        }
    }

    // Serialize FHE data back
    let mut fhe_account_data = fhe_consensus_info.try_borrow_mut_data()?;
    borsh::to_writer(&mut fhe_account_data[..], &fhe_data)?;

    msg!("FHE result submitted successfully");
    msg!("  Job ID: {}", job.id);
    msg!("  Prover: {}", prover_authority_info.key);
    msg!(
        "  Total results: {}/{}",
        fhe_data.results_count,
        fhe_data.required_provers
    );

    Ok(())
}
