//! Initialize instruction processor

use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::{error::BedrockError, state::BedrockConfig, CONFIG_SEED};

/// Process Initialize instruction
///
/// Accounts:
/// 0. `[signer]` Admin (authority)
/// 1. `[writable]` Config PDA
/// 2. `[]` System program
pub fn process_initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    zk_generator_program: Pubkey,
    fhe_generator_program: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let admin_info = next_account_info(account_info_iter)?;
    let config_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    // Verify admin is signer
    if !admin_info.is_signer {
        msg!("Admin must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Derive and verify config PDA
    let (config_pda, bump) = Pubkey::find_program_address(&[CONFIG_SEED], program_id);

    if config_info.key != &config_pda {
        msg!("Invalid config account");
        return Err(BedrockError::InvalidAuthority.into());
    }

    // Check if already initialized
    if config_info.owner == program_id && !config_info.data_is_empty() {
        msg!("Bedrock already initialized");
        return Err(BedrockError::AlreadyInitialized.into());
    }

    // Calculate rent
    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(BedrockConfig::SIZE);

    msg!("Creating Bedrock config account");

    // Create config account
    invoke_signed(
        &system_instruction::create_account(
            admin_info.key,
            config_info.key,
            rent_lamports,
            BedrockConfig::SIZE as u64,
            program_id,
        ),
        &[
            admin_info.clone(),
            config_info.clone(),
            system_program_info.clone(),
        ],
        &[&[CONFIG_SEED, &[bump]]],
    )?;

    // Initialize config data
    let config = BedrockConfig::new(*admin_info.key, zk_generator_program, fhe_generator_program, bump);

    // Serialize config to account data
    let mut config_data = config_info.try_borrow_mut_data()?;
    config.serialize(&mut &mut config_data[..])?;

    msg!("Bedrock initialized successfully");
    msg!("  Admin: {}", admin_info.key);
    msg!("  ZK Generator: {}", zk_generator_program);
    msg!("  FHE Generator: {}", fhe_generator_program);

    Ok(())
}
