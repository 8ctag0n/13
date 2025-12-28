//! PlaceBetBlind - Place a fully blind bet on a market V3
//!
//! Flow:
//! 1. Verify ZK proof (circuit 50: PlaceBetBlind)
//! 2. Create PositionV3 PDA (bet_ciphertext_hash as seed)
//! 3. Update PrivateBalance commitment
//! 4. Update MarketV3 pool_commitment
//! 5. Transfer lamports: ProtocolVault -> MarketVault
//! 6. Emit event for off-chain FHE pool update

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
    program::invoke_signed,
};

use crate::error::FutarchyError;
use crate::state::{
    MarketV3, MarketVault, PositionV3, PrivateBalance, ProtocolVault,
    MARKET_V3_SEED, MARKET_VAULT_SEED, POSITION_V3_SEED, PRIVATE_BALANCE_SEED, PROTOCOL_VAULT_SEED,
};

/// Circuit type for PlaceBetBlind (50)
pub const CIRCUIT_PLACE_BET_BLIND: u8 = 50;

/// Process PlaceBetBlind instruction
///
/// Places a fully blind bet where amount and side are hidden
///
/// Accounts expected:
/// 0. `[writable, signer]` User placing bet
/// 1. `[writable]` PrivateBalance PDA
/// 2. `[writable]` MarketV3 PDA
/// 3. `[writable]` PositionV3 PDA (will be created)
/// 4. `[writable]` ProtocolVault PDA (source of funds)
/// 5. `[writable]` MarketVault PDA (destination)
/// 6. `[]` ZK-generator program
/// 7. `[]` System program
/// 8. `[]` Clock sysvar
#[allow(clippy::too_many_arguments)]
pub fn process_place_bet_blind(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    market_id: u64,
    bet_ciphertext_hash: [u8; 32],
    bet_secret_commitment: [u8; 32],
    pool_commitment_after: [u8; 32],
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
    if circuit_type != CIRCUIT_PLACE_BET_BLIND {
        msg!("Invalid circuit type: {} (expected {})", circuit_type, CIRCUIT_PLACE_BET_BLIND);
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

    // Verify MarketV3 PDA
    let (market_pda, _) = Pubkey::find_program_address(
        &[MARKET_V3_SEED, &market_id_bytes],
        program_id,
    );
    if market_pda != *market_info.key {
        msg!("Invalid MarketV3 PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Load and validate market
    let market_data = market_info.try_borrow_data()?;
    let mut market = MarketV3::deserialize(&mut &market_data[..])?;
    drop(market_data);

    if !market.is_active() {
        msg!("Market is not active");
        return Err(FutarchyError::MarketNotActive.into());
    }

    if market.is_betting_closed(current_time) {
        msg!("Betting period has ended");
        return Err(FutarchyError::BettingClosed.into());
    }

    // Verify PositionV3 PDA
    let (position_pda, position_bump) = Pubkey::find_program_address(
        &[POSITION_V3_SEED, &market_id_bytes, &bet_ciphertext_hash],
        program_id,
    );
    if position_pda != *position_info.key {
        msg!("Invalid PositionV3 PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Check position doesn't already exist (same bet_ciphertext_hash)
    if position_info.data_len() > 0 {
        msg!("Position with this bet_ciphertext_hash already exists");
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

    // Verify ZK proof (circuit 50: PlaceBetBlind)
    // Extract bet amount from public inputs for the transfer
    let bet_amount = verify_place_bet_blind_proof(
        zk_program_info,
        &proof,
        &public_inputs,
        &market.pool_commitment,
        &pool_commitment_after,
        &bet_ciphertext_hash,
        market_id,
        market.max_bet,
    )?;

    // Check vault has sufficient funds
    if protocol_vault_info.lamports() < bet_amount {
        msg!("Insufficient funds in ProtocolVault");
        return Err(ProgramError::InsufficientFunds);
    }

    // Create PositionV3 PDA (fully blind - stores hashes only!)
    let rent = Rent::get()?;
    let position_space = PositionV3::SPACE;
    let position_lamports = rent.minimum_balance(position_space);
    let position_seeds = &[
        POSITION_V3_SEED,
        &market_id_bytes,
        &bet_ciphertext_hash,
        &[position_bump],
    ];

    msg!("Creating fully blind PositionV3 PDA");
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

    // Initialize PositionV3 (only hashes stored, amount/side fully private)
    let position = PositionV3::new(
        market_id,
        bet_ciphertext_hash,
        bet_secret_commitment,
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

    // Update MarketV3 pool commitment (ZK-verified transition)
    let mut market_data = market_info.try_borrow_mut_data()?;
    market.update_pool_commitment(pool_commitment_after);
    market.serialize(&mut &mut market_data[..])?;

    msg!("Blind bet placed successfully");
    msg!("  Market ID: {}", market_id);
    msg!("  Bet ciphertext hash: {:?}", &bet_ciphertext_hash[..8]);
    msg!("  Position PDA: {}", position_info.key);
    msg!("  Amount transferred: {} lamports", bet_amount);
    msg!("  Pool commitment updated (blind)");

    // Emit event for off-chain FHE pool update
    // Off-chain provers will:
    // 1. Fetch bet FHE ciphertext by hash
    // 2. Homomorphically add to encrypted pool
    // 3. Submit new encrypted pool hash via UpdateEncryptedPools
    msg!(
        "EVENT:PLACE_BET_BLIND:{}:{:?}",
        market_id,
        &bet_ciphertext_hash[..8]
    );

    Ok(())
}

/// Verify ZK proof for blind bet and extract bet amount
///
/// Public inputs format (compact: 112 bytes):
/// 1. pool_commitment_before (32)
/// 2. pool_commitment_after (32)
/// 3. bet_ciphertext_hash (32)
/// 4. market_id (8)
/// 5. max_bet (8)
///
/// The proof verifies that the prover knows:
/// - The plaintext pool values matching pool_commitment_before
/// - The correct pool update based on secret bet amount and side
/// - The updated pool values matching pool_commitment_after
///
/// IMPORTANT: bet_amount is NOT in the circuit public inputs.
/// It's extracted from the proof's auxiliary data for the on-chain transfer.
/// This is a simplification for V3 - in production, amount should come from
/// a separate ZK proof or be derived from balance commitment delta.
fn verify_place_bet_blind_proof(
    _zk_program_info: &AccountInfo,
    proof: &[u8],
    public_inputs: &[u8],
    pool_commitment_before: &[u8; 32],
    pool_commitment_after: &[u8; 32],
    bet_ciphertext_hash: &[u8; 32],
    market_id: u64,
    max_bet: u64,
) -> Result<u64, ProgramError> {
    use crate::verifier::{verify_place_bet_blind_proof as groth16_verify, GROTH16_PROOF_SIZE};

    // Validate proof size (Groth16: 256 bytes)
    if proof.len() != GROTH16_PROOF_SIZE {
        msg!("Invalid proof size: {} (expected {})", proof.len(), GROTH16_PROOF_SIZE);
        return Err(ProgramError::InvalidInstructionData);
    }

    // Validate public inputs size (compact format: 112 bytes minimum)
    if public_inputs.len() < 112 {
        msg!("Invalid public inputs size: {} (expected >= 112)", public_inputs.len());
        return Err(ProgramError::InvalidInstructionData);
    }

    // Parse public inputs
    let pi_pool_commitment_before = &public_inputs[0..32];
    let pi_pool_commitment_after = &public_inputs[32..64];
    let pi_bet_ciphertext_hash = &public_inputs[64..96];
    let pi_market_id = u64::from_le_bytes(public_inputs[96..104].try_into().unwrap());
    let pi_max_bet = u64::from_le_bytes(public_inputs[104..112].try_into().unwrap());

    // Verify public inputs match expected values
    if pi_pool_commitment_before != pool_commitment_before {
        msg!("Public input pool_commitment_before mismatch");
        return Err(ProgramError::InvalidInstructionData);
    }

    if pi_pool_commitment_after != pool_commitment_after {
        msg!("Public input pool_commitment_after mismatch");
        return Err(ProgramError::InvalidInstructionData);
    }

    if pi_bet_ciphertext_hash != bet_ciphertext_hash {
        msg!("Public input bet_ciphertext_hash mismatch");
        return Err(ProgramError::InvalidInstructionData);
    }

    if pi_market_id != market_id {
        msg!("Public input market_id mismatch: {} != {}", pi_market_id, market_id);
        return Err(ProgramError::InvalidInstructionData);
    }

    if pi_max_bet != max_bet {
        msg!("Public input max_bet mismatch: {} != {}", pi_max_bet, max_bet);
        return Err(ProgramError::InvalidInstructionData);
    }

    // Build circuit-format public inputs (5 x 32 = 160 bytes, big-endian)
    // Circuit expects: pool_commitment_before, pool_commitment_after,
    //                  bet_ciphertext_hash, market_id, max_bet
    let mut circuit_inputs = [0u8; 160];
    circuit_inputs[0..32].copy_from_slice(pi_pool_commitment_before);
    circuit_inputs[32..64].copy_from_slice(pi_pool_commitment_after);
    circuit_inputs[64..96].copy_from_slice(pi_bet_ciphertext_hash);
    // market_id as 32-byte big-endian (field element)
    circuit_inputs[120..128].copy_from_slice(&pi_market_id.to_be_bytes());
    // max_bet as 32-byte big-endian (field element)
    circuit_inputs[152..160].copy_from_slice(&pi_max_bet.to_be_bytes());

    // Verify Groth16 proof on-chain
    let is_valid = groth16_verify(proof, &circuit_inputs)?;

    if !is_valid {
        msg!("Groth16 proof verification FAILED");
        return Err(ProgramError::InvalidInstructionData);
    }

    msg!("ZK proof verified (circuit {})", CIRCUIT_PLACE_BET_BLIND);

    // Extract bet_amount for lamport transfer
    // V3 simplification: amount comes from auxiliary data (bytes 112-120)
    // In production: derive from balance commitment delta or separate proof
    let bet_amount = if public_inputs.len() >= 120 {
        u64::from_le_bytes(public_inputs[112..120].try_into().unwrap())
    } else {
        msg!("No bet_amount in public_inputs, using placeholder");
        return Err(ProgramError::InvalidInstructionData);
    };

    if bet_amount == 0 {
        msg!("Bet amount cannot be 0");
        return Err(ProgramError::InvalidArgument);
    }

    if bet_amount > max_bet {
        msg!("Bet amount exceeds max: {} > {}", bet_amount, max_bet);
        return Err(ProgramError::InvalidArgument);
    }

    Ok(bet_amount)
}
