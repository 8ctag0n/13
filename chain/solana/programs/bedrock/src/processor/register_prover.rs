//! RegisterProver instruction processor

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

use crate::{error::BedrockError, state::{BedrockConfig, ProverAccount, PROVER_SEED}};

/// Process RegisterProver instruction
///
/// Accounts:
/// 0. `[signer]` Prover wallet
/// 1. `[writable]` Prover account PDA
/// 2. `[writable]` Config (to update total_provers)
/// 3. `[]` System program
pub fn process_register_prover(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    stake_lamports: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let prover_wallet_info = next_account_info(account_info_iter)?;
    let prover_account_info = next_account_info(account_info_iter)?;
    let config_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    // Verify prover wallet is signer
    if !prover_wallet_info.is_signer {
        msg!("Prover wallet must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Derive and verify prover PDA
    let (prover_pda, bump) = Pubkey::find_program_address(
        &[PROVER_SEED, prover_wallet_info.key.as_ref()],
        program_id,
    );

    if prover_account_info.key != &prover_pda {
        msg!("Invalid prover account PDA");
        return Err(BedrockError::InvalidAuthority.into());
    }

    // Check if already registered
    if !prover_account_info.data_is_empty() {
        msg!("Prover already registered");
        return Err(BedrockError::ProverAlreadyRegistered.into());
    }

    // Verify minimum stake
    if stake_lamports < ProverAccount::MIN_STAKE {
        msg!(
            "Insufficient stake: {} < {}",
            stake_lamports,
            ProverAccount::MIN_STAKE
        );
        return Err(BedrockError::InsufficientStake.into());
    }

    // Get current timestamp
    let clock = Clock::get()?;
    let current_timestamp = clock.unix_timestamp;

    // Calculate rent for prover account
    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(ProverAccount::SIZE);

    msg!("Creating prover account");

    // Create prover account
    invoke_signed(
        &system_instruction::create_account(
            prover_wallet_info.key,
            prover_account_info.key,
            rent_lamports,
            ProverAccount::SIZE as u64,
            program_id,
        ),
        &[
            prover_wallet_info.clone(),
            prover_account_info.clone(),
            system_program_info.clone(),
        ],
        &[&[PROVER_SEED, prover_wallet_info.key.as_ref(), &[bump]]],
    )?;

    // Transfer stake to prover account
    msg!("Transferring stake: {} lamports", stake_lamports);
    solana_program::program::invoke(
        &system_instruction::transfer(
            prover_wallet_info.key,
            prover_account_info.key,
            stake_lamports,
        ),
        &[
            prover_wallet_info.clone(),
            prover_account_info.clone(),
            system_program_info.clone(),
        ],
    )?;

    // Initialize prover data
    let prover = ProverAccount::new(
        *prover_wallet_info.key,
        stake_lamports,
        current_timestamp,
        bump,
    );

    // Serialize prover to account data
    let mut prover_data = prover_account_info.try_borrow_mut_data()?;
    prover.serialize(&mut &mut prover_data[..])?;

    // Update config statistics
    let mut config = BedrockConfig::deserialize(&mut &config_info.data.borrow()[..])?;
    config.total_provers = config.total_provers.saturating_add(1);

    let mut config_data = config_info.try_borrow_mut_data()?;
    config.serialize(&mut &mut config_data[..])?;

    msg!("Prover registered successfully");
    msg!("  Authority: {}", prover_wallet_info.key);
    msg!("  Stake: {} lamports", stake_lamports);
    msg!("  Total provers: {}", config.total_provers);

    Ok(())
}
