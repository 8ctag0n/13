pub mod initialize;
pub mod register_prover;
pub mod create_job;
pub mod create_job_with_token;
pub mod claim_job;
pub mod submit_proof;
pub mod cancel_job;
pub mod slash_prover;
pub mod submit_fhe_result;
pub mod finalize_fhe_job;

pub use initialize::*;
pub use register_prover::*;
pub use create_job::*;
pub use create_job_with_token::*;
pub use claim_job::*;
pub use submit_proof::*;
pub use cancel_job::*;
pub use slash_prover::*;
pub use submit_fhe_result::*;
pub use finalize_fhe_job::*;

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
        MarketplaceInstruction::RegisterProver { stake_amount, encryption_pubkey } => {
            msg!("Instruction: RegisterProver");
            process_register_prover(program_id, accounts, stake_amount, encryption_pubkey)
        }
        MarketplaceInstruction::CreateJob {
            circuit_type,
            witness_commitment,
            witness_size,
            price_lamports,
            timeout_seconds,
            fhe_config,
        } => {
            msg!("Instruction: CreateJob");
            process_create_job(
                program_id,
                accounts,
                circuit_type,
                witness_commitment,
                witness_size,
                price_lamports,
                timeout_seconds,
                fhe_config,
            )
        }
        MarketplaceInstruction::ClaimJob => {
            msg!("Instruction: ClaimJob");
            process_claim_job(program_id, accounts)
        }
        MarketplaceInstruction::SubmitProof {
            proof_commitment,
            proof_size,
        } => {
            msg!("Instruction: SubmitProof");
            process_submit_proof(program_id, accounts, proof_commitment, proof_size)
        }
        MarketplaceInstruction::CancelJob => {
            msg!("Instruction: CancelJob");
            process_cancel_job(program_id, accounts)
        }
        MarketplaceInstruction::SlashProver { slash_amount } => {
            msg!("Instruction: SlashProver");
            process_slash_prover(program_id, accounts, slash_amount)
        }
        MarketplaceInstruction::SubmitFheResult { result_hash } => {
            msg!("Instruction: SubmitFheResult");
            process_submit_fhe_result(program_id, accounts, result_hash)
        }
        MarketplaceInstruction::FinalizeFheJob => {
            msg!("Instruction: FinalizeFheJob");
            process_finalize_fhe_job(program_id, accounts)
        }
        MarketplaceInstruction::CreateJobWithToken {
            circuit_type,
            witness_commitment,
            witness_size,
            price_token_amount,
            timeout_seconds,
            fhe_config,
        } => {
            msg!("Instruction: CreateJobWithToken");
            process_create_job_with_token(
                program_id,
                accounts,
                circuit_type,
                witness_commitment,
                witness_size,
                price_token_amount,
                timeout_seconds,
                fhe_config,
            )
        }
    }
}
