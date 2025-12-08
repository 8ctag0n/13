//! FinalizeJob instruction processor for FHE jobs
//!
//! This is the most complex instruction - handles consensus checking
//! and payment distribution to provers.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
    sysvar::Sysvar,
};
use zyberlink_types::JobStatus;

use crate::{
    error::FheGeneratorError,
    state::{FheConsensusData, FheJob, FHE_CONSENSUS_SEED, MAX_FHE_PROVERS},
};

/// Seeds for escrow PDA
const ESCROW_SEED: &[u8] = b"fhe_escrow";

/// Protocol fee (2.5% = 250 basis points)
const PROTOCOL_FEE_BPS: u64 = 250;
const BPS_DENOMINATOR: u64 = 10_000;

/// Process FinalizeJob instruction for FHE jobs
///
/// Checks consensus among prover results and distributes payments accordingly.
/// - If consensus reached: pay matching provers, penalize mismatching ones
/// - If no consensus: refund creator, penalize all provers
///
/// Accounts:
/// 0. `[signer]` Finalizer (anyone can trigger)
/// 1. `[writable]` FheJob account
/// 2. `[writable]` FheConsensusData account
/// 3. `[writable]` Escrow PDA
/// 4. `[writable]` Creator (for refunds)
/// 5. `[writable]` Protocol fee recipient
/// 6-N. `[writable]` Prover wallet accounts (in order of claimed_provers)
pub fn process_finalize_job(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // Fixed accounts
    let _finalizer_info = next_account_info(account_info_iter)?;
    let job_info = next_account_info(account_info_iter)?;
    let consensus_info = next_account_info(account_info_iter)?;
    let escrow_info = next_account_info(account_info_iter)?;
    let creator_info = next_account_info(account_info_iter)?;
    let fee_recipient_info = next_account_info(account_info_iter)?;

    // Remaining accounts are prover wallet accounts
    let prover_accounts: Vec<AccountInfo> = account_info_iter.cloned().collect();

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

    // Verify job is in valid status (Pending or Claimed)
    if fhe_job.common.status != JobStatus::Pending && fhe_job.common.status != JobStatus::Claimed {
        msg!(
            "Job must be in Pending or Claimed status, current: {:?}",
            fhe_job.common.status
        );
        return Err(FheGeneratorError::InvalidJobStatus.into());
    }

    // Verify not already finalized
    if consensus_data.has_consensus() {
        msg!("Job already finalized");
        return Err(FheGeneratorError::AlreadyFinalized.into());
    }

    // Verify all required results submitted
    if consensus_data.results_count < consensus_data.required_provers {
        msg!(
            "Insufficient results: {}/{}",
            consensus_data.results_count,
            consensus_data.required_provers
        );
        return Err(FheGeneratorError::InsufficientResults.into());
    }

    // Verify creator matches
    if creator_info.key != &fhe_job.common.creator {
        msg!("Creator mismatch");
        return Err(FheGeneratorError::Unauthorized.into());
    }

    // Verify escrow PDA
    let (escrow_pda, _) =
        Pubkey::find_program_address(&[ESCROW_SEED, job_info.key.as_ref()], program_id);

    if escrow_info.key != &escrow_pda {
        msg!("Invalid escrow account");
        return Err(FheGeneratorError::InvalidEscrow.into());
    }

    // Verify we have accounts for all provers
    let num_provers = consensus_data.results_count as usize;
    if prover_accounts.len() != num_provers {
        msg!(
            "Invalid number of prover accounts: expected {}, got {}",
            num_provers,
            prover_accounts.len()
        );
        return Err(FheGeneratorError::InvalidInstruction.into());
    }

    // Get current time
    let clock = Clock::get()?;
    let _current_time = clock.unix_timestamp;

    // Check consensus
    let consensus_result = consensus_data.check_consensus();

    match consensus_result {
        Some(consensus_hash) => {
            msg!("Consensus achieved!");

            // Mark job as completed
            fhe_job.common.status = JobStatus::Completed;
            fhe_job.common.proof_hash = Some(consensus_hash);

            // Count matching provers
            let mut matching_count: usize = 0;
            let mut matching_indices: [bool; MAX_FHE_PROVERS] = [false; MAX_FHE_PROVERS];

            for i in 0..num_provers {
                if consensus_data.result_submitted[i]
                    && consensus_data.result_hashes[i] == consensus_hash
                {
                    matching_count += 1;
                    matching_indices[i] = true;
                }
            }

            let mismatching_count = num_provers - matching_count;
            msg!(
                "Matching: {} provers, Mismatching: {} provers",
                matching_count,
                mismatching_count
            );

            // Calculate payments
            let total_reward = fhe_job.common.price_lamports;
            let platform_fee = total_reward.saturating_mul(PROTOCOL_FEE_BPS) / BPS_DENOMINATOR;
            let prover_payout_total = total_reward.saturating_sub(platform_fee);
            let payout_per_prover = if matching_count > 0 {
                prover_payout_total / matching_count as u64
            } else {
                0
            };

            msg!("Payment distribution:");
            msg!("  Total: {} lamports", total_reward);
            msg!("  Platform fee: {} lamports", platform_fee);
            msg!("  Per matching prover: {} lamports", payout_per_prover);

            // Pay platform fee
            if platform_fee > 0 {
                **escrow_info.try_borrow_mut_lamports()? = escrow_info
                    .lamports()
                    .checked_sub(platform_fee)
                    .ok_or(FheGeneratorError::InsufficientFunds)?;
                **fee_recipient_info.try_borrow_mut_lamports()? = fee_recipient_info
                    .lamports()
                    .checked_add(platform_fee)
                    .ok_or(FheGeneratorError::Overflow)?;
            }

            // Pay matching provers
            for (result_idx, prover_info) in prover_accounts.iter().enumerate() {
                let prover_key = &consensus_data.claimed_provers[result_idx];

                // Verify prover account matches
                if prover_info.key != prover_key {
                    msg!(
                        "Prover mismatch at {}: expected {}, got {}",
                        result_idx,
                        prover_key,
                        prover_info.key
                    );
                    return Err(FheGeneratorError::InvalidInstruction.into());
                }

                if matching_indices[result_idx] {
                    // Pay matching prover
                    **escrow_info.try_borrow_mut_lamports()? = escrow_info
                        .lamports()
                        .checked_sub(payout_per_prover)
                        .ok_or(FheGeneratorError::InsufficientFunds)?;
                    **prover_info.try_borrow_mut_lamports()? = prover_info
                        .lamports()
                        .checked_add(payout_per_prover)
                        .ok_or(FheGeneratorError::Overflow)?;

                    msg!("Paid prover {}: {} lamports", prover_key, payout_per_prover);
                } else {
                    msg!("Mismatching prover {} receives nothing", prover_key);
                }
            }

            msg!("Job {} finalized with consensus", fhe_job.common.id);
        }
        None => {
            msg!("Consensus failed! No majority agreement.");

            // Mark job as failed
            fhe_job.common.status = JobStatus::Failed;

            // Refund creator
            let refund_amount = fhe_job.common.price_lamports;
            **escrow_info.try_borrow_mut_lamports()? = escrow_info
                .lamports()
                .checked_sub(refund_amount)
                .ok_or(FheGeneratorError::InsufficientFunds)?;
            **creator_info.try_borrow_mut_lamports()? = creator_info
                .lamports()
                .checked_add(refund_amount)
                .ok_or(FheGeneratorError::Overflow)?;

            msg!("Refunded {} lamports to creator", refund_amount);

            // Verify prover accounts match (for consistency)
            for (result_idx, prover_info) in prover_accounts.iter().enumerate() {
                let prover_key = &consensus_data.claimed_provers[result_idx];
                if prover_info.key != prover_key {
                    msg!(
                        "Prover mismatch at {}: expected {}, got {}",
                        result_idx,
                        prover_key,
                        prover_info.key
                    );
                    return Err(FheGeneratorError::InvalidInstruction.into());
                }
            }

            msg!(
                "Job {} marked as failed - no consensus reached",
                fhe_job.common.id
            );
        }
    }

    // Save updated job
    let mut job_data = job_info.try_borrow_mut_data()?;
    fhe_job.serialize(&mut &mut job_data[..])?;

    // Save updated consensus data
    let mut consensus_account_data = consensus_info.try_borrow_mut_data()?;
    consensus_data.serialize(&mut &mut consensus_account_data[..])?;

    Ok(())
}
