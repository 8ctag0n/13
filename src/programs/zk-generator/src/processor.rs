//! ZK Generator instruction processor

use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

pub mod cancel_job;
pub mod claim_job;
pub mod create_job;
pub mod dispute_proof;
pub mod stake;
pub mod submit_proof;

pub use cancel_job::*;
pub use claim_job::*;
pub use create_job::*;
pub use dispute_proof::*;
pub use stake::*;
pub use submit_proof::*;

/// Process a ZK Generator instruction
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    use crate::instruction::ZkGeneratorInstruction;
    use borsh::BorshDeserialize;

    let instruction = ZkGeneratorInstruction::try_from_slice(instruction_data)?;

    match instruction {
        ZkGeneratorInstruction::CreateJob {
            circuit_type,
            witness_hash,
            witness_size,
            price_lamports,
            timeout_seconds,
        } => process_create_job(
            program_id,
            accounts,
            circuit_type,
            witness_hash,
            witness_size,
            price_lamports,
            timeout_seconds,
        ),

        ZkGeneratorInstruction::ClaimJob => process_claim_job(program_id, accounts),

        ZkGeneratorInstruction::SubmitProof { proof_hash } => {
            process_submit_proof(program_id, accounts, proof_hash)
        }

        ZkGeneratorInstruction::DisputeProof { proof, public_inputs } => {
            dispute_proof::process_dispute_proof(program_id, accounts, &proof, &public_inputs)
        }

        ZkGeneratorInstruction::CancelJob => process_cancel_job(program_id, accounts),

        ZkGeneratorInstruction::RegisterProver { stake_amount } => {
            stake::process_register_prover(program_id, accounts, stake_amount)
        }

        ZkGeneratorInstruction::DepositStake { amount } => {
            stake::process_deposit_stake(program_id, accounts, amount)
        }

        ZkGeneratorInstruction::WithdrawStake { amount } => {
            stake::process_withdraw_stake(program_id, accounts, amount)
        }
    }
}
