use borsh::BorshDeserialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::{
    error::CypherLinkProgramError,
    state::MarketplaceConfig,
};

/// Process Initialize instruction
pub fn process_initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    fee_basis_points: u16,
    min_stake_amount: u64,
    min_reputation_score: u32,
    default_job_timeout_seconds: i64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let authority_info = next_account_info(account_info_iter)?;
    let config_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    // Verify authority is signer
    if !authority_info.is_signer {
        msg!("Authority must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Derive and verify config PDA
    let (config_pda, bump) = Pubkey::find_program_address(&[b"config"], program_id);

    if config_info.key != &config_pda {
        msg!("Invalid config account");
        return Err(CypherLinkProgramError::InvalidAccount.into());
    }

    // Check if already initialized
    // If account is owned by our program and has data, it's already initialized
    if config_info.owner == program_id && config_info.data_len() > 0 {
        // Try to deserialize to verify it's properly initialized
        if let Ok(_config) = MarketplaceConfig::try_from_slice(&config_info.data.borrow()) {
            msg!("Marketplace already initialized");
            return Err(CypherLinkProgramError::AlreadyInitialized.into());
        }
    }

    // Calculate rent
    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(MarketplaceConfig::LEN);

    msg!("Creating marketplace config account");

    // Create config account
    solana_program::program::invoke_signed(
        &system_instruction::create_account(
            authority_info.key,
            config_info.key,
            rent_lamports,
            MarketplaceConfig::LEN as u64,
            program_id,
        ),
        &[
            authority_info.clone(),
            config_info.clone(),
            system_program_info.clone(),
        ],
        &[&[b"config", &[bump]]],
    )?;

    // Initialize config data
    let config = MarketplaceConfig::new(
        *authority_info.key,
        *authority_info.key, // Use authority as initial fee recipient
        bump,
    );

    // Override with custom parameters if provided
    let mut config = config;
    if fee_basis_points > 0 {
        config.fee_basis_points = fee_basis_points;
    }
    if min_stake_amount > 0 {
        config.min_stake_amount = min_stake_amount;
    }
    if min_reputation_score > 0 {
        config.min_reputation_score = min_reputation_score;
    }
    if default_job_timeout_seconds > 0 {
        config.default_job_timeout_seconds = default_job_timeout_seconds;
    }

    // Serialize config to account data
    let mut config_data = config_info.try_borrow_mut_data()?;
    borsh::to_writer(&mut config_data[..], &config)?;

    msg!("Marketplace initialized successfully");
    msg!("  Fee: {} basis points", config.fee_basis_points);
    msg!("  Min stake: {} lamports", config.min_stake_amount);
    msg!("  Min reputation: {}", config.min_reputation_score);
    msg!("  Default timeout: {}s", config.default_job_timeout_seconds);

    Ok(())
}
