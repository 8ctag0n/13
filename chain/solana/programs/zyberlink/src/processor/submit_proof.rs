use borsh::BorshDeserialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
};
use zyberlink_types::JobStatus;

use crate::{
    error::ZyberLinkProgramError,
    state::{JobAccount, MarketplaceConfig, ProverAccount},
};

/// Process SubmitProof instruction (for ZK jobs only)
#[allow(clippy::too_many_arguments)]
pub fn process_submit_proof(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    proof_commitment: [u8; 32],
    _proof_size: u32,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let prover_authority_info = next_account_info(account_info_iter)?;
    let prover_info = next_account_info(account_info_iter)?;
    let job_info = next_account_info(account_info_iter)?;
    let escrow_info = next_account_info(account_info_iter)?;
    let job_creator_info = next_account_info(account_info_iter)?;
    let protocol_fee_recipient_info = next_account_info(account_info_iter)?;
    let config_info = next_account_info(account_info_iter)?;
    let _system_program_info = next_account_info(account_info_iter)?;

    // Verify prover authority is signer
    if !prover_authority_info.is_signer {
        msg!("Prover authority must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Verify config PDA
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], program_id);
    if config_info.key != &config_pda {
        msg!("Invalid config account");
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Load marketplace config
    let mut config: MarketplaceConfig = borsh::from_slice(&config_info.data.borrow())?;

    // Verify prover PDA
    let (prover_pda, _) =
        Pubkey::find_program_address(&[b"prover", prover_authority_info.key.as_ref()], program_id);

    if prover_info.key != &prover_pda {
        msg!("Invalid prover account");
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Load prover account
    let mut prover: ProverAccount = borsh::from_slice(&prover_info.data.borrow())?;

    // Load job account
    let mut job: JobAccount = {
        let mut data_slice = &job_info.data.borrow()[..];
        JobAccount::deserialize(&mut data_slice)?
    };

    // Verify this is a ZK job (FHE jobs use finalize_fhe_job instead)
    if job.is_fhe() {
        msg!("Use finalize_fhe_job for FHE jobs");
        return Err(ZyberLinkProgramError::NotFheJob.into());
    }

    // Verify job is in Claimed status
    if job.status != JobStatus::Claimed {
        msg!("Job is not in Claimed status");
        return Err(ZyberLinkProgramError::JobNotClaimed.into());
    }

    // Verify prover is the one who claimed the job
    if job.prover != Some(*prover_authority_info.key) {
        msg!("Job was not claimed by this prover");
        return Err(ZyberLinkProgramError::Unauthorized.into());
    }

    // Verify job creator matches
    if job.creator != *job_creator_info.key {
        msg!("Invalid job creator");
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Get current time
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    // Check if job timed out
    if job.is_timed_out(current_time) {
        msg!("Job has timed out");
        return Err(ZyberLinkProgramError::JobTimedOut.into());
    }

    // Verify escrow PDA
    let (escrow_pda, _escrow_bump) =
        Pubkey::find_program_address(&[b"escrow", job_info.key.as_ref()], program_id);

    if escrow_info.key != &escrow_pda {
        msg!("Invalid escrow account");
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Verify protocol fee recipient
    if protocol_fee_recipient_info.key != &config.protocol_fee_recipient {
        msg!("Invalid protocol fee recipient");
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Calculate fees and payout
    let platform_fee = config.calculate_platform_fee(job.price_lamports);
    let prover_payout = config.calculate_prover_payout(job.price_lamports);

    msg!("Processing payment");
    msg!("  Total: {} lamports", job.price_lamports);
    msg!("  Platform fee: {} lamports", platform_fee);
    msg!("  Prover payout: {} lamports", prover_payout);

    // Transfer platform fee to protocol recipient (manual lamport transfer)
    if platform_fee > 0 {
        **escrow_info.lamports.borrow_mut() = escrow_info
            .lamports()
            .checked_sub(platform_fee)
            .ok_or(ZyberLinkProgramError::InsufficientFunds)?;
        **protocol_fee_recipient_info.lamports.borrow_mut() = protocol_fee_recipient_info
            .lamports()
            .checked_add(platform_fee)
            .ok_or(ZyberLinkProgramError::Overflow)?;
    }

    // Transfer prover payout (manual lamport transfer)
    **escrow_info.lamports.borrow_mut() = escrow_info
        .lamports()
        .checked_sub(prover_payout)
        .ok_or(ZyberLinkProgramError::InsufficientFunds)?;
    **prover_authority_info.lamports.borrow_mut() = prover_authority_info
        .lamports()
        .checked_add(prover_payout)
        .ok_or(ZyberLinkProgramError::Overflow)?;

    // Update job status (new API: complete(proof_hash, current_time))
    job.complete(proof_commitment, current_time);

    // Update prover statistics
    prover.total_jobs_completed = prover.total_jobs_completed.saturating_add(1);

    // Update marketplace statistics
    config.total_jobs_completed = config.total_jobs_completed.saturating_add(1);

    // Serialize updated states
    let mut job_data = job_info.try_borrow_mut_data()?;
    borsh::to_writer(&mut job_data[..], &job)?;

    let mut prover_data = prover_info.try_borrow_mut_data()?;
    borsh::to_writer(&mut prover_data[..], &prover)?;

    let mut config_data = config_info.try_borrow_mut_data()?;
    borsh::to_writer(&mut config_data[..], &config)?;

    msg!("Proof submitted successfully");
    msg!("  Job ID: {}", job.id);

    Ok(())
}
