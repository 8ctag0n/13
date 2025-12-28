//! CancelMarket instruction processor

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
    state::{Market, MarketStatus},
};

/// Process CancelMarket instruction
///
/// Cancels a market before settlement (authority only)
///
/// Accounts expected:
/// 0. `[writable, signer]` Market authority
/// 1. `[writable]` Market account (PDA)
pub fn process_cancel_market(
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

    // Load market
    let market_data = market_info.try_borrow_mut_data()?;
    let mut market = Market::deserialize(&mut &market_data[..])?;

    // Verify market PDA
    let (market_pda, _) = crate::cpi::derive_market_pda(program_id, market_id);
    if market_pda != *market_info.key {
        msg!("Invalid market PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Verify authority matches
    if market.authority != *authority_info.key {
        msg!("Unauthorized: not market authority");
        return Err(FutarchyError::Unauthorized.into());
    }

    // Verify market is not already settled
    if market.is_settled() {
        msg!("Cannot cancel settled market");
        return Err(FutarchyError::MarketAlreadySettled.into());
    }

    // Update market status
    market.status = MarketStatus::Cancelled;

    // Write updated market
    drop(market_data);
    let mut market_data = market_info.try_borrow_mut_data()?;
    market.serialize(&mut &mut market_data[..])?;

    msg!("Market cancelled successfully");
    msg!("  Market ID: {}", market_id);
    msg!("  Authority: {}", authority_info.key);

    // Note: Refunds would be handled by separate instruction
    // For MVP, users can claim refunds if market is cancelled

    Ok(())
}
