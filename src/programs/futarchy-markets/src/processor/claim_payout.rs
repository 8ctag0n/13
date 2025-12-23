//! ClaimPayout instruction processor

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::{
    cpi::verify_market_claim_proof,
    error::FutarchyError,
    state::{Market, Nullifier, Position, NULLIFIER_SEED, POSITION_SEED},
};

/// Process ClaimPayout instruction
///
/// Claims winnings from a settled market using ZK proof.
/// Uses NullifierAccount PDA for unlimited scalability (vs 100-limit vec).
///
/// Accounts expected:
/// 0. `[writable, signer]` User claiming payout
/// 1. `[]` Market account (PDA)
/// 2. `[writable]` Nullifier account (PDA) - will be created
/// 3. `[writable]` Position account (PDA) - will be verified and closed
/// 4. `[writable]` Escrow account (PDA)
/// 5. `[]` ZK-generator program
/// 6. `[]` System program
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
    let nullifier_info = next_account_info(account_info_iter)?;
    let position_info = next_account_info(account_info_iter)?;
    let escrow_info = next_account_info(account_info_iter)?;
    let zk_program_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    // Verify user is signer
    if !user_info.is_signer {
        msg!("User must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load market (read-only now)
    let market_data = market_info.try_borrow_data()?;
    let market = Market::deserialize(&mut &market_data[..])?;
    drop(market_data);

    // Verify market PDA
    let (market_pda, _market_bump) = crate::cpi::derive_market_pda(program_id, market_id);
    if market_pda != *market_info.key {
        msg!("Invalid market PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Verify market is settled
    if !market.is_settled() {
        msg!("Market not settled yet");
        return Err(FutarchyError::MarketNotSettled.into());
    }

    // Verify nullifier PDA
    let (nullifier_pda, nullifier_bump) = crate::cpi::derive_nullifier_pda(
        program_id,
        market_id,
        &claim_nullifier,
    );
    if nullifier_pda != *nullifier_info.key {
        msg!("Invalid nullifier PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Check nullifier hasn't been used (account should not exist)
    if nullifier_info.data_len() > 0 {
        msg!("Claim nullifier already used");
        return Err(FutarchyError::NullifierUsed.into());
    }

    // Verify escrow PDA
    let (escrow_pda, _escrow_bump) = crate::cpi::derive_escrow_pda(program_id, market_id);
    if escrow_pda != *escrow_info.key {
        msg!("Invalid escrow PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Verify ZK proof
    verify_market_claim_proof(zk_program_info, &proof, &public_inputs)?;
    msg!("ZK claim proof verified successfully");

    // Extract bet_commitment from public_inputs[65..97] (32 bytes)
    if public_inputs.len() < 97 {
        msg!("Invalid public_inputs length: {} < 97", public_inputs.len());
        return Err(ProgramError::InvalidInstructionData);
    }
    let bet_commitment: [u8; 32] = public_inputs[65..97]
        .try_into()
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    msg!("Extracted bet_commitment: {:?}", &bet_commitment[..8]);

    // Verify Position PDA
    let (position_pda, _position_bump) = crate::cpi::derive_position_pda(
        program_id,
        user_info.key,
        market_id,
        &bet_commitment,
    );
    if position_pda != *position_info.key {
        msg!("Invalid position PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Load and verify Position account
    let position_data = position_info.try_borrow_data()?;
    let position = Position::deserialize(&mut &position_data[..])?;
    drop(position_data);

    // Verify position belongs to user
    if position.user != *user_info.key {
        msg!("Position does not belong to user");
        return Err(FutarchyError::InvalidOwner.into());
    }

    // Verify bet_commitment matches
    if position.bet_commitment != bet_commitment {
        msg!("Position bet_commitment mismatch");
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify position has not been claimed
    if position.claimed {
        msg!("Position already claimed");
        return Err(FutarchyError::PositionAlreadyClaimed.into());
    }

    msg!("Position verified successfully");

    // Verify escrow has sufficient balance
    let escrow_balance = escrow_info.lamports();
    if escrow_balance < payout_amount {
        msg!("Insufficient escrow balance: {} < {}", escrow_balance, payout_amount);
        return Err(FutarchyError::InsufficientEscrow.into());
    }

    // Get current timestamp
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    // Create nullifier account (marks nullifier as used)
    let rent = Rent::get()?;
    let nullifier_rent = rent.minimum_balance(Nullifier::SPACE);

    let nullifier_seeds: &[&[u8]] = &[
        NULLIFIER_SEED,
        &market_id.to_le_bytes(),
        &claim_nullifier,
        &[nullifier_bump],
    ];

    invoke_signed(
        &system_instruction::create_account(
            user_info.key,
            nullifier_info.key,
            nullifier_rent,
            Nullifier::SPACE as u64,
            program_id,
        ),
        &[
            user_info.clone(),
            nullifier_info.clone(),
            system_program_info.clone(),
        ],
        &[nullifier_seeds],
    )?;

    // Initialize nullifier account
    let nullifier = Nullifier::new(
        market_id,
        claim_nullifier,
        *user_info.key,
        current_time,
        payout_amount,
        nullifier_bump,
    );

    let mut nullifier_data = nullifier_info.try_borrow_mut_data()?;
    nullifier.serialize(&mut &mut nullifier_data[..])?;
    drop(nullifier_data);

    msg!("Nullifier account created");

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

    // Close Position account - transfer all lamports to user
    let position_lamports = position_info.lamports();
    msg!("Closing position account, reclaiming {} lamports", position_lamports);

    **position_info.try_borrow_mut_lamports()? = 0;
    **user_info.try_borrow_mut_lamports()? = user_info
        .lamports()
        .checked_add(position_lamports)
        .ok_or(FutarchyError::Overflow)?;

    // Zero out position data
    let mut position_data = position_info.try_borrow_mut_data()?;
    position_data.fill(0);
    drop(position_data);

    msg!("Position account closed successfully");

    msg!("Payout claimed successfully");
    msg!("  Market ID: {}", market_id);
    msg!("  User: {}", user_info.key);
    msg!("  Payout Amount: {}", payout_amount);
    msg!("  Rent Reclaimed: {}", position_lamports);
    msg!("  Nullifier: {:?}", &claim_nullifier[..8]);
    msg!("  Bet Commitment: {:?}", &bet_commitment[..8]);

    Ok(())
}
