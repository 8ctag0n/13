//! Deposit V2 - Deposit SOL to ProtocolVault
//!
//! Simple deposit flow:
//! 1. User transfers lamports to ProtocolVault
//! 2. Emits event for FHE provers to update encrypted_balance
//!
//! Note: In V2, the deposit amount is PUBLIC (visible on-chain as transfer).
//! Only the internal balance becomes private after FHE processing.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
};

use crate::state::{PrivateBalance, ProtocolVault, PRIVATE_BALANCE_SEED, PROTOCOL_VAULT_SEED};

/// Process Deposit instruction (V2)
///
/// Deposits SOL from user to ProtocolVault
/// Updates balance commitment immediately
/// FHE balance update happens async via off-chain provers
///
/// Accounts expected:
/// 0. `[writable, signer]` User depositing
/// 1. `[writable]` PrivateBalance PDA
/// 2. `[writable]` ProtocolVault PDA
/// 3. `[]` System program
pub fn process_deposit_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
    new_commitment: [u8; 32],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let user_info = next_account_info(account_info_iter)?;
    let private_balance_info = next_account_info(account_info_iter)?;
    let protocol_vault_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    // Verify user is signer
    if !user_info.is_signer {
        msg!("User must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate amount
    if amount == 0 {
        msg!("Deposit amount must be greater than 0");
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

    // Verify PrivateBalance exists and belongs to user
    let balance_data = private_balance_info.try_borrow_data()?;
    let private_balance = PrivateBalance::deserialize(&mut &balance_data[..])?;
    if private_balance.user != *user_info.key {
        msg!("PrivateBalance user mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    drop(balance_data);

    // Verify ProtocolVault PDA
    let (protocol_vault_pda, _) = ProtocolVault::find_pda(program_id);
    if protocol_vault_pda != *protocol_vault_info.key {
        msg!("Invalid ProtocolVault PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Transfer lamports from user to ProtocolVault
    msg!("Transferring {} lamports to ProtocolVault", amount);
    invoke(
        &system_instruction::transfer(user_info.key, protocol_vault_info.key, amount),
        &[
            user_info.clone(),
            protocol_vault_info.clone(),
            system_program_info.clone(),
        ],
    )?;

    // Update commitment and nonce in PrivateBalance
    let new_nonce = {
        let mut balance_data = private_balance_info.try_borrow_mut_data()?;
        let mut private_balance = PrivateBalance::deserialize(&mut &balance_data[..])?;
        private_balance.update_commitment(new_commitment);
        let new_nonce = private_balance.nonce;
        let serialized = borsh::to_vec(&private_balance)
            .map_err(|_| ProgramError::InvalidAccountData)?;
        balance_data[..serialized.len()].copy_from_slice(&serialized);
        new_nonce
    };

    // Note: FHE balance update happens off-chain
    // FHE provers will:
    // 1. Listen to this deposit event
    // 2. Compute: new_encrypted_balance = encrypted_balance + FHE(amount)
    // 3. Update PrivateBalance.encrypted_balance via consensus

    msg!("Deposit successful");
    msg!("  User: {}", user_info.key);
    msg!("  Amount: {} lamports", amount);
    msg!("  ProtocolVault new balance: {} lamports", protocol_vault_info.lamports());
    msg!("  Commitment updated, new nonce: {}", new_nonce);
    msg!("  Pending: FHE balance update by off-chain provers");

    // Emit event for FHE provers (via logs)
    msg!("EVENT:DEPOSIT:{}:{}:{}", user_info.key, amount, new_nonce);

    Ok(())
}
