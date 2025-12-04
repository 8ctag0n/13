#![allow(unexpected_cfgs, deprecated)]

pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

pub use error::*;
pub use instruction::*;
pub use state::*;

use solana_program::{account_info::AccountInfo, entrypoint, pubkey::Pubkey};

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    processor::process(program_id, accounts, instruction_data)
}
