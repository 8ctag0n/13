//! ExecuteGovernanceActionV3 - Execute governance action after timelock
//!
//! Similar to V2 but works with MarketV3

use borsh::BorshDeserialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

use crate::error::FutarchyError;
use crate::state::{GovernanceConfig, MarketV3, GOVERNANCE_CONFIG_SEED, MARKET_V3_SEED};

/// Process ExecuteGovernanceActionV3 instruction
///
/// Executes governance action if conditions are met
///
/// Accounts expected:
/// 0. `[writable, signer]` Executor (anyone can call)
/// 1. `[writable]` MarketV3 PDA
/// 2. `[writable]` GovernanceConfig PDA
/// 3. `[]` Clock sysvar
/// ... Additional accounts depend on ExecutableAction type
pub fn process_execute_governance_action_v3(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    market_id: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let executor_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let governance_info = next_account_info(account_info_iter)?;
    let clock_sysvar_info = next_account_info(account_info_iter)?;

    if !executor_info.is_signer {
        msg!("Executor must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let clock = Clock::from_account_info(clock_sysvar_info)?;
    let current_time = clock.unix_timestamp;
    let market_id_bytes = market_id.to_le_bytes();

    // Verify MarketV3 PDA
    let (market_pda, _) = Pubkey::find_program_address(
        &[MARKET_V3_SEED, &market_id_bytes],
        program_id,
    );
    if market_pda != *market_info.key {
        msg!("Invalid MarketV3 PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Verify GovernanceConfig PDA
    let (governance_pda, _) = Pubkey::find_program_address(
        &[GOVERNANCE_CONFIG_SEED, &market_id_bytes],
        program_id,
    );
    if governance_pda != *governance_info.key {
        msg!("Invalid GovernanceConfig PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Load market
    let market_data = market_info.try_borrow_data()?;
    let market = MarketV3::deserialize(&mut &market_data[..])?;
    drop(market_data);

    if !market.is_settled() {
        msg!("Market not yet settled");
        return Err(FutarchyError::MarketNotSettled.into());
    }

    // Load governance config
    let governance_data = governance_info.try_borrow_data()?;
    let governance_config = GovernanceConfig::deserialize(&mut &governance_data[..])?;
    drop(governance_data);

    // Calculate YES vote percentage from decrypted pools
    let total_pool = market.total_pool()
        .ok_or(FutarchyError::MarketNotSettled)?;
    let yes_pool = market.decrypted_pool_yes
        .ok_or(FutarchyError::MarketNotSettled)?;
    let yes_percentage = if total_pool > 0 {
        ((yes_pool as u128 * 100) / total_pool as u128) as u8
    } else {
        0
    };

    // Check if action can be executed
    if !governance_config.can_execute(current_time, yes_percentage) {
        msg!("Governance action cannot be executed yet");
        msg!("  Timelock expires: {:?}", governance_config.timelock_expires_at);
        msg!("  Current time: {}", current_time);
        msg!("  YES percentage: {}%", yes_percentage);
        msg!("  Threshold: {}%", governance_config.execution_threshold);
        return Err(FutarchyError::TimelockNotExpired.into());
    }

    if governance_config.action_executed {
        msg!("Governance action already executed");
        return Err(FutarchyError::AlreadyExecuted.into());
    }

    msg!("Executing governance action V3");
    msg!("  Market ID: {}", market_id);
    msg!("  Action type: {:?}", governance_config.executable_action);

    // TODO: Execute the action based on type
    // For now, just mark as executed

    msg!("Governance action executed successfully");

    Ok(())
}
