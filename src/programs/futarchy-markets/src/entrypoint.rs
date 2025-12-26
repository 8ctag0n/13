//! Program entrypoint
//!
//! Supports both V1 and V2 instruction sets.
//! Attempts V2 first (PrivateBalance architecture), falls back to V1.

use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, msg, pubkey::Pubkey,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Try V2 instructions first (PrivateBalance architecture)
    if crate::processor_v2::is_v2_instruction(instruction_data) {
        msg!("Processing V2 instruction");
        return crate::processor_v2::process_v2(program_id, accounts, instruction_data);
    }

    // Fall back to V1 instructions
    msg!("Processing V1 instruction");
    crate::processor::process(program_id, accounts, instruction_data)
}
