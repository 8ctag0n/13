use borsh::BorshDeserialize;
use cypherlink_types::{CircuitType, FheJobResult, JobStatus};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
};
use std::collections::HashMap;

use crate::{
    error::CypherLinkProgramError,
    state::{JobAccount, MarketplaceConfig, ProverAccount},
};

/// Find consensus among FHE results
/// Returns Some(hash) if consensus reached, None if failed
fn find_consensus(
    results: &[FheJobResult],
    consensus_threshold: u8,
) -> Option<[u8; 32]> {
    let mut hash_counts: HashMap<[u8; 32], usize> = HashMap::new();

    for result in results {
        *hash_counts.entry(result.result_hash).or_insert(0) += 1;
    }

    // Find hash with >= consensus_threshold matches
    hash_counts
        .into_iter()
        .find(|(_, count)| *count >= consensus_threshold as usize)
        .map(|(hash, _)| hash)
}

/// Process FinalizeFheJob instruction
///
/// Accounts expected (in order):
/// 0. [writable, signer] finalizer (anyone can trigger finalization)
/// 1. [writable] job_pda
/// 2. [writable] escrow_pda
/// 3. [writable] creator (for refunds)
/// 4. [writable] protocol_fee_recipient
/// 5. [] config_pda
/// 6. [] system_program
/// 7. [] clock_sysvar
/// 8-N. [writable] prover_authority_accounts (dynamic, in same order as fhe_results)
/// N+1-M. [writable] prover_pda_accounts (dynamic, in same order as fhe_results)
pub fn process_finalize_fhe_job(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // Fixed accounts
    let _finalizer_info = next_account_info(account_info_iter)?; // Can be anyone
    let job_info = next_account_info(account_info_iter)?;
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
        return Err(CypherLinkProgramError::InvalidAccount.into());
    }

    // Verify config PDA
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], program_id);
    if config_info.key != &config_pda {
        msg!("Invalid config account");
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

    // Validate: job must be Claimed
    if job.status != JobStatus::Claimed {
        msg!("Job must be in Claimed status, current: {:?}", job.status);
        return Err(CypherLinkProgramError::InvalidJobStatus.into());
    }

    // Validate: all required provers have submitted
    if job.fhe_results.len() < config.required_provers as usize {
        msg!(
            "Insufficient FHE results: {}/{}",
            job.fhe_results.len(),
            config.required_provers
        );
        return Err(CypherLinkProgramError::InsufficientFheResults.into());
    }

    // Validate: not already finalized
    if job.fhe_consensus_hash.is_some() {
        msg!("Job already finalized");
        return Err(CypherLinkProgramError::AlreadyFinalized.into());
    }

    // Validate: creator account matches job creator
    if creator_info.key != &job.creator {
        msg!("Creator account mismatch: expected {}, got {}", job.creator, creator_info.key);
        return Err(CypherLinkProgramError::InvalidAccount.into());
    }

    // Validate: escrow PDA
    let (escrow_pda, _) = Pubkey::find_program_address(
        &[b"escrow", job_info.key.as_ref()],
        program_id,
    );
    if escrow_info.key != &escrow_pda {
        msg!("Invalid escrow account");
        return Err(CypherLinkProgramError::InvalidAccount.into());
    }

    // Load marketplace config
    let marketplace_config: MarketplaceConfig = borsh::from_slice(&config_info.data.borrow())?;

    // Validate: protocol fee recipient
    if protocol_fee_recipient_info.key != &marketplace_config.protocol_fee_recipient {
        msg!("Invalid protocol fee recipient");
        return Err(CypherLinkProgramError::InvalidAccount.into());
    }

    // Validate: we have enough accounts for all provers (2 per prover: authority + PDA)
    let expected_accounts = job.fhe_results.len() * 2;
    if remaining_accounts.len() != expected_accounts {
        msg!(
            "Invalid number of prover accounts: expected {}, got {}",
            expected_accounts,
            remaining_accounts.len()
        );
        return Err(CypherLinkProgramError::InvalidAccount.into());
    }

    // Get current time
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    // Find consensus
    let consensus_result = find_consensus(&job.fhe_results, config.consensus_threshold);

    match consensus_result {
        Some(consensus_hash) => {
            msg!("Consensus achieved! Hash: {:?}", consensus_hash);

            // Mark consensus
            job.fhe_consensus_hash = Some(consensus_hash);
            job.status = JobStatus::Completed;
            job.completed_at = Some(current_time);

            // Identify matching and mismatching provers
            let matching_results: Vec<_> = job.fhe_results
                .iter()
                .filter(|r| r.result_hash == consensus_hash)
                .collect();

            let mismatching_results: Vec<_> = job.fhe_results
                .iter()
                .filter(|r| r.result_hash != consensus_hash)
                .collect();

            msg!(
                "Matching: {} provers, Mismatching: {} provers",
                matching_results.len(),
                mismatching_results.len()
            );

            // Calculate payments
            let total_reward = job.price_lamports;
            let platform_fee = total_reward
                .checked_mul(marketplace_config.fee_basis_points as u64)
                .and_then(|v| v.checked_div(10000))
                .ok_or(CypherLinkProgramError::Overflow)?;
            let prover_payout_total = total_reward
                .checked_sub(platform_fee)
                .ok_or(CypherLinkProgramError::Overflow)?;
            let payout_per_prover = prover_payout_total
                .checked_div(matching_results.len() as u64)
                .ok_or(CypherLinkProgramError::Overflow)?;

            msg!("Payment distribution:");
            msg!("  Total reward: {} lamports", total_reward);
            msg!("  Platform fee: {} lamports", platform_fee);
            msg!("  Prover payout total: {} lamports", prover_payout_total);
            msg!("  Per matching prover: {} lamports", payout_per_prover);

            // Pay platform fee
            **escrow_info.try_borrow_mut_lamports()? = escrow_info
                .lamports()
                .checked_sub(platform_fee)
                .ok_or(CypherLinkProgramError::InsufficientFunds)?;
            **protocol_fee_recipient_info.try_borrow_mut_lamports()? = protocol_fee_recipient_info
                .lamports()
                .checked_add(platform_fee)
                .ok_or(CypherLinkProgramError::Overflow)?;

            // Process all provers (both matching and mismatching)
            for (result_idx, result) in job.fhe_results.iter().enumerate() {
                let prover_authority_info = &remaining_accounts[result_idx * 2];
                let prover_pda_info = &remaining_accounts[result_idx * 2 + 1];

                // Verify prover authority matches
                if prover_authority_info.key != &result.prover {
                    msg!(
                        "Prover authority mismatch at index {}: expected {}, got {}",
                        result_idx,
                        result.prover,
                        prover_authority_info.key
                    );
                    return Err(CypherLinkProgramError::InvalidAccount.into());
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
                    return Err(CypherLinkProgramError::InvalidAccount.into());
                }

                // Load prover account
                let mut prover_account: ProverAccount = borsh::from_slice(&prover_pda_info.data.borrow())?;

                // Check if this prover matched consensus
                let is_matching = result.result_hash == consensus_hash;

                if is_matching {
                    // Pay matching prover
                    **escrow_info.try_borrow_mut_lamports()? = escrow_info
                        .lamports()
                        .checked_sub(payout_per_prover)
                        .ok_or(CypherLinkProgramError::InsufficientFunds)?;
                    **prover_authority_info.try_borrow_mut_lamports()? = prover_authority_info
                        .lamports()
                        .checked_add(payout_per_prover)
                        .ok_or(CypherLinkProgramError::Overflow)?;

                    // Update reputation: +10 for honest work
                    prover_account.on_job_completed(
                        (current_time - result.submitted_at) as u32,
                        payout_per_prover
                    );

                    msg!(
                        "Paid matching prover {}: {} lamports (reputation: {})",
                        result.prover,
                        payout_per_prover,
                        prover_account.reputation_score
                    );
                } else {
                    // Penalize mismatching prover: -50 reputation
                    prover_account.on_job_failed();

                    msg!(
                        "Penalized mismatching prover {}: reputation now {}",
                        result.prover,
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

            job.fhe_consensus_hash = None;
            job.status = JobStatus::Failed;
            job.completed_at = Some(current_time);

            // Refund creator
            let refund_amount = job.price_lamports;
            **escrow_info.try_borrow_mut_lamports()? = escrow_info
                .lamports()
                .checked_sub(refund_amount)
                .ok_or(CypherLinkProgramError::InsufficientFunds)?;
            **creator_info.try_borrow_mut_lamports()? = creator_info
                .lamports()
                .checked_add(refund_amount)
                .ok_or(CypherLinkProgramError::Overflow)?;

            msg!("Refunded {} lamports to creator {}", refund_amount, creator_info.key);

            // Penalize ALL provers (-50 reputation each)
            for (result_idx, result) in job.fhe_results.iter().enumerate() {
                let prover_authority_info = &remaining_accounts[result_idx * 2];
                let prover_pda_info = &remaining_accounts[result_idx * 2 + 1];

                // Verify prover authority matches
                if prover_authority_info.key != &result.prover {
                    msg!(
                        "Prover authority mismatch at index {}: expected {}, got {}",
                        result_idx,
                        result.prover,
                        prover_authority_info.key
                    );
                    return Err(CypherLinkProgramError::InvalidAccount.into());
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
                    return Err(CypherLinkProgramError::InvalidAccount.into());
                }

                // Load and penalize prover
                let mut prover_account: ProverAccount = borsh::from_slice(&prover_pda_info.data.borrow())?;
                prover_account.on_job_failed();

                msg!(
                    "Penalized prover {} for consensus failure: reputation now {}",
                    result.prover,
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

    Ok(())
}
