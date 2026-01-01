use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::{
    error::ZyberLinkProgramError,
    state::{MarketplaceConfig, ProverAccount},
};

/// Process RegisterProver instruction
pub fn process_register_prover(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    stake_amount: u64,
    encryption_pubkey: [u8; 32],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let prover_authority_info = next_account_info(account_info_iter)?;
    let prover_info = next_account_info(account_info_iter)?;
    let config_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    // Verify prover authority is signer
    if !prover_authority_info.is_signer {
        msg!("Prover authority must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Derive and verify prover PDA
    let (prover_pda, bump) =
        Pubkey::find_program_address(&[b"prover", prover_authority_info.key.as_ref()], program_id);

    if prover_info.key != &prover_pda {
        msg!("Invalid prover account");
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Check if already initialized
    if !prover_info.data_is_empty() {
        msg!("Prover already registered");
        return Err(ZyberLinkProgramError::AlreadyInitialized.into());
    }

    // Load and verify marketplace config
    let mut config: MarketplaceConfig = borsh::from_slice(&config_info.data.borrow())?;

    // Verify minimum stake requirement
    if stake_amount < config.min_stake_amount {
        msg!(
            "Insufficient stake: {} < {}",
            stake_amount,
            config.min_stake_amount
        );
        return Err(ZyberLinkProgramError::InsufficientStake.into());
    }

    // Validate encryption_pubkey is not all zeros
    if encryption_pubkey == [0u8; 32] {
        msg!("Invalid encryption_pubkey: cannot be all zeros");
        return Err(ZyberLinkProgramError::InvalidAccount.into());
    }

    // Get current timestamp
    let clock = Clock::get()?;
    let current_timestamp = clock.unix_timestamp;

    // Calculate rent
    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(ProverAccount::LEN);

    msg!("Creating prover account");

    // Create prover account
    solana_program::program::invoke_signed(
        &system_instruction::create_account(
            prover_authority_info.key,
            prover_info.key,
            rent_lamports,
            ProverAccount::LEN as u64,
            program_id,
        ),
        &[
            prover_authority_info.clone(),
            prover_info.clone(),
            system_program_info.clone(),
        ],
        &[&[b"prover", prover_authority_info.key.as_ref(), &[bump]]],
    )?;

    // Transfer stake from prover to prover account
    msg!("Transferring stake: {} lamports", stake_amount);
    solana_program::program::invoke(
        &system_instruction::transfer(prover_authority_info.key, prover_info.key, stake_amount),
        &[
            prover_authority_info.clone(),
            prover_info.clone(),
            system_program_info.clone(),
        ],
    )?;

    // Initialize prover data
    let prover = ProverAccount::new(
        *prover_authority_info.key,
        stake_amount,
        current_timestamp,
        encryption_pubkey,
        bump,
    );

    // Serialize prover to account data
    let mut prover_data = prover_info.try_borrow_mut_data()?;
    borsh::to_writer(&mut prover_data[..], &prover)?;

    // Update marketplace statistics
    config.total_provers = config.total_provers.saturating_add(1);

    // Save updated config
    let mut config_data = config_info.try_borrow_mut_data()?;
    borsh::to_writer(&mut config_data[..], &config)?;

    msg!("Prover registered successfully");
    msg!("  Authority: {}", prover_authority_info.key);
    msg!("  Stake: {} lamports", stake_amount);
    msg!("  Initial reputation: {}", prover.reputation_score);
    msg!("  Encryption pubkey: {:?}", &encryption_pubkey[..8]);

    Ok(())
}
