//! CancelJob instruction processor

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use zyberlink_types::JobStatus;

use crate::{error::ZkGeneratorError, state::ZkJob};

/// Seeds for escrow PDA
const ESCROW_SEED: &[u8] = b"zk_escrow";

/// Process CancelJob instruction
///
/// Can only cancel jobs in Pending status (not yet claimed).
/// Refunds the full escrow amount back to the creator.
///
/// Accounts:
/// 0. `[signer]` Job creator wallet
/// 1. `[writable]` ZkJob account
/// 2. `[writable]` Escrow PDA
pub fn process_cancel_job(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let creator_info = next_account_info(account_info_iter)?;
    let job_info = next_account_info(account_info_iter)?;
    let escrow_info = next_account_info(account_info_iter)?;

    // Verify creator is signer
    if !creator_info.is_signer {
        msg!("Job creator must be a signer");
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

    // Verify creator matches
    if zk_job.common.creator != *creator_info.key {
        msg!("Only job creator can cancel the job");
        return Err(ZkGeneratorError::Unauthorized.into());
    }

    // Verify job is in Pending status (can only cancel unclaimed jobs)
    if zk_job.common.status != JobStatus::Pending {
        msg!("Can only cancel jobs in Pending status");
        return Err(ZkGeneratorError::JobNotPending.into());
    }

    // Verify escrow PDA
    let (escrow_pda, _escrow_bump) =
        Pubkey::find_program_address(&[ESCROW_SEED, job_info.key.as_ref()], program_id);

    if escrow_info.key != &escrow_pda {
        msg!("Invalid escrow account");
        return Err(ZkGeneratorError::InvalidEscrow.into());
    }

    // Verify escrow is owned by this program
    if escrow_info.owner != program_id {
        msg!("Escrow not owned by program");
        return Err(ZkGeneratorError::InvalidEscrow.into());
    }

    // Get escrow balance (should be price + rent)
    let escrow_balance = escrow_info.lamports();

    msg!("Cancelling job and refunding {} lamports", escrow_balance);

    // Transfer all escrow funds back to creator
    **escrow_info.try_borrow_mut_lamports()? = 0;
    **creator_info.try_borrow_mut_lamports()? = creator_info
        .lamports()
        .checked_add(escrow_balance)
        .ok_or(ZkGeneratorError::Overflow)?;

    // Cancel the job
    zk_job.common.cancel();

    // Save updated job
    let mut job_data = job_info.try_borrow_mut_data()?;
    zk_job.serialize(&mut &mut job_data[..])?;

    msg!("ZK job cancelled successfully");
    msg!("  Job ID: {}", zk_job.common.id);
    msg!("  Refunded: {} lamports", escrow_balance);

    Ok(())
}
