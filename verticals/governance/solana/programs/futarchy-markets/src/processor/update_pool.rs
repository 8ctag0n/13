//! UpdatePool instruction processor
//!
//! Called after FHE consensus is reached to update encrypted pools

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
    state::Market,
};

/// Process UpdatePool instruction
///
/// Updates market's encrypted pool after FHE job completes
///
/// Accounts expected:
/// 0. `[writable, signer]` Updater (can be anyone after consensus)
/// 1. `[writable]` Market account (PDA)
/// 2. `[]` FHE job account (for verification)
/// 3. `[]` FHE consensus account (for verification)
pub fn process_update_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    market_id: u64,
    fhe_job_id: u64,
    encrypted_result: Vec<u8>,
    side: bool, // true = YES, false = NO
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let updater_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let fhe_job_info = next_account_info(account_info_iter)?;
    let fhe_consensus_info = next_account_info(account_info_iter)?;

    // Verify updater is signer
    if !updater_info.is_signer {
        msg!("Updater must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load market
    let mut market_data = market_info.try_borrow_mut_data()?;
    let mut market = Market::deserialize(&mut &market_data[..])?;

    // Verify market PDA
    let (market_pda, _) = crate::cpi::derive_market_pda(program_id, market_id);
    if market_pda != *market_info.key {
        msg!("Invalid market PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Verify this market has a pending job
    match market.pending_pool_update_job {
        Some(pending_job_id) => {
            if pending_job_id != fhe_job_id {
                msg!("Job ID mismatch: expected {}, got {}", pending_job_id, fhe_job_id);
                return Err(FutarchyError::InvalidInstruction.into());
            }
        }
        None => {
            msg!("No pending pool update job");
            return Err(FutarchyError::InvalidInstruction.into());
        }
    }

    // TODO: Verify FHE job has reached consensus
    // This would involve checking the FheConsensusData account
    // For MVP, we trust the caller

    // Update encrypted pool
    let result_size = encrypted_result.len();
    if side {
        market.encrypted_pool_yes = encrypted_result;
        msg!("Updated encrypted YES pool");
    } else {
        market.encrypted_pool_no = encrypted_result;
        msg!("Updated encrypted NO pool");
    }

    // Clear pending job
    market.pending_pool_update_job = None;

    // Write updated market
    drop(market_data);
    let mut market_data = market_info.try_borrow_mut_data()?;
    market.serialize(&mut &mut market_data[..])?;

    msg!("Pool updated successfully");
    msg!("  Market ID: {}", market_id);
    msg!("  FHE Job ID: {}", fhe_job_id);
    msg!("  Side: {}", if side { "YES" } else { "NO" });
    msg!("  Encrypted result size: {} bytes", result_size);

    Ok(())
}
