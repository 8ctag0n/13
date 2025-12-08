//! Program instruction processor

use borsh::BorshDeserialize;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey,
};

use crate::{error::ThresholdError, instruction::ThresholdInstruction};

pub mod request_keyshare;
pub mod submit_keyshare;

pub use request_keyshare::*;
pub use submit_keyshare::*;

/// Process threshold program instructions
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = ThresholdInstruction::try_from_slice(instruction_data)
        .map_err(|_| ThresholdError::InvalidInstruction)?;

    match instruction {
        ThresholdInstruction::RequestKeyShare {
            encrypted_witness_cid,
        } => {
            msg!("Instruction: RequestKeyShare");
            request_keyshare::process_request_keyshare(
                program_id,
                accounts,
                encrypted_witness_cid,
            )
        }
        ThresholdInstruction::SubmitKeyShare { encrypted_share } => {
            msg!("Instruction: SubmitKeyShare");
            submit_keyshare::process_submit_keyshare(program_id, accounts, encrypted_share)
        }
    }
}
