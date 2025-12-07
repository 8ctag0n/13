//! SlashProver instruction processor

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

use crate::{
    error::BedrockError,
    instruction::SlashReason,
    state::{BedrockConfig, ProverAccount},
};

/// Process SlashProver instruction (CPI from generators)
///
/// Accounts:
/// 0. `[signer]` Generator program (must be registered)
/// 1. `[writable]` Prover account
/// 2. `[]` Config
/// 3. `[writable]` Slash recipient (treasury or refund to creator)
pub fn process_slash_prover(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
    reason: SlashReason,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let caller_info = next_account_info(account_info_iter)?;
    let prover_account_info = next_account_info(account_info_iter)?;
    let config_info = next_account_info(account_info_iter)?;
    let recipient_info = next_account_info(account_info_iter)?;

    // Verify caller is signer (will be generator program via CPI)
    if !caller_info.is_signer {
        msg!("Caller must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load config and verify caller is a registered generator
    let config = BedrockConfig::try_from_slice(&config_info.data.borrow())?;

    if !config.is_registered_generator(caller_info.key) {
        msg!("Caller is not a registered generator program");
        return Err(BedrockError::InvalidAuthority.into());
    }

    // Verify prover account is owned by this program
    if prover_account_info.owner != program_id {
        msg!("Invalid prover account owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    // Load prover
    let mut prover = ProverAccount::try_from_slice(&prover_account_info.data.borrow())?;

    // Slash the prover
    let slashed_amount = prover.slash(amount);

    if slashed_amount == 0 {
        msg!("Prover has no stake to slash");
        return Ok(());
    }

    // Record failure
    let clock = Clock::get()?;
    prover.record_failure(clock.unix_timestamp);

    // Transfer slashed lamports from prover account to recipient
    **prover_account_info.try_borrow_mut_lamports()? -= slashed_amount;
    **recipient_info.try_borrow_mut_lamports()? += slashed_amount;

    // Save updated prover
    let mut prover_data = prover_account_info.try_borrow_mut_data()?;
    prover.serialize(&mut &mut prover_data[..])?;

    msg!("Prover slashed successfully");
    msg!("  Amount: {} lamports", slashed_amount);
    msg!("  Reason: {:?}", reason);
    msg!("  Remaining stake: {} lamports", prover.stake_lamports);
    msg!("  Active: {}", prover.is_active);

    Ok(())
}
