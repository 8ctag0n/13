use borsh::BorshDeserialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    error::ZyberLinkProgramError,
    state::{JobAccount, MarketplaceConfig, ProverAccount},
};

/// Process SlashProver instruction
pub fn process_slash_prover(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    slash_amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let authority_info = next_account_info(account_info_iter)?;
    let prover_info = next_account_info(account_info_iter)?;
    let job_info = next_account_info(account_info_iter)?;
    let protocol_fee_recipient_info = next_account_info(account_info_iter)?;
    let config_info = next_account_info(account_info_iter)?;

    // Verify authority is signer
    if !authority_info.is_signer {
        msg!("Authority must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Verify config PDA
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], program_id);
    if config_info.key != &config_pda {
        msg!("Invalid config account");
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Load marketplace config
    let config: MarketplaceConfig = borsh::from_slice(&config_info.data.borrow())?;

    // Verify authority matches config
    if config.authority != *authority_info.key {
        msg!("Only marketplace authority can slash provers");
        return Err(ZyberLinkProgramError::Unauthorized.into());
    }

    // Verify protocol fee recipient matches
    if protocol_fee_recipient_info.key != &config.protocol_fee_recipient {
        msg!("Invalid protocol fee recipient");
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Load prover account
    let mut prover: ProverAccount = borsh::from_slice(&prover_info.data.borrow())?;

    // Verify prover account is owned by program
    if prover_info.owner != program_id {
        msg!("Invalid prover account owner");
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Load job account (as evidence)
    let job: JobAccount = {
        let mut data_slice = &job_info.data.borrow()[..];
        JobAccount::deserialize(&mut data_slice)?
    };

    // Verify job is related to this prover (as evidence of misbehavior)
    if job.prover != Some(prover.authority) {
        msg!("Job does not belong to this prover");
        return Err(ZyberLinkProgramError::InvalidProver.into());
    }

    // Verify we can slash this amount
    if slash_amount > prover.stake_amount {
        msg!(
            "Slash amount {} exceeds prover stake {}",
            slash_amount,
            prover.stake_amount
        );
        return Err(ZyberLinkProgramError::InsufficientFunds.into());
    }

    msg!("Slashing prover");
    msg!("  Prover: {}", prover.authority);
    msg!("  Slash amount: {} lamports", slash_amount);
    msg!("  Current stake: {} lamports", prover.stake_amount);

    // Transfer slashed amount from prover to protocol (manual lamport transfer)
    **prover_info.lamports.borrow_mut() = prover_info
        .lamports()
        .checked_sub(slash_amount)
        .ok_or(ZyberLinkProgramError::InsufficientFunds)?;

    **protocol_fee_recipient_info.lamports.borrow_mut() = protocol_fee_recipient_info
        .lamports()
        .checked_add(slash_amount)
        .ok_or(ZyberLinkProgramError::Overflow)?;

    // Update prover stake
    prover.stake_amount = prover
        .stake_amount
        .checked_sub(slash_amount)
        .ok_or(ZyberLinkProgramError::Overflow)?;

    // Increment failed jobs counter
    prover.total_jobs_failed = prover
        .total_jobs_failed
        .checked_add(1)
        .ok_or(ZyberLinkProgramError::Overflow)?;

    // Reduce reputation (e.g., reduce by 10% or a fixed amount)
    let reputation_penalty = 100; // Reduce by 100 points
    prover.reputation_score = prover.reputation_score.saturating_sub(reputation_penalty);

    // Deactivate prover if stake falls below minimum
    if prover.stake_amount < config.min_stake_amount {
        msg!("Prover stake below minimum, deactivating prover");
        prover.is_active = false;
    }

    // Serialize updated prover
    let mut prover_data = prover_info.try_borrow_mut_data()?;
    borsh::to_writer(&mut prover_data[..], &prover)?;

    msg!("Prover slashed successfully");
    msg!("  New stake: {} lamports", prover.stake_amount);
    msg!("  New reputation: {}", prover.reputation_score);
    msg!("  Active: {}", prover.is_active);

    Ok(())
}
