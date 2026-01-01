//! Bedrock instruction processor

use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

pub mod initialize;
pub mod register_prover;
pub mod register_validator;
pub mod slash_prover;
pub mod slash_validator;
pub mod update_prover_stats;

pub use initialize::*;
pub use register_prover::*;
pub use register_validator::*;
pub use slash_prover::*;
pub use slash_validator::*;
pub use update_prover_stats::*;

/// Process a Bedrock instruction
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    use crate::instruction::BedrockInstruction;
    use borsh::BorshDeserialize;

    let instruction = BedrockInstruction::try_from_slice(instruction_data)?;

    match instruction {
        BedrockInstruction::Initialize {
            zk_generator_program,
            fhe_generator_program,
        } => process_initialize(program_id, accounts, zk_generator_program, fhe_generator_program),

        BedrockInstruction::RegisterProver { stake_lamports } => {
            process_register_prover(program_id, accounts, stake_lamports)
        }

        BedrockInstruction::SlashProver { amount, reason } => {
            process_slash_prover(program_id, accounts, amount, reason)
        }

        BedrockInstruction::UpdateProverStats {
            job_completed,
            job_failed,
        } => process_update_prover_stats(program_id, accounts, job_completed, job_failed),

        BedrockInstruction::RegisterValidator {
            endpoint,
            region,
            stake,
        } => process_register_validator(program_id, accounts, endpoint, region, stake),

        BedrockInstruction::SlashValidator { amount, reason } => {
            process_slash_validator(program_id, accounts, amount, reason)
        }
    }
}
