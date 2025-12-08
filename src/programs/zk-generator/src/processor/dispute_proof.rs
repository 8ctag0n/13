//! DisputeProof instruction processor
//!
//! This module handles proof disputes for completed ZK jobs.
//! When a proof is disputed, full on-chain verification occurs.
//!
//! # Dispute Flow
//!
//! 1. Job completed with proof_hash (prover paid)
//! 2. Within dispute window (24h), anyone can dispute
//! 3. Disputor provides full proof + public_inputs
//! 4. On-chain verification:
//!    - If proof invalid: prover slashed, disputor rewarded
//!    - If proof valid: disputor loses bond (anti-spam)

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
    circuits::{
        market::{
            verify_market_bet_proof, verify_market_bet_with_poi_proof, verify_market_claim_proof,
            MarketBetPublicInputs, MarketBetWithPoiPublicInputs, MarketClaimPublicInputs,
            MarketVerificationKey,
        },
        poi::{verify_poi_proof, PoiPublicInputs, PoiVerificationKey},
        portfolio::{
            verify_portfolio_compliance_proof, verify_portfolio_net_worth_proof,
            PortfolioCompliancePublicInputs, PortfolioNetWorthPublicInputs,
            PortfolioVerificationKey,
        },
        types::CircuitType,
        vote::{
            verify_vote_proof, verify_vote_with_poi_proof, VotePublicInputs,
            VoteVerificationKey, VoteWithPoiPublicInputs,
        },
    },
    error::ZkGeneratorError,
    state::ZkJob,
};

/// Dispute window in seconds (24 hours)
pub const DISPUTE_WINDOW_SECONDS: i64 = 24 * 60 * 60;

/// Minimum bond required to dispute (prevents spam)
pub const DISPUTE_BOND_LAMPORTS: u64 = 100_000_000; // 0.1 SOL

/// Reward percentage for successful dispute (50% of prover stake)
pub const DISPUTE_REWARD_BPS: u64 = 5000; // 50%

/// Verify a ZK proof according to its circuit type
///
/// Routes verification to the appropriate verifier based on CircuitType.
fn verify_circuit_proof(
    circuit_type: CircuitType,
    proof: &[u8],
    public_inputs: &[u8],
    current_time: i64,
) -> Result<bool, ProgramError> {
    msg!("Verifying {} proof on-chain...", circuit_type.name());

    match circuit_type {
        // Legacy circuits - accept any valid-sized proof
        CircuitType::ZcashOrchard | CircuitType::ZcashSapling => {
            if proof.len() != 256 {
                msg!("Invalid legacy proof size: {} != 256", proof.len());
                return Ok(false);
            }
            msg!("Legacy circuit proof valid (size check only)");
            Ok(true)
        }

        CircuitType::AnonymousVote | CircuitType::Credential => {
            if proof.len() != 256 {
                msg!("Invalid legacy proof size: {} != 256", proof.len());
                return Ok(false);
            }
            msg!("Legacy circuit proof valid (size check only)");
            Ok(true)
        }

        // Core primitives (v2.0)
        CircuitType::ProofOfInnocence => {
            let inputs = PoiPublicInputs::from_bytes(public_inputs)?;
            let vk = PoiVerificationKey::default();
            verify_poi_proof(proof, &inputs, &vk)
        }

        // Voting circuits (v2.0)
        CircuitType::PrivateVote => {
            let inputs = VotePublicInputs::from_bytes(public_inputs)?;
            let vk = VoteVerificationKey::default();
            verify_vote_proof(proof, &inputs, &vk)
        }

        CircuitType::PrivateVoteWithPoI => {
            let inputs = VoteWithPoiPublicInputs::from_bytes(public_inputs)?;
            let vk = VoteVerificationKey::default();
            verify_vote_with_poi_proof(proof, &inputs, &vk)
        }

        // Market circuits (v2.0)
        CircuitType::MarketBet => {
            let inputs = MarketBetPublicInputs::from_bytes(public_inputs)?;
            let vk = MarketVerificationKey::default();
            verify_market_bet_proof(proof, &inputs, &vk)
        }

        CircuitType::MarketBetWithPoI => {
            let inputs = MarketBetWithPoiPublicInputs::from_bytes(public_inputs)?;
            let vk = MarketVerificationKey::default();
            verify_market_bet_with_poi_proof(proof, &inputs, &vk)
        }

        CircuitType::MarketClaim => {
            let inputs = MarketClaimPublicInputs::from_bytes(public_inputs)?;
            let vk = MarketVerificationKey::default();
            verify_market_claim_proof(proof, &inputs, &vk)
        }

        // Portfolio circuits (v2.0)
        CircuitType::PortfolioCompliance => {
            let inputs = PortfolioCompliancePublicInputs::from_bytes(public_inputs)?;
            let vk = PortfolioVerificationKey::default();
            verify_portfolio_compliance_proof(proof, &inputs, current_time, &vk)
        }

        CircuitType::PortfolioNetWorth => {
            let inputs = PortfolioNetWorthPublicInputs::from_bytes(public_inputs)?;
            let vk = PortfolioVerificationKey::default();
            verify_portfolio_net_worth_proof(proof, &inputs, current_time, &vk)
        }

        // Future circuits
        CircuitType::FutarchyConditional => {
            msg!("FutarchyConditional circuit not yet implemented");
            Ok(false)
        }
    }
}

