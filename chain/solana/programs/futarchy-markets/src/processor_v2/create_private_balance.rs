//! CreatePrivateBalance - Create a user's PrivateBalance account
//!
//! Initializes a new PrivateBalance PDA for a user with:
//! - Initial FHE encrypted balance (encryption of 0)
//! - Initial balance commitment (Poseidon(0, nonce))

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

use crate::state::{PrivateBalance, PRIVATE_BALANCE_SEED, MAX_FHE_CIPHERTEXT_SIZE};

/// Process CreatePrivateBalance instruction
///
/// Creates a new PrivateBalance account for the user
///
/// Accounts expected:
/// 0. `[writable, signer]` User (payer and owner)
/// 1. `[writable]` PrivateBalance PDA (will be created)
/// 2. `[]` System program
pub fn process_create_private_balance(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    initial_encrypted_balance: Vec<u8>,
    initial_commitment: [u8; 32],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let user_info = next_account_info(account_info_iter)?;
    let private_balance_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    // Verify user is signer
    if !user_info.is_signer {
        msg!("User must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate encrypted balance size
    if initial_encrypted_balance.len() > MAX_FHE_CIPHERTEXT_SIZE {
        msg!("Encrypted balance too large: {} > {}",
             initial_encrypted_balance.len(), MAX_FHE_CIPHERTEXT_SIZE);
        return Err(ProgramError::InvalidArgument);
    }

    // Derive and verify PrivateBalance PDA
    let (private_balance_pda, bump) = Pubkey::find_program_address(
        &[PRIVATE_BALANCE_SEED, user_info.key.as_ref()],
        program_id,
    );
    if private_balance_pda != *private_balance_info.key {
        msg!("Invalid PrivateBalance PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Check if already initialized
    if private_balance_info.data_len() > 0 {
        msg!("PrivateBalance already exists for this user");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    // Create PrivateBalance PDA
    let rent = Rent::get()?;
    let space = PrivateBalance::SPACE;
    let lamports = rent.minimum_balance(space);

    let seeds = &[PRIVATE_BALANCE_SEED, user_info.key.as_ref(), &[bump]];

    msg!("Creating PrivateBalance PDA");
    invoke_signed(
        &system_instruction::create_account(
            user_info.key,
            private_balance_info.key,
            lamports,
            space as u64,
            program_id,
        ),
        &[
            user_info.clone(),
            private_balance_info.clone(),
            system_program_info.clone(),
        ],
        &[seeds],
    )?;

    // Initialize PrivateBalance state
    let private_balance = PrivateBalance::new(
        *user_info.key,
        initial_encrypted_balance,
        initial_commitment,
        bump,
    );

    // Serialize to account
    let mut data = private_balance_info.try_borrow_mut_data()?;
    private_balance.serialize(&mut &mut data[..])?;

    msg!("PrivateBalance created successfully");
    msg!("  User: {}", user_info.key);
    msg!("  PDA: {}", private_balance_info.key);
    msg!("  Bump: {}", bump);
    msg!("  Initial nonce: 0");

    Ok(())
}
