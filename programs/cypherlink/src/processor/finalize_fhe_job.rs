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
    state::{JobAccount, MarketplaceConfig},
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
pub fn process_finalize_fhe_job(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let _finalizer_info = next_account_info(account_info_iter)?; // Can be anyone
    let job_info = next_account_info(account_info_iter)?;
    let escrow_info = next_account_info(account_info_iter)?;
    let creator_info = next_account_info(account_info_iter)?;
    let protocol_fee_recipient_info = next_account_info(account_info_iter)?;
    let config_info = next_account_info(account_info_iter)?;
    let _system_program_info = next_account_info(account_info_iter)?;
    let _clock_info = next_account_info(account_info_iter)?;

    // Remaining accounts are prover accounts (dynamic)
    let prover_accounts: Vec<AccountInfo> = account_info_iter.cloned().collect();

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

    // Validate: job must be Claimed
    if job.status != JobStatus::Claimed {
        msg!("Job must be in Claimed status");
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

    // Load marketplace config
    let marketplace_config: MarketplaceConfig = borsh::from_slice(&config_info.data.borrow())?;

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
            let matching_provers: Vec<_> = job.fhe_results
                .iter()
                .filter(|r| r.result_hash == consensus_hash)
                .collect();

            msg!(
                "Matching: {} provers, Mismatching: {} provers",
                matching_provers.len(),
                job.fhe_results.len() - matching_provers.len()
            );

            // Calculate payments (split reward among matching provers)
            let total_reward = job.price_lamports;
            let platform_fee = total_reward
                .checked_mul(marketplace_config.fee_basis_points as u64)
                .and_then(|v| v.checked_div(10000))
                .ok_or(CypherLinkProgramError::Overflow)?;
            let prover_payout = total_reward - platform_fee;
            let payout_per_prover = prover_payout / matching_provers.len() as u64;

            // Pay platform fee
            **escrow_info.try_borrow_mut_lamports()? = escrow_info
                .lamports()
                .checked_sub(platform_fee)
                .ok_or(CypherLinkProgramError::InsufficientFunds)?;
            **protocol_fee_recipient_info.try_borrow_mut_lamports()? = protocol_fee_recipient_info
                .lamports()
                .checked_add(platform_fee)
                .ok_or(CypherLinkProgramError::Overflow)?;

            // Pay matching provers
            for (i, matching_result) in matching_provers.iter().enumerate() {
                if i >= prover_accounts.len() {
                    msg!("Not enough prover accounts provided");
                    return Err(CypherLinkProgramError::InvalidAccount.into());
                }

                let prover_account_info = &prover_accounts[i];

                // SECURITY: Verify prover account matches the result
                if prover_account_info.key != &matching_result.prover {
                    msg!("Prover account mismatch: expected {}, got {}",
                         matching_result.prover,
                         prover_account_info.key);
                    return Err(CypherLinkProgramError::InvalidAccount.into());
                }

                // Transfer SOL from escrow to prover
                **escrow_info.try_borrow_mut_lamports()? = escrow_info
                    .lamports()
                    .checked_sub(payout_per_prover)
                    .ok_or(CypherLinkProgramError::InsufficientFunds)?;
                **prover_account_info.try_borrow_mut_lamports()? = prover_account_info
                    .lamports()
                    .checked_add(payout_per_prover)
                    .ok_or(CypherLinkProgramError::Overflow)?;

                msg!(
                    "Paid prover {} amount {}",
                    matching_result.prover,
                    payout_per_prover
                );
            }

            msg!("Job {} finalized successfully", job.id);
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

            msg!("Refunded {} lamports to creator", refund_amount);
            msg!("Job {} marked as failed", job.id);
        }
    }

    // Serialize job back
    let mut job_data = job_info.try_borrow_mut_data()?;
    borsh::to_writer(&mut job_data[..], &job)?;

    Ok(())
}
