use borsh::BorshDeserialize;
use zyberlink_types::CircuitType;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
};

use crate::{
    error::ZyberLinkProgramError,
    state::{JobAccount, MarketplaceConfig, ProverAccount},
};

/// Process ClaimJob instruction
pub fn process_claim_job(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let prover_authority_info = next_account_info(account_info_iter)?;
    let prover_info = next_account_info(account_info_iter)?;
    let job_info = next_account_info(account_info_iter)?;
    let config_info = next_account_info(account_info_iter)?;

    // Verify prover authority is signer
    if !prover_authority_info.is_signer {
        msg!("Prover authority must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Verify config PDA
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], program_id);
    msg!("DEBUG: Config PDA expected: {}", config_pda);
    msg!("DEBUG: Config account received: {}", config_info.key);
    msg!("DEBUG: Config account data len: {}", config_info.data.borrow().len());
    if config_info.key != &config_pda {
        msg!("Invalid config account");
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Load marketplace config
    msg!("DEBUG: Attempting to deserialize MarketplaceConfig");
    let config: MarketplaceConfig = borsh::from_slice(&config_info.data.borrow())?;
    msg!("DEBUG: MarketplaceConfig deserialized successfully");

    // Check marketplace is not paused
    if config.is_paused {
        msg!("Marketplace is paused");
        return Err(ZyberLinkProgramError::MarketplacePaused.into());
    }

    // Verify prover PDA
    let (prover_pda, _) =
        Pubkey::find_program_address(&[b"prover", prover_authority_info.key.as_ref()], program_id);

    if prover_info.key != &prover_pda {
        msg!("Invalid prover account");
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Load prover account
    let prover: ProverAccount = borsh::from_slice(&prover_info.data.borrow())?;

    // Verify prover is active
    if !prover.is_active {
        msg!("Prover is inactive");
        return Err(ZyberLinkProgramError::ProverInactive.into());
    }

    // Check prover meets reputation requirements
    if prover.reputation_score < config.min_reputation_score {
        msg!(
            "Prover reputation {} below minimum {}",
            prover.reputation_score,
            config.min_reputation_score
        );
        return Err(ZyberLinkProgramError::InsufficientReputation.into());
    }

    // Verify job account is owned by program
    if job_info.owner != program_id {
        msg!("Invalid job account owner");
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Load job account
    let mut job: JobAccount = {
        let mut data_slice = &job_info.data.borrow()[..];
        JobAccount::deserialize(&mut data_slice)?
    };

    // Verify job is in Pending status
    if job.status != zyberlink_types::JobStatus::Pending {
        msg!("Job is not in Pending status");
        return Err(ZyberLinkProgramError::JobNotPending.into());
    }

    // Get current time
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    // Handle FHE multi-prover claiming vs ZK single-prover claiming
    match &job.circuit_type {
        CircuitType::FheComputation(_) => {
            // FHE job: multi-prover support
            let config = job
                .fhe_config
                .as_ref()
                .ok_or(ZyberLinkProgramError::MissingFheConfig)?;

            // Check if job is already fully claimed
            if job.claimed_provers.len() >= config.required_provers as usize {
                msg!("FHE job already fully claimed");
                return Err(ZyberLinkProgramError::FheJobFullyClaimed.into());
            }

            // Check if this prover already claimed
            if job.claimed_provers.contains(prover_authority_info.key) {
                msg!("Prover already claimed this FHE job");
                return Err(ZyberLinkProgramError::ProverAlreadyClaimed.into());
            }

            // Add prover to claimed list
            job.claimed_provers.push(*prover_authority_info.key);

            // If this was the last required prover, mark as Claimed
            if job.claimed_provers.len() == config.required_provers as usize {
                job.status = zyberlink_types::JobStatus::Claimed;
                job.claimed_at = Some(current_time);
                msg!(
                    "FHE job fully claimed by {} provers",
                    config.required_provers
                );
            } else {
                msg!(
                    "FHE job partially claimed: {}/{}",
                    job.claimed_provers.len(),
                    config.required_provers
                );
            }
        }
        _ => {
            // ZK job: single prover (existing logic)
            job.claim(*prover_authority_info.key, current_time);
        }
    }

    // Serialize updated job back to account
    let mut job_data = job_info.try_borrow_mut_data()?;
    borsh::to_writer(&mut job_data[..], &job)?;

    msg!("Job claimed successfully");
    msg!("  Job ID: {}", job.id);
    msg!("  Prover: {}", prover_authority_info.key);
    msg!("  Timeout at: {}", job.timeout_at);

    Ok(())
}
