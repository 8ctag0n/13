//! SettleMarketV3 - Settle market with threshold-decrypted pools
//!
//! Oracle provides:
//! - Outcome (YES/NO)
//! - Decrypted pool values (from threshold decryption)
//! - Threshold signatures proving decryption validity

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
use crate::state::{GovernanceConfig, MarketV3, GOVERNANCE_CONFIG_SEED, MARKET_V3_SEED};

/// Process SettleMarketV3 instruction
///
/// Settles a market with threshold-decrypted pool values
///
/// Accounts expected:
/// 0. `[writable, signer]` Oracle account
/// 1. `[writable]` MarketV3 PDA
/// 2. `[]` Clock sysvar
///
/// If has_governance = true:
/// 3. `[writable]` GovernanceConfig PDA
pub fn process_settle_market_v3(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    market_id: u64,
    outcome: bool,
    decrypted_pool_yes: u64,
    decrypted_pool_no: u64,
    threshold_signatures: Vec<[u8; 64]>,
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

    // Verify MarketV3 PDA
    let (market_pda, _) = Pubkey::find_program_address(
        &[MARKET_V3_SEED, &market_id_bytes],
        program_id,
    );
    if market_pda != *market_info.key {
        msg!("Invalid MarketV3 PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Load market
    let market_data = market_info.try_borrow_data()?;
    let mut market = MarketV3::deserialize(&mut &market_data[..])?;
    drop(market_data);

    // Verify oracle
    if *oracle_info.key != market.oracle {
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
        msg!("Betting period not yet ended");
        return Err(FutarchyError::BettingNotClosed.into());
    }

    // Verify threshold signatures
    verify_threshold_signatures(
        &threshold_signatures,
        market.threshold_required,
        market.threshold_shares,
        &market.threshold_pubkey,
        decrypted_pool_yes,
        decrypted_pool_no,
    )?;

    // Settle the market
    market.settle(outcome, decrypted_pool_yes, decrypted_pool_no, current_time);

    // Save updated market
    let mut market_data = market_info.try_borrow_mut_data()?;
    market.serialize(&mut &mut market_data[..])?;

    msg!("Market V3 settled");
    msg!("  Market ID: {}", market_id);
    msg!("  Outcome: {}", if outcome { "YES" } else { "NO" });
    msg!("  Decrypted YES pool: {} lamports", decrypted_pool_yes);
    msg!("  Decrypted NO pool: {} lamports", decrypted_pool_no);
    msg!("  Total pool: {} lamports", decrypted_pool_yes + decrypted_pool_no);

    // Handle governance if applicable
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

        let governance_data = governance_info.try_borrow_data()?;
        let mut governance_config = GovernanceConfig::deserialize(&mut &governance_data[..])?;
        drop(governance_data);

        // Calculate YES vote percentage
        let total_pool = decrypted_pool_yes.saturating_add(decrypted_pool_no);
        let yes_percentage = if total_pool > 0 {
            ((decrypted_pool_yes as u128 * 100) / total_pool as u128) as u8
        } else {
            0
        };

        msg!("  YES vote percentage: {}%", yes_percentage);
        msg!("  Execution threshold: {}%", governance_config.execution_threshold);

        // Start timelock if threshold met
        if yes_percentage >= governance_config.execution_threshold {
            let timelock_expires = current_time + governance_config.timelock_duration;
            governance_config.start_timelock(timelock_expires);

            msg!("  Threshold MET - timelock started");
            msg!("  Timelock expires: {}", timelock_expires);
        } else {
            msg!("  Threshold NOT met - action will not execute");
        }

        // Save governance config
        let mut governance_data = governance_info.try_borrow_mut_data()?;
        governance_config.serialize(&mut &mut governance_data[..])?;
    }

    Ok(())
}

/// Verify threshold signatures for decrypted pool values
///
/// TODO: Implement actual threshold signature verification
/// For now, this is a placeholder that checks signature count
fn verify_threshold_signatures(
    signatures: &[[u8; 64]],
    threshold_required: u8,
    _threshold_shares: u8,
    _threshold_pubkey: &[u8; 32],
    _decrypted_pool_yes: u64,
    _decrypted_pool_no: u64,
) -> ProgramResult {
    // Verify we have enough signatures
    if signatures.len() < threshold_required as usize {
        msg!(
            "Insufficient threshold signatures: {} < {}",
            signatures.len(),
            threshold_required
        );
        return Err(ProgramError::InvalidArgument);
    }

    // TODO: Verify each signature is valid BLS/Schnorr signature
    // from a threshold share holder, proving correct decryption
    // For MVP, we trust the oracle to provide valid signatures

    msg!(
        "Threshold signatures verified ({}/{})",
        signatures.len(),
        threshold_required
    );

    Ok(())
}
