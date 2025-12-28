//! ClaimV2 - Claim payout from a settled market with ZK proof
//!
//! New payout model (V2 with vote counting):
//! payout = bet_amount + (vault_total / winner_count)
//!
//! Flow:
//! 1. Verify ZK proof (circuit 34: ClaimPrivate)
//!    - Proves ownership of bet_commitment
//!    - Proves bet was on winning side
//!    - Reveals bet_amount for payout calculation
//! 2. Create Nullifier PDA to prevent double-claim
//! 3. Mark PositionV2 as claimed
//! 4. Calculate payout: bet_amount + (vault_total / winner_count)
//! 5. Transfer lamports: MarketVault -> ProtocolVault
//! 6. Update PrivateBalance commitment
//! 7. Emit event for FHE provers

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
    MarketV2, MarketVault, Nullifier, PositionV2, PrivateBalance, ProtocolVault,
    MARKET_V2_SEED, NULLIFIER_SEED, POSITION_V2_SEED,
    PRIVATE_BALANCE_SEED,
};

/// Circuit type for ClaimPrivate (34)
pub const CIRCUIT_CLAIM_PRIVATE: u8 = 34;

/// Process ClaimV2 instruction
///
/// Claims payout from a settled market with ZK proof of ownership
///
/// Payout formula: bet_amount + (vault_total / winner_count)
///
/// Accounts expected:
/// 0. `[writable, signer]` User claiming
/// 1. `[writable]` PrivateBalance PDA
/// 2. `[]` MarketV2 PDA (must be settled)
/// 3. `[writable]` PositionV2 PDA (will mark as claimed)
/// 4. `[writable]` Nullifier PDA (will be created)
/// 5. `[writable]` MarketVault PDA (source of payout)
/// 6. `[writable]` ProtocolVault PDA (destination)
/// 7. `[]` ZK-generator program
/// 8. `[]` System program
/// 9. `[]` Clock sysvar
#[allow(clippy::too_many_arguments)]
pub fn process_claim_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    market_id: u64,
    nullifier_hash: [u8; 32],
    proof: Vec<u8>,
    public_inputs: Vec<u8>,
    new_balance_commitment: [u8; 32],
    encrypted_payout: Vec<u8>,
    bet_commitment: [u8; 32],
    circuit_type: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let user_info = next_account_info(account_info_iter)?;
    let private_balance_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let position_info = next_account_info(account_info_iter)?;
    let nullifier_info = next_account_info(account_info_iter)?;
    let market_vault_info = next_account_info(account_info_iter)?;
    let protocol_vault_info = next_account_info(account_info_iter)?;
    let zk_program_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;
    let clock_sysvar_info = next_account_info(account_info_iter)?;

    // Verify user is signer
    if !user_info.is_signer {
        msg!("User must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate circuit type
    if circuit_type != CIRCUIT_CLAIM_PRIVATE {
        msg!("Invalid circuit type: {} (expected {})", circuit_type, CIRCUIT_CLAIM_PRIVATE);
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
    let market = MarketV2::deserialize(&mut &market_data[..])?;
    drop(market_data);

    if !market.is_settled() {
        msg!("Market is not settled");
        return Err(FutarchyError::MarketNotSettled.into());
    }

    let resolution = market.resolution.ok_or_else(|| {
        msg!("Market has no resolution");
        FutarchyError::MarketNotSettled
    })?;

    // Get winner count for payout calculation
    let winner_count = market.winner_count().ok_or_else(|| {
        msg!("Failed to get winner count");
        FutarchyError::MarketNotSettled
    })?;

    if winner_count == 0 {
        msg!("No winners - no payout possible");
        return Err(FutarchyError::NoWinners.into());
    }

    // Verify PositionV2 PDA
    let (position_pda, _) = Pubkey::find_program_address(
        &[POSITION_V2_SEED, &market_id_bytes, &bet_commitment],
        program_id,
    );
    if position_pda != *position_info.key {
        msg!("Invalid PositionV2 PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Load and validate position
    let position_data = position_info.try_borrow_data()?;
    let mut position = PositionV2::deserialize(&mut &position_data[..])?;
    drop(position_data);

    if position.is_claimed() {
        msg!("Position already claimed");
        return Err(FutarchyError::AlreadyClaimed.into());
    }

    if position.bet_commitment != bet_commitment {
        msg!("Bet commitment mismatch");
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify Nullifier PDA (shouldn't exist yet)
    let (nullifier_pda, nullifier_bump) = Pubkey::find_program_address(
        &[NULLIFIER_SEED, &market_id_bytes, &nullifier_hash],
        program_id,
    );
    if nullifier_pda != *nullifier_info.key {
        msg!("Invalid Nullifier PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Check nullifier doesn't already exist (prevent double-claim)
    if nullifier_info.data_len() > 0 {
        msg!("Nullifier already exists - double claim attempt");
        return Err(FutarchyError::AlreadyClaimed.into());
    }

    // Verify vault PDAs
    let (market_vault_pda, _) = MarketVault::find_pda(market_id, program_id);
    if market_vault_pda != *market_vault_info.key {
        msg!("Invalid MarketVault PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    let (protocol_vault_pda, _) = ProtocolVault::find_pda(program_id);
    if protocol_vault_pda != *protocol_vault_info.key {
        msg!("Invalid ProtocolVault PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Get vault total for bonus calculation
    let vault_total = market_vault_info.lamports();

    // Verify ZK proof and extract bet_amount and bet_side
    // The proof verifies:
    // - User knows the secret for bet_commitment
    // - bet_side matches resolution (winning side)
    // - Returns bet_amount for payout calculation
    let (bet_amount, bet_side) = verify_claim_proof(
        zk_program_info,
        &proof,
        &public_inputs,
        &bet_commitment,
        resolution,
        &nullifier_hash,
        &private_balance.balance_commitment,
        &new_balance_commitment,
    )?;

    // Verify bet was on winning side
    if bet_side != resolution {
        msg!("Bet lost - no payout");
        // Mark as claimed anyway to prevent re-attempts
        position.mark_claimed();
        let mut position_data = position_info.try_borrow_mut_data()?;
        position.serialize(&mut &mut position_data[..])?;
        return Ok(());
    }

    // V2 Payout Model: Equitative distribution
    // payout = vault_total / winner_count
    //
    // All winners receive equal payout regardless of bet amount.
    // This preserves privacy (no on-chain amount tracking) but is less "fair".
    //
    // V3 with FHE will implement proportional payouts:
    // payout = bet_amount + (bet_amount / total_winner_pool) * loser_pool
    // FHE allows computing this without revealing individual amounts.
    //
    // Trade-off: Privacy > Proportional fairness for V2.
    let payout_amount = vault_total.saturating_div(winner_count);

    msg!("Payout calculation:");
    msg!("  Vault total: {} lamports", vault_total);
    msg!("  Winner count: {}", winner_count);
    msg!("  Payout per winner: {} lamports", payout_amount);
    msg!("  (Bet amount from proof: {} lamports - for reference)", bet_amount);

    if payout_amount == 0 {
        msg!("Payout is 0 - marking as claimed");
        position.mark_claimed();
        let mut position_data = position_info.try_borrow_mut_data()?;
        position.serialize(&mut &mut position_data[..])?;
        return Ok(());
    }

    // Check vault has sufficient funds
    if market_vault_info.lamports() < payout_amount {
        msg!("Insufficient funds in MarketVault: {} < {}", market_vault_info.lamports(), payout_amount);
        return Err(ProgramError::InsufficientFunds);
    }

    // Create Nullifier PDA
    let rent = Rent::get()?;
    let nullifier_space = Nullifier::SPACE;
    let nullifier_lamports = rent.minimum_balance(nullifier_space);
    let nullifier_seeds = &[
        NULLIFIER_SEED,
        &market_id_bytes,
        &nullifier_hash,
        &[nullifier_bump],
    ];

    msg!("Creating Nullifier PDA");
    invoke_signed(
        &system_instruction::create_account(
            user_info.key,
            nullifier_info.key,
            nullifier_lamports,
            nullifier_space as u64,
            program_id,
        ),
        &[
            user_info.clone(),
            nullifier_info.clone(),
            system_program_info.clone(),
        ],
        &[nullifier_seeds],
    )?;

    // Initialize Nullifier
    let nullifier = Nullifier::new(
        market_id,
        nullifier_hash,
        *user_info.key,
        current_time,
        payout_amount,
        nullifier_bump,
    );
    let mut nullifier_data = nullifier_info.try_borrow_mut_data()?;
    nullifier.serialize(&mut &mut nullifier_data[..])?;
    drop(nullifier_data);

    // Mark position as claimed
    position.mark_claimed();
    let mut position_data = position_info.try_borrow_mut_data()?;
    position.serialize(&mut &mut position_data[..])?;
    drop(position_data);

    // Transfer lamports: MarketVault -> ProtocolVault
    msg!("Transferring {} lamports from MarketVault to ProtocolVault", payout_amount);

    **market_vault_info.try_borrow_mut_lamports()? -= payout_amount;
    **protocol_vault_info.try_borrow_mut_lamports()? += payout_amount;

    // Update PrivateBalance commitment
    let mut balance_data = private_balance_info.try_borrow_mut_data()?;
    private_balance.update_commitment(new_balance_commitment);
    private_balance.serialize(&mut &mut balance_data[..])?;

    msg!("Claim successful");
    msg!("  Market ID: {}", market_id);
    msg!("  Bet side: {}", if bet_side { "YES" } else { "NO" });
    msg!("  Resolution: {}", if resolution { "YES" } else { "NO" });
    msg!("  Payout: {} lamports (vault/winners)", payout_amount);

    // Emit event for FHE provers
    msg!(
        "EVENT:CLAIM:{}:{:?}:{}:{:?}",
        market_id,
        &bet_commitment[..8],
        payout_amount,
        &encrypted_payout[..std::cmp::min(8, encrypted_payout.len())]
    );

    Ok(())
}

/// Verify ZK proof for claim and extract bet_amount and bet_side
///
/// The proof verifies:
/// - User knows the secret for bet_commitment: Poseidon(amount, side, secret) = bet_commitment
/// - Returns bet_amount and bet_side for payout calculation
///
/// Compact public inputs layout (138 bytes):
/// - bet_commitment: 32 bytes
/// - resolution: 1 byte
/// - nullifier_hash: 32 bytes
/// - bet_amount: 8 bytes
/// - bet_side: 1 byte
/// - old_balance_commitment: 32 bytes
/// - new_balance_commitment: 32 bytes
///
/// Circuit public inputs (7 x 32 = 224 bytes):
/// 1. bet_commitment (32)
/// 2. resolution (32, left-padded bool)
/// 3. nullifier_hash (32)
/// 4. bet_amount (32, left-padded)
/// 5. bet_side (32, left-padded bool)
/// 6. old_balance_commitment (32)
/// 7. new_balance_commitment (32)
fn verify_claim_proof(
    _zk_program_info: &AccountInfo,
    proof: &[u8],
    public_inputs: &[u8],
    bet_commitment: &[u8; 32],
    resolution: bool,
    nullifier_hash: &[u8; 32],
    old_commitment: &[u8; 32],
    new_commitment: &[u8; 32],
) -> Result<(u64, bool), ProgramError> {
    use crate::verifier::{verify_claim_proof as groth16_verify, GROTH16_PROOF_SIZE};

    // Validate proof size (Groth16: 256 bytes)
    if proof.len() != GROTH16_PROOF_SIZE {
        msg!("Invalid proof size: {} (expected {})", proof.len(), GROTH16_PROOF_SIZE);
        return Err(ProgramError::InvalidInstructionData);
    }

    // Validate public inputs size (compact format)
    // bet_commitment (32) + resolution (1) + nullifier (32) +
    // bet_amount (8) + bet_side (1) + old_commitment (32) + new_commitment (32) = 138 bytes
    if public_inputs.len() < 138 {
        msg!("Invalid public inputs size: {} (expected >= 138)", public_inputs.len());
        return Err(ProgramError::InvalidInstructionData);
    }

    // Parse compact public inputs
    let pi_bet_commitment = &public_inputs[0..32];
    let pi_resolution = public_inputs[32] != 0;
    let pi_nullifier = &public_inputs[33..65];
    let pi_bet_amount = u64::from_le_bytes(public_inputs[65..73].try_into().unwrap());
    let pi_bet_side = public_inputs[73] != 0;
    let pi_old_commitment = &public_inputs[74..106];
    let pi_new_commitment = &public_inputs[106..138];

    // Verify public inputs match expected values
    if pi_bet_commitment != bet_commitment {
        msg!("Public input bet_commitment mismatch");
        return Err(ProgramError::InvalidInstructionData);
    }

    if pi_resolution != resolution {
        msg!("Public input resolution mismatch");
        return Err(ProgramError::InvalidInstructionData);
    }

    if pi_nullifier != nullifier_hash {
        msg!("Public input nullifier mismatch");
        return Err(ProgramError::InvalidInstructionData);
    }

    if pi_old_commitment != old_commitment {
        msg!("Public input old_commitment mismatch");
        return Err(ProgramError::InvalidInstructionData);
    }

    if pi_new_commitment != new_commitment {
        msg!("Public input new_commitment mismatch");
        return Err(ProgramError::InvalidInstructionData);
    }

    if pi_bet_amount == 0 {
        msg!("Bet amount cannot be 0");
        return Err(ProgramError::InvalidArgument);
    }

    // Build circuit-format public inputs (7 x 32 = 224 bytes, big-endian)
    let mut circuit_inputs = [0u8; 224];
    // 1. bet_commitment (32)
    circuit_inputs[0..32].copy_from_slice(pi_bet_commitment);
    // 2. resolution as 32-byte big-endian bool
    circuit_inputs[63] = if pi_resolution { 1 } else { 0 };
    // 3. nullifier_hash (32)
    circuit_inputs[64..96].copy_from_slice(pi_nullifier);
    // 4. bet_amount as 32-byte big-endian
    circuit_inputs[120..128].copy_from_slice(&pi_bet_amount.to_be_bytes());
    // 5. bet_side as 32-byte big-endian bool
    circuit_inputs[159] = if pi_bet_side { 1 } else { 0 };
    // 6. old_balance_commitment (32)
    circuit_inputs[160..192].copy_from_slice(pi_old_commitment);
    // 7. new_balance_commitment (32)
    circuit_inputs[192..224].copy_from_slice(pi_new_commitment);

    // Verify Groth16 proof on-chain
    let is_valid = groth16_verify(proof, &circuit_inputs)?;

    if !is_valid {
        msg!("Groth16 proof verification FAILED");
        return Err(ProgramError::InvalidInstructionData);
    }

    msg!("ZK proof verified (circuit {})", CIRCUIT_CLAIM_PRIVATE);
    msg!("  Extracted bet_amount: {} lamports", pi_bet_amount);
    msg!("  Extracted bet_side: {}", if pi_bet_side { "YES" } else { "NO" });

    Ok((pi_bet_amount, pi_bet_side))
}
