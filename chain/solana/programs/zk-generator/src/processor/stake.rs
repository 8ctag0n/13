//! Prover stake management instructions
//!
//! - RegisterProver: Create stake account and deposit initial stake
//! - DepositStake: Add more stake
//! - WithdrawStake: Withdraw available stake

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
    error::ZkGeneratorError,
    state::{ProverStake, MIN_PROVER_STAKE, PROVER_STAKE_SEED},
};

/// Process RegisterProver instruction
///
/// Creates a prover stake account with initial stake.
///
/// Accounts:
/// 0. `[signer]` Prover wallet
/// 1. `[writable]` ProverStake PDA
/// 2. `[]` System program
pub fn process_register_prover(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    stake_amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let prover_info = next_account_info(account_info_iter)?;
    let stake_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    // Verify prover is signer
    if !prover_info.is_signer {
        msg!("Prover must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Verify minimum stake
    if stake_amount < MIN_PROVER_STAKE {
        msg!(
            "Stake amount {} is below minimum {}",
            stake_amount,
            MIN_PROVER_STAKE
        );
        return Err(ZkGeneratorError::InsufficientFunds.into());
    }

    // Derive and verify stake PDA
    let (stake_pda, stake_bump) =
        Pubkey::find_program_address(&[PROVER_STAKE_SEED, prover_info.key.as_ref()], program_id);

    if stake_info.key != &stake_pda {
        msg!("Invalid stake account PDA");
        return Err(ZkGeneratorError::InvalidInstruction.into());
    }

    // Check if already registered
    if !stake_info.data_is_empty() {
        msg!("Prover already registered");
        return Err(ZkGeneratorError::JobAlreadyClaimed.into());
    }

    // Get current time
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    // Calculate rent
    let rent = Rent::get()?;
    let stake_rent = rent.minimum_balance(ProverStake::SIZE);

    msg!("Registering prover with {} lamports stake", stake_amount);

    // Create stake account
    invoke_signed(
        &system_instruction::create_account(
            prover_info.key,
            stake_info.key,
            stake_rent + stake_amount,
            ProverStake::SIZE as u64,
            program_id,
        ),
        &[
            prover_info.clone(),
            stake_info.clone(),
            system_program_info.clone(),
        ],
        &[&[PROVER_STAKE_SEED, prover_info.key.as_ref(), &[stake_bump]]],
    )?;

    // Initialize stake data
    let prover_stake = ProverStake::new(*prover_info.key, stake_amount, current_time, stake_bump);

    let mut stake_data = stake_info.try_borrow_mut_data()?;
    prover_stake.serialize(&mut &mut stake_data[..])?;

    msg!("Prover registered successfully");
    msg!("  Prover: {}", prover_info.key);
    msg!("  Stake: {} lamports", stake_amount);

    Ok(())
}

/// Process DepositStake instruction
///
/// Add more stake to an existing prover account.
///
/// Accounts:
/// 0. `[signer]` Prover wallet
/// 1. `[writable]` ProverStake PDA
/// 2. `[]` System program
pub fn process_deposit_stake(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let prover_info = next_account_info(account_info_iter)?;
    let stake_info = next_account_info(account_info_iter)?;
    let _system_program_info = next_account_info(account_info_iter)?;

    // Verify prover is signer
    if !prover_info.is_signer {
        msg!("Prover must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Verify stake account owned by program
    if stake_info.owner != program_id {
        msg!("Invalid stake account owner");
        return Err(ZkGeneratorError::Unauthorized.into());
    }

    // Load stake
    let data = stake_info.data.borrow();
    let mut prover_stake: ProverStake = BorshDeserialize::deserialize_reader(&mut &data[..])?;
    drop(data);

    // Verify prover matches
    if prover_stake.prover != *prover_info.key {
        msg!("Prover mismatch");
        return Err(ZkGeneratorError::Unauthorized.into());
    }

    // Transfer lamports from prover to stake account
    **prover_info.try_borrow_mut_lamports()? = prover_info
        .lamports()
        .checked_sub(amount)
        .ok_or(ZkGeneratorError::InsufficientFunds)?;
    **stake_info.try_borrow_mut_lamports()? = stake_info
        .lamports()
        .checked_add(amount)
        .ok_or(ZkGeneratorError::Overflow)?;

    // Update stake data
    prover_stake.deposit(amount);

    let mut stake_data = stake_info.try_borrow_mut_data()?;
    prover_stake.serialize(&mut &mut stake_data[..])?;

    msg!("Stake deposited: {} lamports", amount);
    msg!("  New total: {} lamports", prover_stake.staked_amount);

    Ok(())
}

/// Process WithdrawStake instruction
///
/// Withdraw available stake (not locked in jobs).
///
/// Accounts:
/// 0. `[signer]` Prover wallet
/// 1. `[writable]` ProverStake PDA
pub fn process_withdraw_stake(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let prover_info = next_account_info(account_info_iter)?;
    let stake_info = next_account_info(account_info_iter)?;

    // Verify prover is signer
    if !prover_info.is_signer {
        msg!("Prover must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Verify stake account owned by program
    if stake_info.owner != program_id {
        msg!("Invalid stake account owner");
        return Err(ZkGeneratorError::Unauthorized.into());
    }

    // Load stake
    let data = stake_info.data.borrow();
    let mut prover_stake: ProverStake = BorshDeserialize::deserialize_reader(&mut &data[..])?;
    drop(data);

    // Verify prover matches
    if prover_stake.prover != *prover_info.key {
        msg!("Prover mismatch");
        return Err(ZkGeneratorError::Unauthorized.into());
    }

    // Check available stake
    let available = prover_stake.available_stake();
    if amount > available {
        msg!("Insufficient available stake: {} < {}", available, amount);
        return Err(ZkGeneratorError::InsufficientFunds.into());
    }

    // Withdraw
    let withdrawn = prover_stake.withdraw(amount);

    // Transfer lamports from stake account to prover
    **stake_info.try_borrow_mut_lamports()? = stake_info
        .lamports()
        .checked_sub(withdrawn)
        .ok_or(ZkGeneratorError::InsufficientFunds)?;
    **prover_info.try_borrow_mut_lamports()? = prover_info
        .lamports()
        .checked_add(withdrawn)
        .ok_or(ZkGeneratorError::Overflow)?;

    // Update stake data
    let mut stake_data = stake_info.try_borrow_mut_data()?;
    prover_stake.serialize(&mut &mut stake_data[..])?;

    msg!("Stake withdrawn: {} lamports", withdrawn);
    msg!("  Remaining: {} lamports", prover_stake.staked_amount);

    Ok(())
}
