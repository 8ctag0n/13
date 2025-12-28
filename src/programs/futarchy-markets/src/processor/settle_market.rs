//! SettleMarket instruction processor

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

use crate::{
    error::FutarchyError,
    state::{Market, MarketStatus},
};

/// Process SettleMarket instruction
///
/// Settles a market with the final outcome (oracle only)
///
/// Accounts expected:
/// 0. `[writable, signer]` Oracle account
/// 1. `[writable]` Market account (PDA)
/// 2. `[]` Clock sysvar
pub fn process_settle_market(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    market_id: u64,
    outcome: bool,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let oracle_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let clock_info = next_account_info(account_info_iter)?;

    // Verify oracle is signer
    if !oracle_info.is_signer {
        msg!("Oracle must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Get current time
    let clock = Clock::from_account_info(clock_info)?;
    let current_time = clock.unix_timestamp;

    // Load market
    let market_data = market_info.try_borrow_mut_data()?;
    let mut market = Market::deserialize(&mut &market_data[..])?;

    // Verify market PDA
    let (market_pda, _) = crate::cpi::derive_market_pda(program_id, market_id);
    if market_pda != *market_info.key {
        msg!("Invalid market PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Verify oracle matches
    if market.oracle != *oracle_info.key {
        msg!("Invalid oracle");
        return Err(FutarchyError::InvalidOracle.into());
    }

    // Verify market is not already settled
    if market.is_settled() {
        msg!("Market already settled");
        return Err(FutarchyError::MarketAlreadySettled.into());
    }

    // Verify betting period has ended
    if !market.is_betting_closed(current_time) {
        msg!("Betting period has not ended yet");
        return Err(FutarchyError::BettingClosed.into());
    }

    // Update market state
    market.status = MarketStatus::Settled;
    market.resolution = Some(outcome);
    market.settled_at = Some(current_time);

    msg!("Market settled");
    msg!("  Outcome: {}", if outcome { "YES" } else { "NO" });
    msg!("  Total pool: {}", market.total_pool());
    msg!("  Winning pool: {:?}", market.winning_pool());
    msg!("  Losing pool: {:?}", market.losing_pool());

    // Governance: Check if action should be triggered
    if market.is_governance_market() {
        msg!("Checking governance execution criteria");
        msg!("  YES votes: {}%", market.yes_vote_percentage());
        msg!("  Required threshold: {}%", market.execution_threshold);

        if market.threshold_met() {
            msg!("Threshold met! Initializing timelock");

            // Calculate timelock expiry
            let timelock_expires_at = current_time
                .checked_add(market.timelock_duration)
                .ok_or(FutarchyError::Overflow)?;

            market.timelock_expires_at = Some(timelock_expires_at);

            msg!("  Timelock duration: {}s", market.timelock_duration);
            msg!("  Timelock expires at: {}", timelock_expires_at);
            msg!("  Action type: {}", market.executable_action.action_type());
            msg!("  Action will be executable after timelock");
        } else {
            msg!("Threshold NOT met - action will not be executed");
            msg!("  Vote difference: {}%", market.execution_threshold as i16 - market.yes_vote_percentage() as i16);
        }
    }

    // Write updated market
    drop(market_data);
    let mut market_data = market_info.try_borrow_mut_data()?;
    market.serialize(&mut &mut market_data[..])?;

    msg!("Market settled successfully");

    Ok(())
}
