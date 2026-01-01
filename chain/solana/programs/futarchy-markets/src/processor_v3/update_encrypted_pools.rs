//! UpdateEncryptedPools - Update encrypted pool hashes after FHE operation
//!
//! This is called by off-chain provers after homomorphically adding
//! a bet ciphertext to the encrypted pools.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::state::{MarketV3, MARKET_V3_SEED};

/// Process UpdateEncryptedPools instruction
///
/// Updates the hashes of encrypted YES and NO pools
///
/// Accounts expected:
/// 0. `[writable, signer]` Updater (prover/aggregator)
/// 1. `[writable]` MarketV3 PDA
pub fn process_update_encrypted_pools(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    market_id: u64,
    encrypted_pool_yes_hash: [u8; 32],
    encrypted_pool_no_hash: [u8; 32],
    _fhe_proof: Vec<u8>,
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

    // Update encrypted pool hashes
    market.update_encrypted_pool_hashes(encrypted_pool_yes_hash, encrypted_pool_no_hash);

    // Save updated market
    let mut market_data = market_info.try_borrow_mut_data()?;
    market.serialize(&mut &mut market_data[..])?;

    msg!("Encrypted pools updated");
    msg!("  Market ID: {}", market_id);
    msg!("  YES pool hash: {:?}", &encrypted_pool_yes_hash[..8]);
    msg!("  NO pool hash: {:?}", &encrypted_pool_no_hash[..8]);

    // TODO: Verify fhe_proof in production
    // For now, we trust the updater (centralized prover)
    // Future: verify consensus proof from multiple provers

    Ok(())
}