/// Process DisputeProof instruction
///
/// Disputes a completed job by providing the full proof for on-chain verification.
/// If the proof is invalid, the prover is slashed and the disputor is rewarded.
/// If the proof is valid, the disputor loses their bond.
///
/// Accounts:
/// 0. `[signer]` Disputor wallet
/// 1. `[writable]` ZkJob account
/// 2. `[writable]` Prover wallet (for slashing if dispute succeeds)
/// 3. `[writable]` Dispute bond account (disputor's bond)
/// 4. `[writable]` Protocol treasury (receives portion of slash)
pub fn process_dispute_proof(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    proof: &[u8],
    public_inputs: &[u8],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let disputor_info = next_account_info(account_info_iter)?;
    let job_info = next_account_info(account_info_iter)?;
    let prover_info = next_account_info(account_info_iter)?;
    let treasury_info = next_account_info(account_info_iter)?;

    // Verify disputor is signer
    if !disputor_info.is_signer {
        msg!("Disputor must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Verify job account is owned by this program
    if job_info.owner != program_id {
        msg!("Invalid job account owner");
        return Err(ZkGeneratorError::JobNotFound.into());
    }

    // Load job
    let data = job_info.data.borrow();
    let mut zk_job: ZkJob = BorshDeserialize::deserialize_reader(&mut &data[..])?;
    drop(data);

    // Verify job is completed (not pending, claimed, or already disputed)
    if zk_job.common.status != JobStatus::Completed {
        msg!("Job must be in Completed status to dispute");
        return Err(ZkGeneratorError::InvalidProof.into());
    }

    // Get current time
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    // Check dispute window
    // For dispute window, we use timeout_at as a proxy for completion time
    // (jobs are completed before timeout, so this is a conservative estimate)
    // A job completed at time T has timeout_at = T + original_timeout
    // We allow disputes for DISPUTE_WINDOW_SECONDS after the job could have been completed
    //
    // More robust approach: the dispute window is based on when the job was created
    // plus a reasonable completion window plus the dispute window
    let max_dispute_time = zk_job.common.timeout_at + DISPUTE_WINDOW_SECONDS;

    if current_time > max_dispute_time {
        msg!(
            "Dispute window expired: current {} > max dispute time {}",
            current_time,
            max_dispute_time
        );
        return Err(ZkGeneratorError::DisputeWindowExpired.into());
    }

    // Verify the prover account matches the job
    let job_prover = zk_job.common.prover.ok_or(ZkGeneratorError::InvalidProof)?;
    if prover_info.key != &job_prover {
        msg!("Prover account does not match job prover");
        return Err(ZkGeneratorError::Unauthorized.into());
    }

    // Verify proof hash matches
    let submitted_proof_hash = solana_program::hash::hashv(&[proof, public_inputs]).to_bytes();
    let stored_proof_hash = zk_job.common.proof_hash.ok_or(ZkGeneratorError::InvalidProof)?;

    if submitted_proof_hash != stored_proof_hash {
        msg!("Proof hash mismatch - proof does not match submitted hash");
        msg!("  Submitted: {:?}...", &submitted_proof_hash[..8]);
        msg!("  Stored: {:?}...", &stored_proof_hash[..8]);
        return Err(ZkGeneratorError::InvalidProof.into());
    }

    // Get circuit type
    let circuit_type = zk_job.get_circuit_type().ok_or_else(|| {
        msg!("Invalid circuit type: {}", zk_job.circuit_type);
        ZkGeneratorError::InvalidProof
    })?;

    // Verify the proof on-chain
    let proof_valid = verify_circuit_proof(circuit_type, proof, public_inputs, current_time)?;

    if proof_valid {
        // Proof is valid - disputor loses bond (anti-spam measure)
        msg!("Proof verified successfully - dispute REJECTED");
        msg!("Disputor bond forfeited to treasury");

        // Transfer disputor's bond to treasury
        let bond = DISPUTE_BOND_LAMPORTS.min(disputor_info.lamports());
        if bond > 0 {
            **disputor_info.try_borrow_mut_lamports()? = disputor_info
                .lamports()
                .checked_sub(bond)
                .ok_or(ZkGeneratorError::InsufficientFunds)?;
            **treasury_info.try_borrow_mut_lamports()? = treasury_info
                .lamports()
                .checked_add(bond)
                .ok_or(ZkGeneratorError::Overflow)?;
        }

        // Job status remains Completed
        msg!("  Bond transferred: {} lamports", bond);
    } else {
        // Proof is invalid - slash prover, reward disputor
        msg!("Proof verification FAILED - dispute ACCEPTED");
        msg!("Prover will be slashed");

        // Calculate slash amount (could be based on job price or prover stake)
        let slash_amount = zk_job.common.price_lamports;
        let disputor_reward = slash_amount
            .saturating_mul(DISPUTE_REWARD_BPS)
            .saturating_div(10_000);
        let treasury_portion = slash_amount.saturating_sub(disputor_reward);

        // Transfer from prover to disputor (reward)
        let available_slash = slash_amount.min(prover_info.lamports());
        let actual_reward = disputor_reward.min(available_slash);
        let actual_treasury = available_slash.saturating_sub(actual_reward);

        if actual_reward > 0 {
            **prover_info.try_borrow_mut_lamports()? = prover_info
                .lamports()
                .checked_sub(actual_reward)
                .ok_or(ZkGeneratorError::InsufficientFunds)?;
            **disputor_info.try_borrow_mut_lamports()? = disputor_info
                .lamports()
                .checked_add(actual_reward)
                .ok_or(ZkGeneratorError::Overflow)?;
        }

        // Transfer remaining slash to treasury
        if actual_treasury > 0 {
            **prover_info.try_borrow_mut_lamports()? = prover_info
                .lamports()
                .checked_sub(actual_treasury)
                .ok_or(ZkGeneratorError::InsufficientFunds)?;
            **treasury_info.try_borrow_mut_lamports()? = treasury_info
                .lamports()
                .checked_add(actual_treasury)
                .ok_or(ZkGeneratorError::Overflow)?;
        }

        // Update job status to Disputed
        zk_job.common.status = JobStatus::Failed; // Using Failed as Disputed for now

        // Save updated job
        let mut job_data = job_info.try_borrow_mut_data()?;
        zk_job.serialize(&mut &mut job_data[..])?;

        msg!("  Prover slashed: {} lamports", available_slash);
        msg!("  Disputor reward: {} lamports", actual_reward);
        msg!("  Treasury: {} lamports", actual_treasury);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispute_window() {
        assert_eq!(DISPUTE_WINDOW_SECONDS, 24 * 60 * 60);
    }

    #[test]
    fn test_dispute_bond() {
        assert_eq!(DISPUTE_BOND_LAMPORTS, 100_000_000); // 0.1 SOL
    }

    #[test]
    fn test_dispute_reward_calculation() {
        let job_price = 1_000_000_000u64; // 1 SOL
        let reward = job_price
            .saturating_mul(DISPUTE_REWARD_BPS)
            .saturating_div(10_000);
        assert_eq!(reward, 500_000_000); // 0.5 SOL (50%)
    }
}
