use borsh::BorshDeserialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{error::CypherLinkProgramError, state::JobAccount};

/// Process CancelJob instruction
pub fn process_cancel_job(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let job_creator_info = next_account_info(account_info_iter)?;
    let job_info = next_account_info(account_info_iter)?;
    let escrow_info = next_account_info(account_info_iter)?;

    // Verify job creator is signer
    if !job_creator_info.is_signer {
        msg!("Job creator must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Verify job account is owned by program
    if job_info.owner != program_id {
        msg!("Invalid job account owner");
        return Err(CypherLinkProgramError::InvalidAccount.into());
    }

    // Load job account
    let mut job: JobAccount = {
        let mut data_slice = &job_info.data.borrow()[..];
        JobAccount::deserialize(&mut data_slice)?
    };

    // Verify job creator matches
    if job.creator != *job_creator_info.key {
        msg!("Only job creator can cancel the job");
        return Err(CypherLinkProgramError::Unauthorized.into());
    }

    // Verify job is in Pending status (can only cancel unclaimed jobs)
    if job.status != cypherlink_types::JobStatus::Pending {
        msg!("Can only cancel jobs in Pending status");
        return Err(CypherLinkProgramError::JobNotPending.into());
    }

    // Verify escrow PDA
    let (escrow_pda, escrow_bump) =
        Pubkey::find_program_address(&[b"escrow", job_info.key.as_ref()], program_id);

    if escrow_info.key != &escrow_pda {
        msg!("Invalid escrow account");
        return Err(CypherLinkProgramError::InvalidAccount.into());
    }

    // Get escrow balance (should be price + rent)
    let escrow_balance = escrow_info.lamports();

    msg!("Cancelling job and refunding {} lamports", escrow_balance);

    // Transfer all escrow funds back to creator (manual lamport transfer)
    **escrow_info.lamports.borrow_mut() = 0;
    **job_creator_info.lamports.borrow_mut() = job_creator_info
        .lamports()
        .checked_add(escrow_balance)
        .ok_or(CypherLinkProgramError::Overflow)?;

    // Update job status
    job.cancel();

    // Serialize updated job
    let mut job_data = job_info.try_borrow_mut_data()?;
    borsh::to_writer(&mut job_data[..], &job)?;

    msg!("Job cancelled successfully");
    msg!("  Job ID: {}", job.id);
    msg!("  Refunded: {} lamports", escrow_balance);

    Ok(())
}
