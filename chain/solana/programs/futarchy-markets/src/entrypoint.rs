//! Program entrypoint
//!
//! Supports V1, V2, and V3 instruction sets.
//! Attempts V3 first (Blind Markets), then V2 (PrivateBalance), falls back to V1.

use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, msg, pubkey::Pubkey,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Try V3 instructions first (Blind Markets with FHE+ZK)
    if crate::processor_v3::is_v3_instruction(instruction_data) {
        msg!("Processing V3 instruction (Blind Markets)");
        return crate::processor_v3::process_v3(program_id, accounts, instruction_data);
    }

    // Try V2 instructions (PrivateBalance architecture)
    if crate::processor_v2::is_v2_instruction(instruction_data) {
        msg!("Processing V2 instruction (PrivateBalance)");
        return crate::processor_v2::process_v2(program_id, accounts, instruction_data);
    }

    // Fall back to V1 instructions
    msg!("Processing V1 instruction");
    crate::processor::process(program_id, accounts, instruction_data)
}
