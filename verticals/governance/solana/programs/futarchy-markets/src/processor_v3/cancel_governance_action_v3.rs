//! CancelGovernanceActionV3 - Cancel governance action (authority only)
//!
//! Similar to V2 but works with MarketV3

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::error::FutarchyError;
use crate::state::{GovernanceConfig, MarketV3, GOVERNANCE_CONFIG_SEED, MARKET_V3_SEED};

/// Process CancelGovernanceActionV3 instruction
///
/// Cancels a governance action (market authority only)
///
/// Accounts expected:
/// 0. `[writable, signer]` Market authority
/// 1. `[]` MarketV3 PDA
/// 2. `[writable]` GovernanceConfig PDA
pub fn process_cancel_governance_action_v3(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    market_id: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let authority_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let governance_info = next_account_info(account_info_iter)?;

    if !authority_info.is_signer {
        msg!("Authority must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

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

    // Verify authority
    if *authority_info.key != market.authority {
        msg!("Invalid authority");
        return Err(FutarchyError::Unauthorized.into());
    }

    // Load governance config
    let governance_data = governance_info.try_borrow_data()?;
    let mut governance_config = GovernanceConfig::deserialize(&mut &governance_data[..])?;
    drop(governance_data);

    if governance_config.action_executed {
        msg!("Cannot cancel already executed action");
        return Err(FutarchyError::AlreadyExecuted.into());
    }

    // Cancel the action by resetting timelock
    governance_config.timelock_expires_at = None;

    // Save updated governance config
    let mut governance_data = governance_info.try_borrow_mut_data()?;
    governance_config.serialize(&mut &mut governance_data[..])?;

    msg!("Governance action V3 cancelled");
    msg!("  Market ID: {}", market_id);

    Ok(())
}
