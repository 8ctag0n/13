//! CancelJob instruction processor for FHE jobs

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use zyberlink_types::JobStatus;

use crate::{
    error::FheGeneratorError,
    state::{FheConsensusData, FheJob, FHE_CONSENSUS_SEED},
};

/// Seeds for escrow PDA
const ESCROW_SEED: &[u8] = b"fhe_escrow";

/// Process CancelJob instruction for FHE jobs
///
/// Can only cancel jobs in Pending status (not yet claimed or partially claimed
/// but no provers have submitted results yet).
///
/// Accounts:
/// 0. `[signer]` Job creator wallet
/// 1. `[writable]` FheJob account
/// 2. `[writable]` FheConsensusData account
/// 3. `[writable]` Escrow PDA
pub fn process_cancel_job(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let creator_info = next_account_info(account_info_iter)?;
    let job_info = next_account_info(account_info_iter)?;
    let consensus_info = next_account_info(account_info_iter)?;
    let escrow_info = next_account_info(account_info_iter)?;

    // Verify creator is signer
    if !creator_info.is_signer {
        msg!("Creator must be a signer");
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

    // Load FHE job (use deserialize_reader to handle account padding)
    let job_data = job_info.data.borrow();
    let mut fhe_job: FheJob = BorshDeserialize::deserialize_reader(&mut &job_data[..])?;
    drop(job_data);

    // Verify creator matches
    if fhe_job.common.creator != *creator_info.key {
        msg!("Only job creator can cancel the job");
        return Err(FheGeneratorError::Unauthorized.into());
    }

    // Verify job is in Pending status
    if fhe_job.common.status != JobStatus::Pending {
        msg!("Can only cancel jobs in Pending status");
        return Err(FheGeneratorError::JobNotPending.into());
    }

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
    let consensus_data: FheConsensusData =
        BorshDeserialize::deserialize_reader(&mut &consensus_data_ref[..])?;
    drop(consensus_data_ref);

    // Don't allow cancellation if any prover has already submitted results
    if consensus_data.results_count > 0 {
        msg!(
            "Cannot cancel: {} provers have already submitted results",
            consensus_data.results_count
        );
        return Err(FheGeneratorError::InvalidJobStatus.into());
    }

    // Verify escrow PDA
    let (escrow_pda, _) =
        Pubkey::find_program_address(&[ESCROW_SEED, job_info.key.as_ref()], program_id);

    if escrow_info.key != &escrow_pda {
        msg!("Invalid escrow account");
        return Err(FheGeneratorError::InvalidEscrow.into());
    }

    // Verify escrow is owned by this program
    if escrow_info.owner != program_id {
        msg!("Escrow not owned by program");
        return Err(FheGeneratorError::InvalidEscrow.into());
    }

    // Get escrow balance
    let escrow_balance = escrow_info.lamports();

    msg!("Cancelling FHE job and refunding {} lamports", escrow_balance);

    // Transfer all escrow funds back to creator
    **escrow_info.try_borrow_mut_lamports()? = 0;
    **creator_info.try_borrow_mut_lamports()? = creator_info
        .lamports()
        .checked_add(escrow_balance)
        .ok_or(FheGeneratorError::Overflow)?;

    // Cancel the job
    fhe_job.common.cancel();

    // Save updated job
    let mut job_data = job_info.try_borrow_mut_data()?;
    fhe_job.serialize(&mut &mut job_data[..])?;

    msg!("FHE job cancelled successfully");
    msg!("  Job ID: {}", fhe_job.common.id);
    msg!("  Refunded: {} lamports", escrow_balance);
    if consensus_data.claimed_count > 0 {
        msg!(
            "  Note: {} provers had claimed but not submitted",
            consensus_data.claimed_count
        );
    }

    Ok(())
}
