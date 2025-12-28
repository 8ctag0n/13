//! ClaimJob instruction processor for FHE jobs

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

/// Process ClaimJob instruction for FHE jobs
///
/// FHE jobs support multi-prover claiming via FheConsensusData.
/// Multiple provers can claim the same job until required_provers is reached.
///
/// Accounts:
/// 0. `[signer]` Prover wallet
/// 1. `[writable]` FheJob account
/// 2. `[writable]` FheConsensusData account
/// 3. `[]` Prover PDA (bedrock)
/// 4. `[]` Bedrock program
pub fn process_claim_job(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let prover_info = next_account_info(account_info_iter)?;
    let job_info = next_account_info(account_info_iter)?;
    let consensus_info = next_account_info(account_info_iter)?;

    // Verify prover is signer
    if !prover_info.is_signer {
        msg!("Prover must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Get bedrock accounts for prover verification
    let prover_pda_info = next_account_info(account_info_iter)?;
    let bedrock_program_info = next_account_info(account_info_iter)?;

    // Derive expected prover PDA
    let (expected_prover_pda, _) = bedrock::cpi::derive_prover_pda(
        bedrock_program_info.key,
        prover_info.key,
    );
    if prover_pda_info.key != &expected_prover_pda {
        msg!("Invalid prover PDA");
        return Err(FheGeneratorError::Unauthorized.into());
    }

    // Verify prover is active and can take jobs
    let is_registered = bedrock::cpi::verify_prover(
        bedrock_program_info.key,
        prover_pda_info,
    )?;
    if !is_registered {
        msg!("Prover not registered or not active in Bedrock");
        return Err(FheGeneratorError::Unauthorized.into());
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

    // Load FHE job (use deserialize_reader to handle account padding)
    let job_data = job_info.data.borrow();
    let mut fhe_job: FheJob = BorshDeserialize::deserialize_reader(&mut &job_data[..])?;
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

    // Verify job is in Pending status (FHE jobs stay Pending until fully claimed)
    if fhe_job.common.status != JobStatus::Pending {
        msg!("Job is not in Pending status");
        return Err(FheGeneratorError::JobNotPending.into());
    }

    // Get current time
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    // Check if job timed out
    if fhe_job.common.is_expired(current_time) {
        msg!("Job has timed out");
        return Err(FheGeneratorError::JobExpired.into());
    }

    // Check if already fully claimed
    if consensus_data.is_fully_claimed() {
        msg!("FHE job already fully claimed");
        return Err(FheGeneratorError::JobFullyClaimed.into());
    }

    // Add prover to consensus
    match consensus_data.add_prover(*prover_info.key) {
        Ok(idx) => {
            msg!("Prover added to FHE job at slot {}", idx);
        }
        Err(e) => {
            msg!("Failed to add prover: {}", e);
            if e.contains("already") {
                return Err(FheGeneratorError::ProverAlreadyClaimed.into());
            }
            return Err(FheGeneratorError::JobFullyClaimed.into());
        }
    }

    // If fully claimed after this addition, update job status
    if consensus_data.is_fully_claimed() {
        fhe_job.common.status = JobStatus::Claimed;
        // Set the first prover in common.prover for reference
        if fhe_job.common.prover.is_none() {
            fhe_job.common.prover = Some(consensus_data.claimed_provers[0]);
        }
        msg!(
            "FHE job fully claimed by {} provers",
            consensus_data.required_provers
        );
    } else {
        msg!(
            "FHE job partially claimed: {}/{}",
            consensus_data.claimed_count,
            consensus_data.required_provers
        );
    }

    // Save updated job
    let mut job_data = job_info.try_borrow_mut_data()?;
    fhe_job.serialize(&mut &mut job_data[..])?;

    // Save updated consensus data
    let mut consensus_account_data = consensus_info.try_borrow_mut_data()?;
    consensus_data.serialize(&mut &mut consensus_account_data[..])?;

    msg!("FHE job claim successful");
    msg!("  Job ID: {}", fhe_job.common.id);
    msg!("  Prover: {}", prover_info.key);
    msg!(
        "  Claimed: {}/{}",
        consensus_data.claimed_count,
        consensus_data.required_provers
    );

    Ok(())
}
