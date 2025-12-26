//! CancelGovernanceActionV2 - Cancel a governance action (authority only)
//!
//! Can only be called by the market authority before the action is executed.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::error::FutarchyError;
use crate::state::{
    ExecutableAction, GovernanceConfig, MarketV2,
    GOVERNANCE_CONFIG_SEED, MARKET_V2_SEED,
};

/// Process CancelGovernanceActionV2 instruction
///
/// Cancels a governance action (authority only)
///
/// Accounts expected:
/// 0. `[writable, signer]` Market authority
/// 1. `[]` MarketV2 PDA
/// 2. `[writable]` GovernanceConfig PDA
pub fn process_cancel_governance_action_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    market_id: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let authority_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let governance_info = next_account_info(account_info_iter)?;

    // Verify authority is signer
    if !authority_info.is_signer {
        msg!("Authority must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let market_id_bytes = market_id.to_le_bytes();

    // Verify MarketV2 PDA
    let (market_pda, _) = Pubkey::find_program_address(
        &[MARKET_V2_SEED, &market_id_bytes],
        program_id,
    );
    if market_pda != *market_info.key {
        msg!("Invalid MarketV2 PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Load market
    let market_data = market_info.try_borrow_data()?;
    let market = MarketV2::deserialize(&mut &market_data[..])?;
    drop(market_data);

    // Verify authority matches market authority
    if market.authority != *authority_info.key {
        msg!("Signer is not the market authority");
        return Err(FutarchyError::UnauthorizedAuthority.into());
    }

    // Verify market has governance
    if !market.has_governance {
        msg!("Market does not have governance");
        return Err(FutarchyError::NotGovernanceMarket.into());
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

    // Load governance config
    let governance_data = governance_info.try_borrow_data()?;
    let mut governance = GovernanceConfig::deserialize(&mut &governance_data[..])?;
    drop(governance_data);

    // Verify action not already executed
    if governance.action_executed {
        msg!("Cannot cancel: action already executed");
        return Err(FutarchyError::ActionAlreadyExecuted.into());
    }

    // Clear the executable action (set to None)
    governance.executable_action = ExecutableAction::None;
    governance.action_executed = true; // Mark as "executed" to prevent future execution

    // Serialize updated governance config
    let mut governance_data = governance_info.try_borrow_mut_data()?;
    governance.serialize(&mut &mut governance_data[..])?;

    msg!("Governance action cancelled");
    msg!("  Market ID: {}", market_id);
    msg!("  Cancelled by: {}", authority_info.key);

    Ok(())
}
