//! CreateJob instruction processor

use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

/// Process CreateJob instruction
#[allow(clippy::too_many_arguments)]
pub fn process_create_job(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _circuit_type: u8,
    _witness_hash: [u8; 32],
    _witness_size: u32,
    _price_lamports: u64,
    _timeout_seconds: i64,
) -> ProgramResult {
    // TODO: Implement in Day 5
    Ok(())
}
