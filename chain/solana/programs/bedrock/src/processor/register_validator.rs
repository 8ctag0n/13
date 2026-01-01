//! RegisterValidator instruction processor

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::{
    error::BedrockError,
    state::{BedrockConfig, ValidatorAccount, ValidatorRegion, VALIDATOR_SEED},
};

/// Process RegisterValidator instruction
///
/// Accounts:
/// 0. `[signer]` Validator wallet
/// 1. `[writable]` Validator account PDA
/// 2. `[writable]` Config (to update total_validators)
/// 3. `[]` System program
pub fn process_register_validator(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    endpoint: String,
    region: ValidatorRegion,
    stake: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let validator_wallet_info = next_account_info(account_info_iter)?;
    let validator_account_info = next_account_info(account_info_iter)?;
    let config_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    // Verify validator wallet is signer
    if !validator_wallet_info.is_signer {
        msg!("Validator wallet must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Verify endpoint length
    if endpoint.len() > ValidatorAccount::MAX_ENDPOINT_LENGTH {
        msg!(
            "Endpoint too long: {} > {}",
            endpoint.len(),
            ValidatorAccount::MAX_ENDPOINT_LENGTH
        );
        return Err(BedrockError::EndpointTooLong.into());
    }

    // Derive and verify validator PDA
    let (validator_pda, bump) = Pubkey::find_program_address(
        &[VALIDATOR_SEED, validator_wallet_info.key.as_ref()],
        program_id,
    );

    if validator_account_info.key != &validator_pda {
        msg!("Invalid validator account PDA");
        return Err(BedrockError::InvalidAuthority.into());
    }

    // Check if already registered
    if !validator_account_info.data_is_empty() {
        msg!("Validator already registered");
        return Err(BedrockError::ValidatorAlreadyRegistered.into());
    }

    // Load config to check minimum stake
    let mut config = BedrockConfig::deserialize(&mut &config_info.data.borrow()[..])?;

    // Verify minimum stake
    if stake < config.min_validator_stake {
        msg!(
            "Insufficient stake: {} < {}",
            stake,
            config.min_validator_stake
        );
        return Err(BedrockError::InsufficientStake.into());
    }

    // Get current timestamp
    let clock = Clock::get()?;
    let current_timestamp = clock.unix_timestamp;

    // Calculate rent for validator account
    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(ValidatorAccount::SIZE);

    msg!("Creating validator account");

    // Create validator account
    invoke_signed(
        &system_instruction::create_account(
            validator_wallet_info.key,
            validator_account_info.key,
            rent_lamports,
            ValidatorAccount::SIZE as u64,
            program_id,
        ),
        &[
            validator_wallet_info.clone(),
            validator_account_info.clone(),
            system_program_info.clone(),
        ],
        &[&[
            VALIDATOR_SEED,
            validator_wallet_info.key.as_ref(),
            &[bump],
        ]],
    )?;

    // Transfer stake to validator account
    msg!("Transferring stake: {} lamports", stake);
    solana_program::program::invoke(
        &system_instruction::transfer(
            validator_wallet_info.key,
            validator_account_info.key,
            stake,
        ),
        &[
            validator_wallet_info.clone(),
            validator_account_info.clone(),
            system_program_info.clone(),
        ],
    )?;

    // Initialize validator data
    let validator = ValidatorAccount::new(
        *validator_wallet_info.key,
        endpoint.clone(),
        region,
        stake,
        current_timestamp,
        bump,
    );

    // Serialize validator to account data
    let mut validator_data = validator_account_info.try_borrow_mut_data()?;
    validator.serialize(&mut &mut validator_data[..])?;

    // Update config statistics
    config.total_validators = config.total_validators.saturating_add(1);

    let mut config_data = config_info.try_borrow_mut_data()?;
    config.serialize(&mut &mut config_data[..])?;

    msg!("Validator registered successfully");
    msg!("  Authority: {}", validator_wallet_info.key);
    msg!("  Endpoint: {}", endpoint);
    msg!("  Region: {:?}", region);
    msg!("  Stake: {} lamports", stake);
    msg!("  Total validators: {}", config.total_validators);

    Ok(())
}
