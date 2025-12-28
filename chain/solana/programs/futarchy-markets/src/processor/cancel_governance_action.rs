//! CancelGovernanceAction instruction processor

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
    state::{Market, MARKET_SEED},
};

/// Process CancelGovernanceAction instruction
///
/// Cancels a pending governance action (authority only)
///
/// Accounts expected:
/// 0. `[writable, signer]` Market authority
/// 1. `[writable]` Market account (PDA)
pub fn process_cancel_governance_action(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    market_id: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let authority_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;

    // Verify authority is signer
    if !authority_info.is_signer {
        msg!("Authority must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    msg!("Canceling governance action for market {}", market_id);

    // Derive market PDA
    let market_seeds = &[MARKET_SEED, &market_id.to_le_bytes()];
    let (market_pda, _) = Pubkey::find_program_address(market_seeds, program_id);

    if market_pda != *market_info.key {
        msg!("Invalid market PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Load market
    let mut market_data = market_info.try_borrow_mut_data()?;
    let mut market = Market::deserialize(&mut &market_data[..])?;

    // Verify signer is market authority
    if market.authority != *authority_info.key {
        msg!("Only market authority can cancel governance action");
        return Err(FutarchyError::Unauthorized.into());
    }

    // Verify is governance market
    if !market.is_governance_market() {
        msg!("Market has no executable action to cancel");
        return Err(FutarchyError::NoActionConfigured.into());
    }

    // Verify action not already executed
    if market.action_executed {
        msg!("Cannot cancel: action already executed");
        return Err(FutarchyError::ActionAlreadyExecuted.into());
    }

    msg!("Canceling governance action");
    msg!("  Action type: {}", market.executable_action.action_type());

    // Mark action as executed (prevents future execution)
    market.action_executed = true;

    // Save market state
    drop(market_data); // Release borrow
    let mut market_data = market_info.try_borrow_mut_data()?;
    market.serialize(&mut &mut market_data[..])?;

    msg!("Governance action canceled successfully");

    Ok(())
}
