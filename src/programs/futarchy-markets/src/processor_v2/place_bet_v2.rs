//! PlaceBetV2 - Place an anonymous private bet on a market
//!
//! Flow:
//! 1. Verify ZK proof (circuit 33: PlaceBetPrivate)
//! 2. Create anonymous PositionV2 PDA (no user field)
//! 3. Update PrivateBalance commitment
//! 4. Transfer lamports: ProtocolVault -> MarketVault
//! 5. Emit event for FHE provers

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

use crate::error::FutarchyError;
use crate::state::{
    MarketV2, MarketVault, PositionV2, PrivateBalance, ProtocolVault,
    MARKET_V2_SEED, MARKET_VAULT_SEED, POSITION_V2_SEED, PRIVATE_BALANCE_SEED, PROTOCOL_VAULT_SEED,
};

/// Circuit type for PlaceBetPrivate (33)
pub const CIRCUIT_PLACE_BET_PRIVATE: u8 = 33;

/// Process PlaceBetV2 instruction
///
/// Places an anonymous private bet on a market
///
/// Accounts expected:
/// 0. `[writable, signer]` User placing bet
/// 1. `[writable]` PrivateBalance PDA
/// 2. `[writable]` MarketV2 PDA
/// 3. `[writable]` PositionV2 PDA (will be created)
/// 4. `[writable]` ProtocolVault PDA (source of funds)
/// 5. `[writable]` MarketVault PDA (destination)
/// 6. `[]` ZK-generator program
/// 7. `[]` System program
/// 8. `[]` Clock sysvar
#[allow(clippy::too_many_arguments)]
pub fn process_place_bet_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    market_id: u64,
    bet_commitment: [u8; 32],
    bet_side: bool,
    proof: Vec<u8>,
    public_inputs: Vec<u8>,
    new_balance_commitment: [u8; 32],
    encrypted_bet: Vec<u8>,
    circuit_type: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let user_info = next_account_info(account_info_iter)?;
    let private_balance_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let position_info = next_account_info(account_info_iter)?;
    let protocol_vault_info = next_account_info(account_info_iter)?;
    let market_vault_info = next_account_info(account_info_iter)?;
    let zk_program_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;
    let clock_sysvar_info = next_account_info(account_info_iter)?;

    // Verify user is signer
    if !user_info.is_signer {
        msg!("User must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate circuit type
    if circuit_type != CIRCUIT_PLACE_BET_PRIVATE {
        msg!("Invalid circuit type: {} (expected {})", circuit_type, CIRCUIT_PLACE_BET_PRIVATE);
        return Err(ProgramError::InvalidArgument);
    }

    let clock = Clock::from_account_info(clock_sysvar_info)?;
    let current_time = clock.unix_timestamp;
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
    if private_balance.user != *user_info.key {
        msg!("PrivateBalance user mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    drop(balance_data);

    // Verify MarketV2 PDA
    let (market_pda, _) = Pubkey::find_program_address(
        &[MARKET_V2_SEED, &market_id_bytes],
        program_id,
    );
    if market_pda != *market_info.key {
        msg!("Invalid MarketV2 PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Load and validate market
    let market_data = market_info.try_borrow_data()?;
    let mut market = MarketV2::deserialize(&mut &market_data[..])?;
    drop(market_data);

    if !market.is_active() {
        msg!("Market is not active");
        return Err(FutarchyError::MarketNotActive.into());
    }

    if market.is_betting_closed(current_time) {
        msg!("Betting period has ended");
        return Err(FutarchyError::BettingClosed.into());
    }

    // Increment bet counter for the chosen side
    market.increment_bet_count(bet_side);

    // Verify PositionV2 PDA
    let (position_pda, position_bump) = Pubkey::find_program_address(
        &[POSITION_V2_SEED, &market_id_bytes, &bet_commitment],
        program_id,
    );
    if position_pda != *position_info.key {
        msg!("Invalid PositionV2 PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Check position doesn't already exist (same bet_commitment)
    if position_info.data_len() > 0 {
        msg!("Position with this bet_commitment already exists");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    // Verify ProtocolVault PDA
    let (protocol_vault_pda, protocol_vault_bump) = ProtocolVault::find_pda(program_id);
    if protocol_vault_pda != *protocol_vault_info.key {
        msg!("Invalid ProtocolVault PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Verify MarketVault PDA
    let (market_vault_pda, _) = MarketVault::find_pda(market_id, program_id);
    if market_vault_pda != *market_vault_info.key {
        msg!("Invalid MarketVault PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Verify ZK proof (circuit 33: PlaceBetPrivate)
    // Extract bet amount from public inputs for the transfer
    let bet_amount = verify_place_bet_proof(
        zk_program_info,
        &proof,
        &public_inputs,
        &private_balance.balance_commitment,
        &new_balance_commitment,
        &bet_commitment,
        market_id,
        market.max_bet,
    )?;

    // Check vault has sufficient funds
    if protocol_vault_info.lamports() < bet_amount {
        msg!("Insufficient funds in ProtocolVault");
        return Err(ProgramError::InsufficientFunds);
    }

    // Create PositionV2 PDA (anonymous - no user field!)
    let rent = Rent::get()?;
    let position_space = PositionV2::SPACE;
    let position_lamports = rent.minimum_balance(position_space);
    let position_seeds = &[
        POSITION_V2_SEED,
        &market_id_bytes,
        &bet_commitment,
        &[position_bump],
    ];

    msg!("Creating anonymous PositionV2 PDA");
    invoke_signed(
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
        &[position_seeds],
    )?;

    // Initialize PositionV2 (only commitment stored, amount/side private)
    let position = PositionV2::new(
        market_id,
        bet_commitment,
        current_time,
        position_bump,
    );
    let mut position_data = position_info.try_borrow_mut_data()?;
    position.serialize(&mut &mut position_data[..])?;
    drop(position_data);

    // Transfer lamports: ProtocolVault -> MarketVault
    msg!("Transferring {} lamports from ProtocolVault to MarketVault", bet_amount);

    **protocol_vault_info.try_borrow_mut_lamports()? -= bet_amount;
    **market_vault_info.try_borrow_mut_lamports()? += bet_amount;

    // Update PrivateBalance commitment
    let mut balance_data = private_balance_info.try_borrow_mut_data()?;
    private_balance.update_commitment(new_balance_commitment);
    private_balance.serialize(&mut &mut balance_data[..])?;

    // Save updated market with incremented bet counter
    let mut market_data = market_info.try_borrow_mut_data()?;
    market.serialize(&mut &mut market_data[..])?;

    msg!("Bet placed successfully");
    msg!("  Market ID: {}", market_id);
    msg!("  Bet side: {}", if bet_side { "YES" } else { "NO" });
    msg!("  Bet commitment: {:?}", &bet_commitment[..8]);
    msg!("  Position PDA: {}", position_info.key);
    msg!("  Amount transferred: {} lamports", bet_amount);
    msg!("  Bet counts: YES={}, NO={}", market.bet_count_yes, market.bet_count_no);

    // Emit event for FHE provers to update pools
    // encrypted_bet contains: FHE(amount) || side_encrypted
    msg!(
        "EVENT:PLACE_BET:{}:{:?}:{:?}",
        market_id,
        &bet_commitment[..8],
        &encrypted_bet[..std::cmp::min(8, encrypted_bet.len())]
    );

    Ok(())
}

/// Verify ZK proof for place bet and extract bet amount
///
/// Public inputs format (5 x 32 = 160 bytes for circuit):
/// 1. old_balance_commitment (32)
/// 2. new_balance_commitment (32)
/// 3. bet_commitment (32)
/// 4. max_bet (32, left-padded)
/// 5. bet_amount (32, left-padded)
fn verify_place_bet_proof(
    _zk_program_info: &AccountInfo,
    proof: &[u8],
    public_inputs: &[u8],
    old_commitment: &[u8; 32],
    new_commitment: &[u8; 32],
    bet_commitment: &[u8; 32],
    market_id: u64,
    max_bet: u64,
) -> Result<u64, ProgramError> {
    use crate::verifier::{verify_place_bet_proof as groth16_verify, GROTH16_PROOF_SIZE};

    // Validate proof size (Groth16: 256 bytes)
    if proof.len() != GROTH16_PROOF_SIZE {
        msg!("Invalid proof size: {} (expected {})", proof.len(), GROTH16_PROOF_SIZE);
        return Err(ProgramError::InvalidInstructionData);
    }

    // Accept either compact format (112 bytes) or full format (160 bytes)
    if public_inputs.len() < 112 {
        msg!("Invalid public inputs size: {} (expected >= 112)", public_inputs.len());
        return Err(ProgramError::InvalidInstructionData);
    }

    // Parse public inputs (compact format: 32 + 32 + 32 + 8 + 8 = 112)
    let pi_old_commitment = &public_inputs[0..32];
    let pi_new_commitment = &public_inputs[32..64];
    let pi_bet_commitment = &public_inputs[64..96];
    let pi_max_bet = u64::from_le_bytes(public_inputs[96..104].try_into().unwrap());
    let pi_bet_amount = u64::from_le_bytes(public_inputs[104..112].try_into().unwrap());

    // Verify public inputs match expected values
    if pi_old_commitment != old_commitment {
        msg!("Public input old_commitment mismatch");
        return Err(ProgramError::InvalidInstructionData);
    }

    if pi_new_commitment != new_commitment {
        msg!("Public input new_commitment mismatch");
        return Err(ProgramError::InvalidInstructionData);
    }

    if pi_bet_commitment != bet_commitment {
        msg!("Public input bet_commitment mismatch");
        return Err(ProgramError::InvalidInstructionData);
    }

    if pi_max_bet != max_bet {
        msg!("Public input max_bet mismatch: {} != {}", pi_max_bet, max_bet);
        return Err(ProgramError::InvalidInstructionData);
    }

    if pi_bet_amount == 0 {
        msg!("Bet amount cannot be 0");
        return Err(ProgramError::InvalidArgument);
    }

    if pi_bet_amount > max_bet {
        msg!("Bet amount exceeds max: {} > {}", pi_bet_amount, max_bet);
        return Err(ProgramError::InvalidArgument);
    }

    // Build circuit-format public inputs (5 x 32 = 160 bytes, big-endian)
    // Circuit expects: old_commitment, new_commitment, bet_commitment, market_id, max_bet
    // Input layout: [0-32: old][32-64: new][64-96: bet][96-128: market_id][128-160: max_bet]
    let mut circuit_inputs = [0u8; 160];
    circuit_inputs[0..32].copy_from_slice(pi_old_commitment);
    circuit_inputs[32..64].copy_from_slice(pi_new_commitment);
    circuit_inputs[64..96].copy_from_slice(pi_bet_commitment);
    // market_id as 32-byte big-endian (field element, u64 in last 8 bytes)
    circuit_inputs[120..128].copy_from_slice(&market_id.to_be_bytes());
    // max_bet as 32-byte big-endian (field element, u64 in last 8 bytes)
    circuit_inputs[152..160].copy_from_slice(&max_bet.to_be_bytes());

    // Verify Groth16 proof on-chain
    let is_valid = groth16_verify(proof, &circuit_inputs)?;

    if !is_valid {
        msg!("Groth16 proof verification FAILED");
        return Err(ProgramError::InvalidInstructionData);
    }

    msg!("ZK proof verified (circuit {})", CIRCUIT_PLACE_BET_PRIVATE);
    Ok(pi_bet_amount)
}
