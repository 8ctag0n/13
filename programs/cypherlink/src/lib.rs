pub mod state;
pub mod error;
pub mod instruction;
pub mod processor;

pub use state::*;
pub use error::*;
pub use instruction::*;

use solana_program::{
    account_info::AccountInfo, entrypoint, pubkey::Pubkey,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    processor::process(program_id, accounts, instruction_data)
}
