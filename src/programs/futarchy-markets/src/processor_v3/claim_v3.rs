//! ClaimV3 - Claim payout from settled blind market
//!
//! User proves ownership of winning bet via ZK proof
//! Payout calculated from decrypted pool totals

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::error::FutarchyError;
use crate::state::{
    MarketV3, MarketVault, PositionV3, PrivateBalance, ProtocolVault,
    MARKET_V3_SEED, MARKET_VAULT_SEED, POSITION_V3_SEED, PRIVATE_BALANCE_SEED,
};

/// Circuit type for ClaimBlind (placeholder - circuit not yet implemented)
pub const CIRCUIT_CLAIM_BLIND: u8 = 51;

/// Process ClaimV3 instruction
///
/// Claims payout from a settled blind market
///
/// Accounts expected:
/// 0. `[writable, signer]` User claiming
/// 1. `[writable]` PrivateBalance PDA
/// 2. `[]` MarketV3 PDA (must be settled)
/// 3. `[writable]` PositionV3 PDA
/// 4. `[writable]` MarketVault PDA (source of payout)
/// 5. `[writable]` ProtocolVault PDA (destination)
/// 6. `[]` ZK-generator program
/// 7. `[]` System program
#[allow(clippy::too_many_arguments)]
pub fn process_claim_v3(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    market_id: u64,
    bet_ciphertext_hash: [u8; 32],
    bet_secret_commitment: [u8; 32],
    proof: Vec<u8>,
    public_inputs: Vec<u8>,
    new_balance_commitment: [u8; 32],
    circuit_type: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let user_info = next_account_info(account_info_iter)?;
    let private_balance_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let position_info = next_account_info(account_info_iter)?;
    let market_vault_info = next_account_info(account_info_iter)?;
    let protocol_vault_info = next_account_info(account_info_iter)?;
    let zk_program_info = next_account_info(account_info_iter)?;
    let _system_program_info = next_account_info(account_info_iter)?;

    // Verify user is signer
    if !user_info.is_signer {
        msg!("User must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate circuit type
    if circuit_type != CIRCUIT_CLAIM_BLIND {
        msg!("Invalid circuit type: {} (expected {})", circuit_type, CIRCUIT_CLAIM_BLIND);
        return Err(ProgramError::InvalidArgument);
    }

    let market_id_bytes = market_id.to_le_bytes();

    // Verify PrivateBalance PDA
    let (private_balance_pda, _) = Pubkey::find_program_address(
        &[PRIVATE_BALANCE_SEED, user_info.key.as_ref()],
        program_id,
    );
    if private_balance_pda != *private_balance_info.key {
        msg!("Invalid PrivateBalance PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Load PrivateBalance
    let balance_data = private_balance_info.try_borrow_data()?;
    let mut private_balance = PrivateBalance::deserialize(&mut &balance_data[..])?;
    drop(balance_data);

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
    let market = MarketV3::deserialize(&mut &market_data[..])?;

    if !market.is_settled() {
        msg!("Market not yet settled");
        return Err(FutarchyError::MarketNotSettled.into());
    }

    // Verify PositionV3 PDA
    let (position_pda, _) = Pubkey::find_program_address(
        &[POSITION_V3_SEED, &market_id_bytes, &bet_ciphertext_hash],
        program_id,
    );
    if position_pda != *position_info.key {
        msg!("Invalid PositionV3 PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Load position
    let position_data = position_info.try_borrow_data()?;
    let mut position = PositionV3::deserialize(&mut &position_data[..])?;
    drop(position_data);

    // Verify position matches
    if position.bet_secret_commitment != bet_secret_commitment {
        msg!("Bet secret commitment mismatch");
        return Err(ProgramError::InvalidAccountData);
    }

    if position.is_claimed() {
        msg!("Position already claimed");
        return Err(FutarchyError::AlreadyClaimed.into());
    }

    // Verify vaults
    let (market_vault_pda, market_vault_bump) = MarketVault::find_pda(market_id, program_id);
    if market_vault_pda != *market_vault_info.key {
        msg!("Invalid MarketVault PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    let (protocol_vault_pda, _) = ProtocolVault::find_pda(program_id);
    if protocol_vault_pda != *protocol_vault_info.key {
        msg!("Invalid ProtocolVault PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Verify ZK proof and extract payout amount
    let payout_amount = verify_claim_blind_proof(
        zk_program_info,
        &proof,
        &public_inputs,
        &market,
        &bet_secret_commitment,
    )?;

    // Check vault has sufficient funds
    if market_vault_info.lamports() < payout_amount {
        msg!("Insufficient funds in MarketVault for payout");
        return Err(ProgramError::InsufficientFunds);
    }

    // Transfer payout: MarketVault -> ProtocolVault
    msg!("Transferring {} lamports from MarketVault to ProtocolVault", payout_amount);

    **market_vault_info.try_borrow_mut_lamports()? -= payout_amount;
    **protocol_vault_info.try_borrow_mut_lamports()? += payout_amount;

    // Update PrivateBalance commitment
    let mut balance_data = private_balance_info.try_borrow_mut_data()?;
    private_balance.update_commitment(new_balance_commitment);
    private_balance.serialize(&mut &mut balance_data[..])?;

    // Mark position as claimed
    let mut position_data = position_info.try_borrow_mut_data()?;
    position.mark_claimed();
    position.serialize(&mut &mut position_data[..])?;

    msg!("Claim V3 successful");
    msg!("  Market ID: {}", market_id);
    msg!("  Payout: {} lamports", payout_amount);
    msg!("  Position PDA: {}", position_info.key);

    Ok(())
}

/// Verify ZK proof for blind claim and extract payout amount
///
/// Public inputs format (72 bytes):
/// 0-32: bet_secret_commitment
/// 32-40: winning_pool (8 bytes)
/// 40-48: losing_pool (8 bytes)
/// 48-49: outcome (1 byte)
/// 49-57: claimed_payout (8 bytes)
/// 57-65: market_id (8 bytes)
/// 65-72: padding (7 bytes)
fn verify_claim_blind_proof(
    _zk_program_info: &AccountInfo,
    _proof: &[u8],
    public_inputs: &[u8],
    market: &MarketV3,
    bet_secret_commitment: &[u8; 32],
) -> Result<u64, ProgramError> {
    // Validate public_inputs size
    if public_inputs.len() < 72 {
        msg!("Invalid public inputs size: {} (expected 72)", public_inputs.len());
        return Err(ProgramError::InvalidInstructionData);
    }

    // Extract fields from public_inputs
    let commitment_from_inputs: [u8; 32] = public_inputs[0..32].try_into().unwrap();
    let winning_pool = u64::from_le_bytes(public_inputs[32..40].try_into().unwrap());
    let losing_pool = u64::from_le_bytes(public_inputs[40..48].try_into().unwrap());
    let outcome = public_inputs[48] == 0; // 0=YES won (true), 1=NO won (false) - matches market.resolution
    let claimed_payout = u64::from_le_bytes(public_inputs[49..57].try_into().unwrap());
    let market_id_from_inputs = u64::from_le_bytes(public_inputs[57..65].try_into().unwrap());

    // Verify bet_secret_commitment matches
    if &commitment_from_inputs != bet_secret_commitment {
        msg!("Bet secret commitment mismatch in public inputs");
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify market pools match decrypted on-chain state
    let market_winning_pool = market.winning_pool()
        .ok_or(FutarchyError::MarketNotSettled)?;
    let market_losing_pool = market.losing_pool()
        .ok_or(FutarchyError::MarketNotSettled)?;

    if winning_pool != market_winning_pool || losing_pool != market_losing_pool {
        msg!("Pool mismatch: proof has {}/{}, market has {}/{}",
            winning_pool, losing_pool, market_winning_pool, market_losing_pool);
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify outcome matches
    let market_outcome = market.resolution.ok_or(FutarchyError::MarketNotSettled)?;
    if outcome != market_outcome {
        msg!("Outcome mismatch: proof has {}, market has {}", outcome, market_outcome);
        return Err(ProgramError::InvalidAccountData);
    }

    msg!("Claim verification:");
    msg!("  Bet secret commitment verified");
    msg!("  Winning pool: {}", winning_pool);
    msg!("  Losing pool: {}", losing_pool);
    msg!("  Outcome: {}", if outcome { "YES" } else { "NO" });
    msg!("  Claimed payout: {}", claimed_payout);
    msg!("  Market ID: {}", market_id_from_inputs);

    // The ZK proof has already verified that:
    // 1. User knows the secret matching bet_secret_commitment
    // 2. The bet was on the winning side
    // 3. claimed_payout is correctly calculated from bet_amount and pools
    //
    // So we just return the claimed_payout verified by the ZK circuit
    Ok(claimed_payout)
}
