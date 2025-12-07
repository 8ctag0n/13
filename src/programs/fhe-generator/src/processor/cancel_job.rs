//! CancelJob instruction processor

use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

/// Process CancelJob instruction
pub fn process_cancel_job(_program_id: &Pubkey, _accounts: &[AccountInfo]) -> ProgramResult {
    // TODO: Implement in Day 8
    Ok(())
}
