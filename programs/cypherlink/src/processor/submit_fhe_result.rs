use borsh::BorshDeserialize;
use cypherlink_types::{CircuitType, FheJobResult, JobStatus};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
};

use crate::{
    error::CypherLinkProgramError,
    state::JobAccount,
};

/// Process SubmitFheResult instruction
pub fn process_submit_fhe_result(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    result_hash: [u8; 32],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let prover_authority_info = next_account_info(account_info_iter)?;
    let job_info = next_account_info(account_info_iter)?;

    // Prover must sign
    if !prover_authority_info.is_signer {
        msg!("Prover authority must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Verify job account is owned by program
    if job_info.owner != program_id {
        msg!("Invalid job account owner");
        return Err(CypherLinkProgramError::InvalidAccount.into());
    }

    // Deserialize job
    let mut job: JobAccount = {
        let mut data_slice = &job_info.data.borrow()[..];
        JobAccount::deserialize(&mut data_slice)?
    };

    // Validate: must be FHE job
    if !matches!(job.circuit_type, CircuitType::FheComputation(_)) {
        msg!("Job is not an FHE job");
        return Err(CypherLinkProgramError::NotFheJob.into());
    }

    let config = job.fhe_config
        .as_ref()
        .ok_or(CypherLinkProgramError::MissingFheConfig)?;

    // Validate: job must be Claimed (all provers have claimed)
    if job.status != JobStatus::Claimed {
        msg!("Job must be in Claimed status, current: {:?}", job.status);
        return Err(CypherLinkProgramError::InvalidJobStatus.into());
    }

    // Validate: prover must have claimed this job
    if !job.claimed_provers.contains(prover_authority_info.key) {
        msg!("Prover did not claim this FHE job");
        return Err(CypherLinkProgramError::ProverNotClaimed.into());
    }

    // Validate: prover hasn't already submitted
    if job.fhe_results.iter().any(|r| r.prover == *prover_authority_info.key) {
        msg!("Prover already submitted result");
        return Err(CypherLinkProgramError::ResultAlreadySubmitted.into());
    }

    // Validate: not exceeded max provers
    if job.fhe_results.len() >= 10 {
        msg!("Max provers (10) reached");
        return Err(CypherLinkProgramError::FheJobFullyClaimed.into());
    }

    // Get current time
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    // Validate: not timed out
    if job.is_timed_out(current_time) {
        msg!("Job has timed out");
        return Err(CypherLinkProgramError::JobTimedOut.into());
    }

    // Create result
    let result = FheJobResult::new(*prover_authority_info.key, result_hash, current_time);
    job.fhe_results.push(result);

    // Serialize back
    let mut job_data = job_info.try_borrow_mut_data()?;
    borsh::to_writer(&mut job_data[..], &job)?;

    msg!("FHE result submitted successfully");
    msg!("  Job ID: {}", job.id);
    msg!("  Prover: {}", prover_authority_info.key);
    msg!("  Total results: {}/{}", job.fhe_results.len(), config.required_provers);

    Ok(())
}
