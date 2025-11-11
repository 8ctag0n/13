pub mod state;
pub mod error;

pub use state::*;
pub use error::*;

use solana_program::{
    account_info::AccountInfo, entrypoint, pubkey::Pubkey,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    // TODO: Implement instruction processing
    // This will be implemented in Day 2
    Ok(())
}
