//! UpdatePoolState - Update market pool state root after FHE consensus
//!
//! Called by FHE provers (or anyone with valid consensus proof) to update
//! the pool_state_root commitment after processing bets/claims.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::state::{MarketV2, MARKET_V2_SEED};

/// Process UpdatePoolState instruction
///
/// Updates the pool state root commitment after FHE prover consensus
///
/// Accounts expected:
/// 0. `[writable, signer]` Updater (anyone with valid consensus proof)
/// 1. `[writable]` MarketV2 PDA
pub fn process_update_pool_state(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    market_id: u64,
    new_pool_state_root: [u8; 32],
    consensus_proof: Vec<u8>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let updater_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;

    // Verify updater is signer
    if !updater_info.is_signer {
        msg!("Updater must be signer");
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

    // Verify consensus proof
    // TODO: Implement actual FHE consensus verification
    // For now, we just verify the proof is not empty
    if consensus_proof.is_empty() {
        msg!("Consensus proof is required");
        return Err(ProgramError::InvalidArgument);
    }

    // TODO: Verify consensus proof includes:
    // - Signatures from 3-of-5 threshold provers
    // - Each prover computed same result
    // - Result matches new_pool_state_root

    // Load market
    let market_data = market_info.try_borrow_data()?;
    let mut market = MarketV2::deserialize(&mut &market_data[..])?;
    drop(market_data);

    let old_root = market.pool_state_root;

    // Update pool state root
    market.update_pool_state_root(new_pool_state_root);

    // Serialize updated market
    let mut market_data = market_info.try_borrow_mut_data()?;
    market.serialize(&mut &mut market_data[..])?;

    msg!("Pool state updated");
    msg!("  Market ID: {}", market_id);
    msg!("  Old root: {:?}", &old_root[..8]);
    msg!("  New root: {:?}", &new_pool_state_root[..8]);

    Ok(())
}
