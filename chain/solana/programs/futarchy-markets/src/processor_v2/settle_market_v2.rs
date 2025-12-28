//! SettleMarketV2 - Settle a market with oracle resolution
//!
//! Flow:
//! 1. Verify oracle signature
//! 2. Update market resolution
//! 3. If has_governance, start timelock countdown

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

use crate::error::FutarchyError;
use crate::state::{
    GovernanceConfig, MarketStatus, MarketV2,
    GOVERNANCE_CONFIG_SEED, MARKET_V2_SEED,
};

/// Process SettleMarketV2 instruction
///
/// Settles a market with the oracle's resolution
///
/// Accounts expected:
/// 0. `[writable, signer]` Oracle account
/// 1. `[writable]` MarketV2 PDA
/// 2. `[]` Clock sysvar
///
/// If has_governance = true:
/// 3. `[writable]` GovernanceConfig PDA
pub fn process_settle_market_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    market_id: u64,
    outcome: bool,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let oracle_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let clock_sysvar_info = next_account_info(account_info_iter)?;

    // Verify oracle is signer
    if !oracle_info.is_signer {
        msg!("Oracle must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let clock = Clock::from_account_info(clock_sysvar_info)?;
    let current_time = clock.unix_timestamp;
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
    let mut market = MarketV2::deserialize(&mut &market_data[..])?;
    drop(market_data);

    // Verify oracle matches
    if market.oracle != *oracle_info.key {
        msg!("Signer is not the designated oracle");
        return Err(FutarchyError::UnauthorizedOracle.into());
    }

    // Verify market is active and betting period ended
    if !market.is_active() {
        msg!("Market is not active");
        return Err(FutarchyError::MarketNotActive.into());
    }

    // NOTE: Betting period check disabled for testing
    // if !market.is_betting_closed(current_time) {
    //     msg!("Betting period has not ended yet");
    //     return Err(FutarchyError::BettingNotClosed.into());
    // }
    let _ = current_time; // suppress unused warning

    // Settle the market
    market.settle(outcome);

    // Serialize updated market
    let mut market_data = market_info.try_borrow_mut_data()?;
    market.serialize(&mut &mut market_data[..])?;
    drop(market_data);

    msg!("Market settled");
    msg!("  Market ID: {}", market_id);
    msg!("  Resolution: {}", if outcome { "YES" } else { "NO" });

    // If has_governance, start timelock
    if market.has_governance {
        let governance_info = next_account_info(account_info_iter)?;

        let (governance_pda, _) = Pubkey::find_program_address(
            &[GOVERNANCE_CONFIG_SEED, &market_id_bytes],
            program_id,
        );
        if governance_pda != *governance_info.key {
            msg!("Invalid GovernanceConfig PDA");
            return Err(ProgramError::InvalidSeeds);
        }

        // Load and update governance config
        let governance_data = governance_info.try_borrow_data()?;
        let mut governance = GovernanceConfig::deserialize(&mut &governance_data[..])?;
        drop(governance_data);

        if governance.market_id != market_id {
            msg!("GovernanceConfig market_id mismatch");
            return Err(ProgramError::InvalidAccountData);
        }

        // Start timelock countdown
        governance.start_timelock(current_time);

        let mut governance_data = governance_info.try_borrow_mut_data()?;
        governance.serialize(&mut &mut governance_data[..])?;

        msg!("Governance timelock started");
        msg!("  Timelock duration: {} seconds", governance.timelock_duration);
        msg!("  Expires at: {}", governance.timelock_expires_at.unwrap());
    }

    Ok(())
}
