//! FHE Generator instruction processor

use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

pub mod create_job;
pub mod claim_job;
pub mod submit_result;
pub mod finalize_job;
pub mod cancel_job;

pub use create_job::*;
pub use claim_job::*;
pub use submit_result::*;
pub use finalize_job::*;
pub use cancel_job::*;

/// Process an FHE Generator instruction
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    use crate::instruction::FheGeneratorInstruction;
    use borsh::BorshDeserialize;

    let instruction = FheGeneratorInstruction::try_from_slice(instruction_data)?;

    match instruction {
        FheGeneratorInstruction::CreateJob {
            job_id,
            circuit_type,
            witness_hash,
            witness_size,
            price_lamports,
            timeout_seconds,
            required_provers,
            consensus_threshold,
            operation_param1,
            operation_param2,
            operation_param3,
        } => process_create_job(
            program_id,
            accounts,
            job_id,
            circuit_type,
            witness_hash,
            witness_size,
            price_lamports,
            timeout_seconds,
            required_provers,
            consensus_threshold,
            operation_param1,
            operation_param2,
            operation_param3,
        ),

        FheGeneratorInstruction::ClaimJob => process_claim_job(program_id, accounts),

        FheGeneratorInstruction::SubmitResult { result_hash } => {
            process_submit_result(program_id, accounts, result_hash)
        }

        FheGeneratorInstruction::FinalizeJob => process_finalize_job(program_id, accounts),

        FheGeneratorInstruction::CancelJob => process_cancel_job(program_id, accounts),
    }
}
