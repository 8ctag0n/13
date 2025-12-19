//! PlaceBet instruction processor

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::{
    cpi::verify_market_bet_proof,
    error::FutarchyError,
    state::{Market, Position, UserEligibility, UserEscrow, USER_ELIGIBILITY_SEED, USER_ESCROW_SEED},
};

/// Process PlaceBet instruction
///
/// Places a bet on a market with ZK proof verification
///
/// Accounts expected (basic):
/// 0. `[writable, signer]` User placing bet
/// 1. `[writable]` Market account (PDA)
/// 2. `[writable]` Position account (PDA)
/// 3. `[writable]` UserEscrow account (PDA) - NEW: User's deposit balance
/// 4. `[writable]` Market Escrow account (PDA) - Market's pool
/// 5. `[]` ZK-generator program
/// 6. `[]` System program
/// 7. `[]` Clock sysvar
///
/// Additional accounts (if circuit_type == 31, MarketBetWithPoI):
/// 8. `[]` UserEligibility account (PDA)
///
/// Additional accounts (if encrypted_bet_amount is Some):
/// 8/9. `[writable]` FHE job account (PDA)
/// 9/10. `[writable]` FHE consensus account (PDA)
/// 10/11. `[writable]` FHE escrow account (PDA)
/// 11/12. `[]` FHE-generator program
#[allow(clippy::too_many_arguments)]
pub fn process_place_bet(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    market_id: u64,
    bet_commitment: [u8; 32],
    proof: Vec<u8>,
    public_inputs: Vec<u8>,
    amount: u64,
    circuit_type: u8,
    encrypted_bet_amount: Option<Vec<u8>>,
    side: Option<bool>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let user_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let position_info = next_account_info(account_info_iter)?;
    let user_escrow_info = next_account_info(account_info_iter)?;
    let market_escrow_info = next_account_info(account_info_iter)?;
    let zk_program_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;
    let clock_info = next_account_info(account_info_iter)?;

    // Verify user is signer
    if !user_info.is_signer {
        msg!("User must be signer");
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

    // Verify market is active
    if !market.is_active() {
        msg!("Market is not active");
        return Err(FutarchyError::MarketNotActive.into());
    }

    // Verify betting period hasn't ended
    if market.is_betting_closed(current_time) {
        msg!("Betting period has ended");
        return Err(FutarchyError::BettingClosed.into());
    }

    // Verify bet amount doesn't exceed max
    if amount > market.max_bet {
        msg!("Bet amount exceeds maximum: {} > {}", amount, market.max_bet);
        return Err(FutarchyError::BetExceedsMax.into());
    }

    // Verify ZK proof
    verify_market_bet_proof(zk_program_info, &proof, &public_inputs, circuit_type)?;

    msg!("ZK proof verified successfully");

    // Verify and load user escrow
    let (user_escrow_pda, _) = Pubkey::find_program_address(
        &[USER_ESCROW_SEED, user_info.key.as_ref(), market_info.key.as_ref()],
        program_id,
    );

    if user_escrow_pda != *user_escrow_info.key {
        msg!("Invalid user escrow PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    let mut user_escrow_data = user_escrow_info.try_borrow_mut_data()?;
    let mut user_escrow = UserEscrow::deserialize(&mut &user_escrow_data[..])?;

    // Verify user has sufficient available balance in escrow
    if amount > user_escrow.available {
        msg!("Insufficient escrow balance");
        msg!("  Required: {} lamports", amount);
        msg!("  Available: {} lamports", user_escrow.available);
        msg!("  Reserved: {} lamports", user_escrow.reserved);
        return Err(ProgramError::InsufficientFunds);
    }

    msg!("User escrow verified");
    msg!("  Deposited: {} lamports", user_escrow.deposited);
    msg!("  Available: {} lamports", user_escrow.available);
    msg!("  Reserving: {} lamports", amount);

    // If using Circuit 31 (MarketBetWithPoI), verify user eligibility
    if circuit_type == 31 {
        msg!("Verifying user eligibility (Circuit 31 - MarketBetWithPoI)");

        // Get UserEligibility account
        let user_eligibility_info = next_account_info(account_info_iter)?;

        // Derive expected UserEligibility PDA
        let (eligibility_pda, _) = Pubkey::find_program_address(
            &[USER_ELIGIBILITY_SEED, user_info.key.as_ref()],
            program_id,
        );

        if eligibility_pda != *user_eligibility_info.key {
            msg!("Invalid UserEligibility PDA");
            return Err(ProgramError::InvalidSeeds);
        }

        // Load and verify UserEligibility
        let eligibility_data = user_eligibility_info.try_borrow_data()?;
        let user_eligibility = UserEligibility::deserialize(&mut &eligibility_data[..])?;

        // Check if eligibility is valid
        if !user_eligibility.is_valid(current_time) {
            msg!("User eligibility expired or inactive");
            msg!("  Current time: {}", current_time);
            msg!("  Expires at: {}", user_eligibility.expires_at);
            msg!("  Is active: {}", user_eligibility.is_active);
            return Err(FutarchyError::UserNotEligible.into());
        }

        msg!("User eligibility verified");
        msg!("  Registered at: {}", user_eligibility.registered_at);
        msg!("  Expires at: {}", user_eligibility.expires_at);
        msg!("  Blacklist version: {}", user_eligibility.blacklist_version);
    }

    // Derive position PDA
    let position_seeds = &[
        crate::state::POSITION_SEED,
        user_info.key.as_ref(),
        &market_id.to_le_bytes(),
        &bet_commitment,
    ];
    let (position_pda, position_bump) = Pubkey::find_program_address(position_seeds, program_id);

    if position_pda != *position_info.key {
        msg!("Invalid position PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Create position account (PDA requires invoke_signed)
    let rent = Rent::get()?;
    let position_space = if let Some(ref encrypted_bet) = encrypted_bet_amount {
        Position::space(encrypted_bet.len())
    } else {
        Position::SPACE
    };
    let position_lamports = rent.minimum_balance(position_space);

    msg!("Creating position account");

    let position_signer_seeds: &[&[u8]] = &[
        crate::state::POSITION_SEED,
        user_info.key.as_ref(),
        &market_id.to_le_bytes(),
        &bet_commitment,
        &[position_bump],
    ];

    solana_program::program::invoke_signed(
        &system_instruction::create_account(
            user_info.key,
            position_info.key,
            position_lamports,
            position_space as u64,
            program_id,
        ),
        &[
            user_info.clone(),
            position_info.clone(),
            system_program_info.clone(),
        ],
        &[position_signer_seeds],
    )?;

    // Initialize position state
    let position = Position {
        user: *user_info.key,
        market: *market_info.key,
        bet_commitment,
        placed_at: current_time,
        claimed: false,
        bump: position_bump,
        encrypted_amount: encrypted_bet_amount.clone(),
    };

    // Serialize position
    let mut position_data = position_info.try_borrow_mut_data()?;
    position.serialize(&mut &mut position_data[..])?;

    // Reserve funds in user escrow
    user_escrow.reserve(amount)?;

    // Transfer bet amount from user escrow to market escrow
    msg!("Transferring {} lamports from user escrow to market escrow", amount);

    **user_escrow_info.try_borrow_mut_lamports()? -= amount;
    **market_escrow_info.try_borrow_mut_lamports()? += amount;

    // Update escrow state: funds left the escrow
    user_escrow.transfer_to_market(amount)?;

    // Write updated user escrow
    drop(user_escrow_data);
    let mut user_escrow_data = user_escrow_info.try_borrow_mut_data()?;
    user_escrow.serialize(&mut &mut user_escrow_data[..])?;

    msg!("Funds reserved and transferred");
    msg!("  User escrow deposited: {} lamports", user_escrow.deposited);
    msg!("  User escrow available: {} lamports", user_escrow.available);
    msg!("  User escrow reserved: {} lamports", user_escrow.reserved);

    // Handle FHE pool update if encrypted bet provided
    let fhe_enabled = encrypted_bet_amount.is_some();
    if let (Some(encrypted_bet), Some(bet_side)) = (encrypted_bet_amount, side) {
        msg!("Creating FHE job for encrypted pool update");

        // Get FHE accounts
        let fhe_job_info = next_account_info(account_info_iter)?;
        let fhe_consensus_info = next_account_info(account_info_iter)?;
        let fhe_escrow_info = next_account_info(account_info_iter)?;
        let fhe_program_info = next_account_info(account_info_iter)?;

        // Generate unique job ID (using current slot + market_id)
        use solana_program::sysvar::Sysvar;
        use solana_program::clock::Clock;
        let clock = Clock::get()?;
        let fhe_job_id = clock.slot + market_id;

        // Get current encrypted pool for the chosen side
        let current_pool = if bet_side {
            market.encrypted_pool_yes.clone()
        } else {
            market.encrypted_pool_no.clone()
        };

        // Create FHE job to add bet to pool
        crate::cpi::create_fhe_pool_addition_job(
            fhe_program_info,
            market_info,  // Market program signs as creator
            fhe_job_info,
            fhe_consensus_info,
            fhe_escrow_info,
            system_program_info,
            fhe_job_id,
            current_pool,
            encrypted_bet,
            100_000,  // 0.0001 SOL payment to provers
        )?;

        // Mark job as pending
        market.pending_pool_update_job = Some(fhe_job_id);

        msg!("FHE job created: {}", fhe_job_id);
        msg!("  Side: {}", if bet_side { "YES" } else { "NO" });
        msg!("  Provers will compute new encrypted pool");
    }

    // Update transparent totals (for backward compatibility)
    // In FHE mode, these are just approximate/placeholders
    market.total_yes_bets = market.total_yes_bets.checked_add(amount)
        .ok_or(FutarchyError::Overflow)?;

    // Write updated market
    drop(market_data);
    let mut market_data = market_info.try_borrow_mut_data()?;
    market.serialize(&mut &mut market_data[..])?;

    msg!("Bet placed successfully");
    msg!("  Market ID: {}", market_id);
    msg!("  User: {}", user_info.key);
    msg!("  Amount: {}", amount);
    msg!("  Commitment: {:?}", &bet_commitment[..8]);
    if fhe_enabled {
        msg!("  Mode: FHE ENCRYPTED");
    } else {
        msg!("  Mode: Transparent");
    }

    Ok(())
}
