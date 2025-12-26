//! InitializeProtocol - Create the global ProtocolVault PDA
//!
//! This is a one-time setup instruction that creates the protocol vault
//! where all user deposits are held.

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

use crate::state::{ProtocolVault, PROTOCOL_VAULT_SEED};

/// Process InitializeProtocol instruction
///
/// Creates the global ProtocolVault PDA (lamports-only, no data)
///
/// Accounts expected:
/// 0. `[writable, signer]` Protocol authority/admin (payer)
/// 1. `[writable]` ProtocolVault PDA (will be created)
/// 2. `[]` System program
pub fn process_initialize_protocol(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let authority_info = next_account_info(account_info_iter)?;
    let protocol_vault_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    // Verify authority is signer
    if !authority_info.is_signer {
        msg!("Authority must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Derive and verify ProtocolVault PDA
    let (protocol_vault_pda, bump) = ProtocolVault::find_pda(program_id);
    if protocol_vault_pda != *protocol_vault_info.key {
        msg!("Invalid ProtocolVault PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Check if already initialized
    if protocol_vault_info.lamports() > 0 {
        msg!("ProtocolVault already initialized");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    // Create ProtocolVault PDA (0 bytes data, just rent-exempt lamports)
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(0);

    let seeds = &[PROTOCOL_VAULT_SEED, &[bump]];

    msg!("Creating ProtocolVault PDA");
    invoke_signed(
        &system_instruction::create_account(
            authority_info.key,
            protocol_vault_info.key,
            lamports,
            0, // 0 bytes - vault only holds lamports
            program_id,
        ),
        &[
            authority_info.clone(),
            protocol_vault_info.clone(),
            system_program_info.clone(),
        ],
        &[seeds],
    )?;

    msg!("ProtocolVault initialized successfully");
    msg!("  PDA: {}", protocol_vault_info.key);
    msg!("  Bump: {}", bump);
    msg!("  Initial lamports: {}", lamports);

    Ok(())
}
