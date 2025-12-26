//! Withdraw V2 - Withdraw SOL from ProtocolVault with ZK proof
//!
//! Requires ZK proof that user has sufficient balance (circuit 36)
//!
//! Flow:
//! 1. Verify ZK proof of sufficient balance
//! 2. Transfer lamports from ProtocolVault to user
//! 3. Update balance_commitment
//! 4. Emit event for FHE provers

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
};

use crate::state::{PrivateBalance, ProtocolVault, PRIVATE_BALANCE_SEED, PROTOCOL_VAULT_SEED};

/// Circuit type for WithdrawPrivate (36)
pub const CIRCUIT_WITHDRAW_PRIVATE: u8 = 36;

/// Process Withdraw instruction (V2)
///
/// Withdraws SOL from ProtocolVault with ZK proof of sufficient balance
///
/// Accounts expected:
/// 0. `[writable, signer]` User withdrawing
/// 1. `[writable]` PrivateBalance PDA
/// 2. `[writable]` ProtocolVault PDA
/// 3. `[]` ZK-generator program (for proof verification)
/// 4. `[]` System program
pub fn process_withdraw_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
    proof: Vec<u8>,
    public_inputs: Vec<u8>,
    new_balance_commitment: [u8; 32],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let user_info = next_account_info(account_info_iter)?;
    let private_balance_info = next_account_info(account_info_iter)?;
    let protocol_vault_info = next_account_info(account_info_iter)?;
    let zk_program_info = next_account_info(account_info_iter)?;
    let _system_program_info = next_account_info(account_info_iter)?;

    // Verify user is signer
    if !user_info.is_signer {
        msg!("User must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate amount
    if amount == 0 {
        msg!("Withdraw amount must be greater than 0");
        return Err(ProgramError::InvalidArgument);
    }

    // Verify PrivateBalance PDA
    let (private_balance_pda, _) = Pubkey::find_program_address(
        &[PRIVATE_BALANCE_SEED, user_info.key.as_ref()],
        program_id,
    );
    if private_balance_pda != *private_balance_info.key {
        msg!("Invalid PrivateBalance PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Load and verify PrivateBalance
    let balance_data = private_balance_info.try_borrow_data()?;
    let mut private_balance = PrivateBalance::deserialize(&mut &balance_data[..])?;
    if private_balance.user != *user_info.key {
        msg!("PrivateBalance user mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    drop(balance_data);

    // Verify ProtocolVault PDA
    let (protocol_vault_pda, vault_bump) = ProtocolVault::find_pda(program_id);
    if protocol_vault_pda != *protocol_vault_info.key {
        msg!("Invalid ProtocolVault PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Check vault has sufficient funds
    if protocol_vault_info.lamports() < amount {
        msg!("Insufficient funds in ProtocolVault: {} < {}",
             protocol_vault_info.lamports(), amount);
        return Err(ProgramError::InsufficientFunds);
    }

    // Verify ZK proof (circuit 36: WithdrawPrivate)
    // Public inputs should contain:
    // - old_balance_commitment (32 bytes)
    // - new_balance_commitment (32 bytes)
    // - withdraw_amount_public (8 bytes) = 72 bytes minimum
    verify_withdraw_proof(
        zk_program_info,
        &proof,
        &public_inputs,
        &private_balance.balance_commitment,
        &new_balance_commitment,
        amount,
    )?;

    // Transfer lamports from ProtocolVault to user (signed by PDA)
    msg!("Transferring {} lamports from ProtocolVault to user", amount);

    let vault_seeds = &[PROTOCOL_VAULT_SEED, &[vault_bump]];

    **protocol_vault_info.try_borrow_mut_lamports()? -= amount;
    **user_info.try_borrow_mut_lamports()? += amount;

    // Update PrivateBalance commitment
    let mut balance_data = private_balance_info.try_borrow_mut_data()?;
    private_balance.update_commitment(new_balance_commitment);
    private_balance.serialize(&mut &mut balance_data[..])?;

    msg!("Withdraw successful");
    msg!("  User: {}", user_info.key);
    msg!("  Amount: {} lamports", amount);
    msg!("  New nonce: {}", private_balance.nonce);
    msg!("  ProtocolVault remaining: {} lamports", protocol_vault_info.lamports());
    msg!("  Pending: FHE balance update by off-chain provers");

    // Emit event for FHE provers
    msg!("EVENT:WITHDRAW:{}:{}:{}", user_info.key, amount, private_balance.nonce);

    Ok(())
}

/// Verify ZK proof for withdrawal
///
/// Public inputs format (3 field elements = 96 bytes for circuit):
/// 1. old_balance_commitment (32 bytes)
/// 2. new_balance_commitment (32 bytes)
/// 3. withdraw_amount (32 bytes, left-padded from u64)
fn verify_withdraw_proof(
    _zk_program_info: &AccountInfo,
    proof: &[u8],
    public_inputs: &[u8],
    old_commitment: &[u8; 32],
    new_commitment: &[u8; 32],
    amount: u64,
) -> ProgramResult {
    use crate::verifier::{verify_withdraw_proof as groth16_verify, GROTH16_PROOF_SIZE};

    // Validate proof size (Groth16: 256 bytes)
    if proof.len() != GROTH16_PROOF_SIZE {
        msg!("Invalid proof size: {} (expected {})", proof.len(), GROTH16_PROOF_SIZE);
        return Err(ProgramError::InvalidInstructionData);
    }

    // Accept compact format (72 bytes) from CLI
    // Compact: old_commitment(32) + new_commitment(32) + amount(8)
    if public_inputs.len() < 72 {
        msg!("Invalid public inputs size: {} (expected >= 72)", public_inputs.len());
        return Err(ProgramError::InvalidInstructionData);
    }

    // Parse public inputs (compact format)
    let pi_old_commitment = &public_inputs[0..32];
    let pi_new_commitment = &public_inputs[32..64];
    let pi_amount = u64::from_le_bytes(public_inputs[64..72].try_into().unwrap());

    // Verify public inputs match expected values
    if pi_old_commitment != old_commitment {
        msg!("Public input old_commitment mismatch");
        return Err(ProgramError::InvalidInstructionData);
    }

    if pi_new_commitment != new_commitment {
        msg!("Public input new_commitment mismatch");
        return Err(ProgramError::InvalidInstructionData);
    }

    if pi_amount != amount {
        msg!("Public input amount mismatch: {} != {}", pi_amount, amount);
        return Err(ProgramError::InvalidInstructionData);
    }

    if pi_amount == 0 {
        msg!("Withdraw amount cannot be 0");
        return Err(ProgramError::InvalidArgument);
    }

    // Build circuit-format public inputs (3 x 32 = 96 bytes, big-endian field elements)
    let mut circuit_inputs = [0u8; 96];
    circuit_inputs[0..32].copy_from_slice(pi_old_commitment);
    circuit_inputs[32..64].copy_from_slice(pi_new_commitment);
    // amount as 32-byte big-endian (field element)
    circuit_inputs[88..96].copy_from_slice(&pi_amount.to_be_bytes());

    // Verify Groth16 proof on-chain
    let is_valid = groth16_verify(proof, &circuit_inputs)?;

    if !is_valid {
        msg!("Groth16 proof verification FAILED");
        return Err(ProgramError::InvalidInstructionData);
    }

    msg!("ZK proof verified (circuit {})", CIRCUIT_WITHDRAW_PRIVATE);
    Ok(())
}
