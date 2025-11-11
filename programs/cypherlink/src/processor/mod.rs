pub mod initialize;
pub mod register_prover;

pub use initialize::*;
pub use register_prover::*;

use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey,
};

use crate::instruction::MarketplaceInstruction;

/// Process instruction
pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = MarketplaceInstruction::unpack(instruction_data)?;

    match instruction {
        MarketplaceInstruction::Initialize {
            fee_basis_points,
            min_stake_amount,
            min_reputation_score,
            default_job_timeout_seconds,
        } => {
            msg!("Instruction: Initialize");
            process_initialize(
                program_id,
                accounts,
                fee_basis_points,
                min_stake_amount,
                min_reputation_score,
                default_job_timeout_seconds,
            )
        }
        MarketplaceInstruction::RegisterProver { stake_amount } => {
            msg!("Instruction: RegisterProver");
            process_register_prover(program_id, accounts, stake_amount)
        }
        MarketplaceInstruction::CreateJob { .. } => {
            msg!("Instruction: CreateJob");
            // TODO: Implement in Day 3
            Ok(())
        }
        MarketplaceInstruction::ClaimJob => {
            msg!("Instruction: ClaimJob");
            // TODO: Implement in Day 3
            Ok(())
        }
        MarketplaceInstruction::SubmitProof { .. } => {
            msg!("Instruction: SubmitProof");
            // TODO: Implement in Day 3
            Ok(())
        }
        MarketplaceInstruction::CancelJob => {
            msg!("Instruction: CancelJob");
            // TODO: Implement in Day 3
            Ok(())
        }
        MarketplaceInstruction::SlashProver { .. } => {
            msg!("Instruction: SlashProver");
            // TODO: Implement in Day 3
            Ok(())
        }
    }
}
