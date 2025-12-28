//! WithdrawFromEscrow instruction processor

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    error::FutarchyError,
    state::{Market, UserEscrow, USER_ESCROW_SEED},
};

/// Process WithdrawFromEscrow instruction
///
/// Withdraws available funds from user's escrow account
///
/// Accounts expected:
/// 0. `[writable, signer]` User withdrawing
/// 1. `[writable]` UserEscrow account (PDA)
/// 2. `[]` Market account (PDA)
/// 3. `[]` System program
pub fn process_withdraw_from_escrow(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    market_id: u64,
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let user_info = next_account_info(account_info_iter)?;
    let user_escrow_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let _system_program_info = next_account_info(account_info_iter)?;

    // Verify user is signer
    if !user_info.is_signer {
        msg!("User must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Verify amount is positive
    if amount == 0 {
        msg!("Withdrawal amount must be greater than 0");
        return Err(ProgramError::InvalidArgument);
    }

    // Verify market PDA
    let (market_pda, _) = crate::cpi::derive_market_pda(program_id, market_id);
    if market_pda != *market_info.key {
        msg!("Invalid market PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Load market to verify it exists
    let market_data = market_info.try_borrow_data()?;
    let market = Market::deserialize(&mut &market_data[..])?;
    drop(market_data);

    // Note: We allow withdrawals even if market is settled/cancelled
    // Users can withdraw their remaining available balance at any time

    // Verify user escrow PDA
    let (user_escrow_pda, _) = Pubkey::find_program_address(
        &[USER_ESCROW_SEED, user_info.key.as_ref(), market_info.key.as_ref()],
        program_id,
    );

    if user_escrow_pda != *user_escrow_info.key {
        msg!("Invalid user escrow PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Load escrow
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

    // Verify user has sufficient available balance
    if amount > user_escrow.available {
        msg!("Insufficient available balance");
        msg!("  Requested: {} lamports", amount);
        msg!("  Available: {} lamports", user_escrow.available);
        msg!("  Reserved: {} lamports", user_escrow.reserved);
        return Err(ProgramError::InsufficientFunds);
    }

    // Update escrow state
    user_escrow.withdraw(amount)?;

    // Drop borrow before modifying lamports
    drop(escrow_data);

    // Transfer lamports from escrow to user
    // We do this by decreasing escrow's lamports and increasing user's lamports
    let escrow_lamports = user_escrow_info.lamports();

    if escrow_lamports < amount {
        msg!("Escrow account doesn't have sufficient lamports");
        msg!("  Escrow lamports: {}", escrow_lamports);
        msg!("  Withdrawal amount: {}", amount);
        return Err(ProgramError::InsufficientFunds);
    }

    **user_escrow_info.try_borrow_mut_lamports()? -= amount;
    **user_info.try_borrow_mut_lamports()? += amount;

    // Write updated escrow
    let mut escrow_data = user_escrow_info.try_borrow_mut_data()?;
    user_escrow.serialize(&mut &mut escrow_data[..])?;

    msg!("Withdrawal successful");
    msg!("  User: {}", user_info.key);
    msg!("  Market: {}", market_info.key);
    msg!("  Withdrawn: {} lamports", amount);
    msg!("  Remaining deposited: {} lamports", user_escrow.deposited);
    msg!("  Remaining available: {} lamports", user_escrow.available);
    msg!("  Reserved in bets: {} lamports", user_escrow.reserved);

    Ok(())
}
