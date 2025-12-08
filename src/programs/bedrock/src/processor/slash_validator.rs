//! SlashValidator instruction processor

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
    state::{BedrockConfig, ValidatorAccount, VALIDATOR_SEED},
};

/// Process SlashValidator instruction
///
/// Accounts:
/// 0. `[signer]` Admin (from config)
/// 1. `[writable]` Validator account
/// 2. `[]` Config
pub fn process_slash_validator(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
    reason: SlashReason,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let admin_info = next_account_info(account_info_iter)?;
    let validator_account_info = next_account_info(account_info_iter)?;
    let config_info = next_account_info(account_info_iter)?;

    // Verify admin is signer
    if !admin_info.is_signer {
        msg!("Admin must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify config
    let config = BedrockConfig::try_from_slice(&config_info.data.borrow())?;

    if admin_info.key != &config.admin {
        msg!("Invalid admin authority");
        return Err(BedrockError::InvalidAuthority.into());
    }

    // Load validator account
    let mut validator = ValidatorAccount::try_from_slice(&validator_account_info.data.borrow())?;

    // Verify validator PDA
    let (validator_pda, _bump) = Pubkey::find_program_address(
        &[VALIDATOR_SEED, validator.authority.as_ref()],
        program_id,
    );

    if validator_account_info.key != &validator_pda {
        msg!("Invalid validator account PDA");
        return Err(BedrockError::InvalidAuthority.into());
    }

    // Check if validator has enough stake
    if validator.stake < amount {
        msg!(
            "Insufficient stake to slash: {} < {}",
            validator.stake,
            amount
        );
        return Err(BedrockError::InsufficientStake.into());
    }

    // Get current timestamp
    let clock = Clock::get()?;

    // Slash the validator
    let slashed_amount = validator.slash(amount, config.min_validator_stake);
    validator.record_failure(clock.unix_timestamp);

    msg!("Validator slashed");
    msg!("  Validator: {}", validator.authority);
    msg!("  Amount: {} lamports", slashed_amount);
    msg!("  Reason: {:?}", reason);
    msg!("  Remaining stake: {} lamports", validator.stake);
    msg!("  Active: {}", validator.is_active);

    // Save updated validator
    let mut validator_data = validator_account_info.try_borrow_mut_data()?;
    validator.serialize(&mut &mut validator_data[..])?;

    Ok(())
}
