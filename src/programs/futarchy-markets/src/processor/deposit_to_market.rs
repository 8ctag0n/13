//! DepositToMarket instruction processor

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::{
    error::FutarchyError,
    state::{Market, UserEscrow, USER_ESCROW_SEED},
};

/// Process DepositToMarket instruction
///
/// Deposits funds to user's escrow account for a specific market
///
/// Accounts expected:
/// 0. `[writable, signer]` User depositing
/// 1. `[writable]` UserEscrow account (PDA)
/// 2. `[]` Market account (PDA)
/// 3. `[]` System program
pub fn process_deposit_to_market(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    market_id: u64,
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let user_info = next_account_info(account_info_iter)?;
    let user_escrow_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    // Verify user is signer
    if !user_info.is_signer {
        msg!("User must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Verify amount is positive
    if amount == 0 {
        msg!("Deposit amount must be greater than 0");
        return Err(ProgramError::InvalidArgument);
    }

    // Verify market PDA
    let (market_pda, _) = crate::cpi::derive_market_pda(program_id, market_id);
    if market_pda != *market_info.key {
        msg!("Invalid market PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Load market to verify it exists and is active
    let market_data = market_info.try_borrow_data()?;
    let market = Market::deserialize(&mut &market_data[..])?;

    // Verify market is active (can't deposit to settled/cancelled markets)
    if !market.is_active() {
        msg!("Cannot deposit to inactive market");
        return Err(FutarchyError::MarketNotActive.into());
    }

    // Derive user escrow PDA
    let (user_escrow_pda, user_escrow_bump) = Pubkey::find_program_address(
        &[USER_ESCROW_SEED, user_info.key.as_ref(), market_info.key.as_ref()],
        program_id,
    );

    if user_escrow_pda != *user_escrow_info.key {
        msg!("Invalid user escrow PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Check if escrow account exists
    let escrow_exists = user_escrow_info.data_len() > 0;

    if escrow_exists {
        // Load existing escrow and add deposit
        let mut escrow_data = user_escrow_info.try_borrow_mut_data()?;
        let mut user_escrow = UserEscrow::deserialize(&mut &escrow_data[..])?;

        // Verify escrow belongs to this user and market
        if user_escrow.user != *user_info.key {
            msg!("Escrow user mismatch");
            return Err(ProgramError::InvalidAccountData);
        }

        if user_escrow.market != *market_info.key {
            msg!("Escrow market mismatch");
            return Err(ProgramError::InvalidAccountData);
        }

        // Transfer lamports to escrow account
        invoke(
            &system_instruction::transfer(user_info.key, user_escrow_info.key, amount),
            &[
                user_info.clone(),
                user_escrow_info.clone(),
                system_program_info.clone(),
            ],
        )?;

        // Update escrow balance
        user_escrow.deposit(amount)?;

        // Write updated escrow
        drop(escrow_data);
        let mut escrow_data = user_escrow_info.try_borrow_mut_data()?;
        user_escrow.serialize(&mut &mut escrow_data[..])?;

        msg!("Deposited to existing escrow");
        msg!("  User: {}", user_info.key);
        msg!("  Market: {}", market_info.key);
        msg!("  Deposit amount: {} lamports", amount);
        msg!("  New balance: {} lamports", user_escrow.deposited);
        msg!("  Available: {} lamports", user_escrow.available);
    } else {
        // Create new escrow account
        let rent = Rent::get()?;
        let escrow_space = UserEscrow::SPACE;
        let escrow_lamports = rent.minimum_balance(escrow_space);

        msg!("Creating new user escrow account");

        // Create account
        invoke(
            &system_instruction::create_account(
                user_info.key,
                user_escrow_info.key,
                escrow_lamports,
                escrow_space as u64,
                program_id,
            ),
            &[
                user_info.clone(),
                user_escrow_info.clone(),
                system_program_info.clone(),
            ],
        )?;

        // Initialize escrow state
        let user_escrow = UserEscrow::new(
            *user_info.key,
            *market_info.key,
            0,  // Initial deposit is 0, we'll add it next
            user_escrow_bump,
        );

        // Serialize initial state
        let mut escrow_data = user_escrow_info.try_borrow_mut_data()?;
        user_escrow.serialize(&mut &mut escrow_data[..])?;

        // Now transfer the deposit amount
        drop(escrow_data);
        invoke(
            &system_instruction::transfer(user_info.key, user_escrow_info.key, amount),
            &[
                user_info.clone(),
                user_escrow_info.clone(),
                system_program_info.clone(),
            ],
        )?;

        // Update escrow with deposit
        let mut escrow_data = user_escrow_info.try_borrow_mut_data()?;
        let mut user_escrow = UserEscrow::deserialize(&mut &escrow_data[..])?;
        user_escrow.deposit(amount)?;

        // Write final state
        drop(escrow_data);
        let mut escrow_data = user_escrow_info.try_borrow_mut_data()?;
        user_escrow.serialize(&mut &mut escrow_data[..])?;

        msg!("Created new user escrow and deposited");
        msg!("  User: {}", user_info.key);
        msg!("  Market: {}", market_info.key);
        msg!("  Escrow PDA: {}", user_escrow_info.key);
        msg!("  Initial deposit: {} lamports", amount);
        msg!("  Rent exempt: {} lamports", escrow_lamports);
    }

    Ok(())
}
