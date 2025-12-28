//! ClaimPayout instruction processor

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    cpi::verify_market_claim_proof,
    error::FutarchyError,
    state::Market,
};

/// Process ClaimPayout instruction
///
/// Claims winnings from a settled market using ZK proof
///
/// Accounts expected:
/// 0. `[writable, signer]` User claiming payout
/// 1. `[writable]` Market account (PDA)
/// 2. `[writable]` Position account (PDA, optional - for tracking)
/// 3. `[writable]` Escrow account (PDA)
/// 4. `[]` ZK-generator program
/// 5. `[]` System program
#[allow(clippy::too_many_arguments)]
pub fn process_claim_payout(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    market_id: u64,
    claim_nullifier: [u8; 32],
    proof: Vec<u8>,
    public_inputs: Vec<u8>,
    payout_amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let user_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let _position_info = next_account_info(account_info_iter).ok(); // Optional
    let escrow_info = next_account_info(account_info_iter)?;
    let zk_program_info = next_account_info(account_info_iter)?;
    let _system_program_info = next_account_info(account_info_iter)?;

    // Verify user is signer
    if !user_info.is_signer {
        msg!("User must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load market
    let market_data = market_info.try_borrow_mut_data()?;
    let mut market = Market::deserialize(&mut &market_data[..])?;

    // Verify market PDA
    let (market_pda, market_bump) = crate::cpi::derive_market_pda(program_id, market_id);
    if market_pda != *market_info.key {
        msg!("Invalid market PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Verify market is settled
    if !market.is_settled() {
        msg!("Market not settled yet");
        return Err(FutarchyError::MarketNotSettled.into());
    }

    // Verify escrow PDA
    let (escrow_pda, escrow_bump) = crate::cpi::derive_escrow_pda(program_id, market_id);
    if escrow_pda != *escrow_info.key {
        msg!("Invalid escrow PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Check nullifier hasn't been used
    if market.claim_nullifiers.contains(&claim_nullifier) {
        msg!("Claim nullifier already used");
        return Err(FutarchyError::NullifierUsed.into());
    }

    // Verify ZK proof
    verify_market_claim_proof(zk_program_info, &proof, &public_inputs)?;

    msg!("ZK claim proof verified successfully");

    // Verify escrow has sufficient balance
    let escrow_balance = escrow_info.lamports();
    if escrow_balance < payout_amount {
        msg!("Insufficient escrow balance: {} < {}", escrow_balance, payout_amount);
        return Err(FutarchyError::InsufficientEscrow.into());
    }

    // Add nullifier to prevent double claim
    market.add_nullifier(claim_nullifier)
        .map_err(|_| FutarchyError::NullifierUsed)?;

    // Write updated market
    drop(market_data);
    let mut market_data = market_info.try_borrow_mut_data()?;
    market.serialize(&mut &mut market_data[..])?;

    // Transfer payout from escrow to user
    msg!("Transferring {} lamports from escrow to user", payout_amount);

    **escrow_info.try_borrow_mut_lamports()? = escrow_info
        .lamports()
        .checked_sub(payout_amount)
        .ok_or(FutarchyError::InsufficientEscrow)?;

    **user_info.try_borrow_mut_lamports()? = user_info
        .lamports()
        .checked_add(payout_amount)
        .ok_or(FutarchyError::Overflow)?;

    msg!("Payout claimed successfully");
    msg!("  Market ID: {}", market_id);
    msg!("  User: {}", user_info.key);
    msg!("  Amount: {}", payout_amount);
    msg!("  Nullifier: {:?}", &claim_nullifier[..8]);

    Ok(())
}
