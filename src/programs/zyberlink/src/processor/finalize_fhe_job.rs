use borsh::BorshDeserialize;
use zyberlink_types::JobStatus;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
};

use crate::{
    error::ZyberLinkProgramError,
    state::{FheConsensusData, JobAccount, MarketplaceConfig, ProverAccount, MAX_FHE_PROVERS},
};

/// Process FinalizeFheJob instruction
///
/// Accounts expected (in order):
/// 0. [writable, signer] finalizer (anyone can trigger finalization)
/// 1. [writable] job_pda
/// 2. [writable] fhe_consensus_pda
/// 3. [writable] escrow_pda
/// 4. [writable] creator (for refunds)
/// 5. [writable] protocol_fee_recipient
/// 6. [] config_pda
/// 7. [] system_program
/// 8. [] clock_sysvar
/// 9-N. [writable] prover_authority_accounts (in order of claimed_provers)
/// N+1-M. [writable] prover_pda_accounts (in order of claimed_provers)
pub fn process_finalize_fhe_job(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // Fixed accounts
    let _finalizer_info = next_account_info(account_info_iter)?; // Can be anyone
    let job_info = next_account_info(account_info_iter)?;
    let fhe_consensus_info = next_account_info(account_info_iter)?;
    let escrow_info = next_account_info(account_info_iter)?;
    let creator_info = next_account_info(account_info_iter)?;
    let protocol_fee_recipient_info = next_account_info(account_info_iter)?;
    let config_info = next_account_info(account_info_iter)?;
    let _system_program_info = next_account_info(account_info_iter)?;
    let _clock_info = next_account_info(account_info_iter)?;

    // Remaining accounts are prover accounts (dynamic)
    // They must be provided in pairs: [prover_authority, prover_pda] for each result
    let remaining_accounts: Vec<AccountInfo> = account_info_iter.cloned().collect();

    // Verify job account is owned by program
    if job_info.owner != program_id {
        msg!("Invalid job account owner");
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Verify config PDA
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], program_id);
    if config_info.key != &config_pda {
        msg!("Invalid config account");
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Deserialize job
    let mut job: JobAccount = {
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

    // Validate: job must be Claimed
    if job.status != JobStatus::Claimed {
        msg!("Job must be in Claimed status, current: {:?}", job.status);
        return Err(ZyberLinkProgramError::InvalidJobStatus.into());
    }

    // Validate: all required provers have submitted
    if fhe_data.results_count < fhe_data.required_provers {
        msg!(
            "Insufficient FHE results: {}/{}",
            fhe_data.results_count,
            fhe_data.required_provers
        );
        return Err(ZyberLinkProgramError::InsufficientFheResults.into());
    }

    // Validate: not already finalized
    if fhe_data.has_consensus() {
        msg!("Job already finalized");
        return Err(ZyberLinkProgramError::AlreadyFinalized.into());
    }

    // Validate: creator account matches job creator
    if creator_info.key != &job.creator {
        msg!(
            "Creator account mismatch: expected {}, got {}",
            job.creator,
            creator_info.key
        );
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Validate: escrow PDA
    let (escrow_pda, _) =
        Pubkey::find_program_address(&[b"escrow", job_info.key.as_ref()], program_id);
    if escrow_info.key != &escrow_pda {
        msg!("Invalid escrow account");
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Load marketplace config
    let marketplace_config: MarketplaceConfig = borsh::from_slice(&config_info.data.borrow())?;

    // Validate: protocol fee recipient
    if protocol_fee_recipient_info.key != &marketplace_config.protocol_fee_recipient {
        msg!("Invalid protocol fee recipient");
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Validate: we have enough accounts for all provers (2 per prover: authority + PDA)
    let num_provers = fhe_data.results_count as usize;
    let expected_accounts = num_provers * 2;
    if remaining_accounts.len() != expected_accounts {
        msg!(
            "Invalid number of prover accounts: expected {}, got {}",
            expected_accounts,
            remaining_accounts.len()
        );
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Get current time
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    // Check consensus
    let consensus_result = fhe_data.check_consensus();

    match consensus_result {
        Some(consensus_hash) => {
            msg!("Consensus achieved! Hash: {:?}", consensus_hash);

            // Mark job as completed
            job.status = JobStatus::Completed;
            job.proof_hash = Some(consensus_hash);

            // Count matching and mismatching provers
            let mut matching_count: usize = 0;
            let mut matching_indices: [bool; MAX_FHE_PROVERS] = [false; MAX_FHE_PROVERS];

            for i in 0..num_provers {
                if fhe_data.result_submitted[i] && fhe_data.result_hashes[i] == consensus_hash {
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
            let total_reward = job.price_lamports;
            let platform_fee = total_reward
                .checked_mul(marketplace_config.fee_basis_points as u64)
                .and_then(|v| v.checked_div(10000))
                .ok_or(ZyberLinkProgramError::Overflow)?;
            let prover_payout_total = total_reward
                .checked_sub(platform_fee)
                .ok_or(ZyberLinkProgramError::Overflow)?;
            let payout_per_prover = if matching_count > 0 {
                prover_payout_total
                    .checked_div(matching_count as u64)
                    .ok_or(ZyberLinkProgramError::Overflow)?
            } else {
                0
            };

            msg!("Payment distribution:");
            msg!("  Total reward: {} lamports", total_reward);
            msg!("  Platform fee: {} lamports", platform_fee);
            msg!("  Prover payout total: {} lamports", prover_payout_total);
            msg!("  Per matching prover: {} lamports", payout_per_prover);

            // Pay platform fee
            **escrow_info.try_borrow_mut_lamports()? = escrow_info
                .lamports()
                .checked_sub(platform_fee)
                .ok_or(ZyberLinkProgramError::InsufficientFunds)?;
            **protocol_fee_recipient_info.try_borrow_mut_lamports()? = protocol_fee_recipient_info
                .lamports()
                .checked_add(platform_fee)
                .ok_or(ZyberLinkProgramError::Overflow)?;

            // Process all provers (both matching and mismatching)
            for result_idx in 0..num_provers {
                let prover_authority_info = &remaining_accounts[result_idx * 2];
                let prover_pda_info = &remaining_accounts[result_idx * 2 + 1];

                let prover_key = &fhe_data.claimed_provers[result_idx];

                // Verify prover authority matches
                if prover_authority_info.key != prover_key {
                    msg!(
                        "Prover authority mismatch at index {}: expected {}, got {}",
                        result_idx,
                        prover_key,
                        prover_authority_info.key
                    );
                    return Err(ZyberLinkProgramError::InvalidAccount.into());
                }

                // Verify prover PDA
                let (expected_prover_pda, _) = Pubkey::find_program_address(
                    &[b"prover", prover_authority_info.key.as_ref()],
                    program_id,
                );
                if prover_pda_info.key != &expected_prover_pda {
                    msg!(
                        "Prover PDA mismatch at index {}: expected {}, got {}",
                        result_idx,
                        expected_prover_pda,
                        prover_pda_info.key
                    );
                    return Err(ZyberLinkProgramError::InvalidAccount.into());
                }

                // Load prover account
                let mut prover_account: ProverAccount =
                    borsh::from_slice(&prover_pda_info.data.borrow())?;

                // Check if this prover matched consensus
                let is_matching = matching_indices[result_idx];

                if is_matching {
                    // Pay matching prover
                    **escrow_info.try_borrow_mut_lamports()? = escrow_info
                        .lamports()
                        .checked_sub(payout_per_prover)
                        .ok_or(ZyberLinkProgramError::InsufficientFunds)?;
                    **prover_authority_info.try_borrow_mut_lamports()? = prover_authority_info
                        .lamports()
                        .checked_add(payout_per_prover)
                        .ok_or(ZyberLinkProgramError::Overflow)?;

                    // Update reputation: +10 for honest work
                    prover_account.on_job_completed(
                        (current_time - fhe_data.submission_timeout) as u32,
                        payout_per_prover,
                    );

                    msg!(
                        "Paid matching prover {}: {} lamports (reputation: {})",
                        prover_key,
                        payout_per_prover,
                        prover_account.reputation_score
                    );
                } else {
                    // Penalize mismatching prover: -50 reputation
                    prover_account.on_job_failed();

                    msg!(
                        "Penalized mismatching prover {}: reputation now {}",
                        prover_key,
                        prover_account.reputation_score
                    );
                }

                // Save updated prover account
                let mut prover_data = prover_pda_info.try_borrow_mut_data()?;
                borsh::to_writer(&mut prover_data[..], &prover_account)?;
            }

            msg!("Job {} finalized successfully with consensus", job.id);
        }
        None => {
            msg!("Consensus failed! No majority agreement.");

            job.status = JobStatus::Failed;

            // Refund creator
            let refund_amount = job.price_lamports;
            **escrow_info.try_borrow_mut_lamports()? = escrow_info
                .lamports()
                .checked_sub(refund_amount)
                .ok_or(ZyberLinkProgramError::InsufficientFunds)?;
            **creator_info.try_borrow_mut_lamports()? = creator_info
                .lamports()
                .checked_add(refund_amount)
                .ok_or(ZyberLinkProgramError::Overflow)?;

            msg!(
                "Refunded {} lamports to creator {}",
                refund_amount,
                creator_info.key
            );

            // Penalize ALL provers (-50 reputation each)
            for result_idx in 0..num_provers {
                let prover_authority_info = &remaining_accounts[result_idx * 2];
                let prover_pda_info = &remaining_accounts[result_idx * 2 + 1];

                let prover_key = &fhe_data.claimed_provers[result_idx];

                // Verify prover authority matches
                if prover_authority_info.key != prover_key {
                    msg!(
                        "Prover authority mismatch at index {}: expected {}, got {}",
                        result_idx,
                        prover_key,
                        prover_authority_info.key
                    );
                    return Err(ZyberLinkProgramError::InvalidAccount.into());
                }

                // Verify prover PDA
                let (expected_prover_pda, _) = Pubkey::find_program_address(
                    &[b"prover", prover_authority_info.key.as_ref()],
                    program_id,
                );
                if prover_pda_info.key != &expected_prover_pda {
                    msg!(
                        "Prover PDA mismatch at index {}: expected {}, got {}",
                        result_idx,
                        expected_prover_pda,
                        prover_pda_info.key
                    );
                    return Err(ZyberLinkProgramError::InvalidAccount.into());
                }

                // Load and penalize prover
                let mut prover_account: ProverAccount =
                    borsh::from_slice(&prover_pda_info.data.borrow())?;
                prover_account.on_job_failed();

                msg!(
                    "Penalized prover {} for consensus failure: reputation now {}",
                    prover_key,
                    prover_account.reputation_score
                );

                // Save updated prover account
                let mut prover_data = prover_pda_info.try_borrow_mut_data()?;
                borsh::to_writer(&mut prover_data[..], &prover_account)?;
            }

            msg!("Job {} marked as failed - no consensus reached", job.id);
        }
    }

    // Serialize job back
    let mut job_data = job_info.try_borrow_mut_data()?;
    borsh::to_writer(&mut job_data[..], &job)?;

    // Serialize FHE consensus data back
    let mut fhe_account_data = fhe_consensus_info.try_borrow_mut_data()?;
    borsh::to_writer(&mut fhe_account_data[..], &fhe_data)?;

    Ok(())
}
