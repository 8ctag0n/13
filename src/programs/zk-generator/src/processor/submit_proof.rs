//! SubmitProof instruction processor

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

use crate::{error::ZkGeneratorError, state::ZkJob};

/// Seeds for escrow PDA
const ESCROW_SEED: &[u8] = b"zk_escrow";

/// Protocol fee (2.5% = 250 basis points)
const PROTOCOL_FEE_BPS: u64 = 250;
const BPS_DENOMINATOR: u64 = 10_000;

/// Process SubmitProof instruction
///
/// Accounts:
/// 0. `[signer]` Prover wallet (must match job.prover)
/// 1. `[writable]` ZkJob account
/// 2. `[writable]` Escrow PDA (holds payment)
/// 3. `[writable]` Protocol fee recipient
pub fn process_submit_proof(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    proof_hash: [u8; 32],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let prover_info = next_account_info(account_info_iter)?;
    let job_info = next_account_info(account_info_iter)?;
    let escrow_info = next_account_info(account_info_iter)?;
    let fee_recipient_info = next_account_info(account_info_iter)?;

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

    // Verify job is in Claimed status
    if zk_job.common.status != JobStatus::Claimed {
        msg!("Job is not in Claimed status");
        return Err(ZkGeneratorError::JobNotClaimed.into());
    }

    // Verify prover matches
    if zk_job.common.prover != Some(*prover_info.key) {
        msg!("Prover does not match job claimant");
        return Err(ZkGeneratorError::Unauthorized.into());
    }

    // Get current time
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    // Check if job timed out
    if zk_job.common.is_expired(current_time) {
        msg!("Job has timed out");
        return Err(ZkGeneratorError::JobExpired.into());
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

    // Calculate fees and payout
    let platform_fee = zk_job
        .common
        .price_lamports
        .saturating_mul(PROTOCOL_FEE_BPS)
        / BPS_DENOMINATOR;
    let prover_payout = zk_job.common.price_lamports.saturating_sub(platform_fee);

    msg!("Processing payment");
    msg!("  Total: {} lamports", zk_job.common.price_lamports);
    msg!("  Platform fee: {} lamports", platform_fee);
    msg!("  Prover payout: {} lamports", prover_payout);

    // Transfer platform fee to fee recipient
    if platform_fee > 0 {
        **escrow_info.try_borrow_mut_lamports()? = escrow_info
            .lamports()
            .checked_sub(platform_fee)
            .ok_or(ZkGeneratorError::InsufficientFunds)?;
        **fee_recipient_info.try_borrow_mut_lamports()? = fee_recipient_info
            .lamports()
            .checked_add(platform_fee)
            .ok_or(ZkGeneratorError::Overflow)?;
    }

    // Transfer prover payout
    **escrow_info.try_borrow_mut_lamports()? = escrow_info
        .lamports()
        .checked_sub(prover_payout)
        .ok_or(ZkGeneratorError::InsufficientFunds)?;
    **prover_info.try_borrow_mut_lamports()? = prover_info
        .lamports()
        .checked_add(prover_payout)
        .ok_or(ZkGeneratorError::Overflow)?;

    // Complete the job
    zk_job.common.complete(proof_hash);

    // Save updated job
    let mut job_data = job_info.try_borrow_mut_data()?;
    zk_job.serialize(&mut &mut job_data[..])?;

    msg!("ZK proof submitted successfully");
    msg!("  Job ID: {}", zk_job.common.id);
    msg!("  Prover: {}", prover_info.key);
    msg!("  Proof hash: {:?}", &proof_hash[..8]);

    Ok(())
}
