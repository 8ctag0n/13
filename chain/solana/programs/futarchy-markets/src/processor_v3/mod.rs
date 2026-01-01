//! V3 Processors for Fully Blind Markets with FHE+ZK
//!
//! These processors handle instructions using:
//! - MarketV3 accounts (encrypted pools with commitments)
//! - PositionV3 accounts (FHE ciphertext hashes)
//! - PrivateBalance accounts (reused from V2)
//! - Vaults (ProtocolVault, MarketVault)
//! - ZK proofs for blind betting (circuit 50: PlaceBetBlind)
//! - Threshold decryption for settlement

use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

// V3 processor modules
pub mod create_market_v3;
pub mod place_bet_blind;
pub mod update_encrypted_pools;
pub mod settle_market_v3;
pub mod claim_v3;
pub mod execute_governance_action_v3;
pub mod cancel_governance_action_v3;

use crate::instruction_v3::FutarchyInstructionV3;

// Re-exports
pub use create_market_v3::process_create_market_v3;
pub use place_bet_blind::process_place_bet_blind;
pub use update_encrypted_pools::process_update_encrypted_pools;
pub use settle_market_v3::process_settle_market_v3;
pub use claim_v3::process_claim_v3;
pub use execute_governance_action_v3::process_execute_governance_action_v3;
pub use cancel_governance_action_v3::process_cancel_governance_action_v3;

/// Process V3 instruction
pub fn process_v3(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = FutarchyInstructionV3::unpack(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    match instruction {
        FutarchyInstructionV3::CreateMarketV3 {
            market_id,
            question_hash,
            end_time,
            max_bet,
            threshold_pubkey,
            threshold_required,
            threshold_shares,
            has_governance,
            executable_action,
            execution_threshold,
            timelock_duration,
        } => {
            msg!("Instruction V3: CreateMarketV3");
            process_create_market_v3(
                program_id,
                accounts,
                market_id,
                question_hash,
                end_time,
                max_bet,
                threshold_pubkey,
                threshold_required,
                threshold_shares,
                has_governance,
                executable_action,
                execution_threshold,
                timelock_duration,
            )
        }

        FutarchyInstructionV3::PlaceBetBlind {
            market_id,
            bet_ciphertext_hash,
            bet_secret_commitment,
            pool_commitment_after,
            proof,
            public_inputs,
            new_balance_commitment,
            circuit_type,
        } => {
            msg!("Instruction V3: PlaceBetBlind");
            process_place_bet_blind(
                program_id,
                accounts,
                market_id,
                bet_ciphertext_hash,
                bet_secret_commitment,
                pool_commitment_after,
                proof,
                public_inputs,
                new_balance_commitment,
                circuit_type,
            )
        }

        FutarchyInstructionV3::UpdateEncryptedPools {
            market_id,
            encrypted_pool_yes_hash,
            encrypted_pool_no_hash,
            fhe_proof,
        } => {
            msg!("Instruction V3: UpdateEncryptedPools");
            process_update_encrypted_pools(
                program_id,
                accounts,
                market_id,
                encrypted_pool_yes_hash,
                encrypted_pool_no_hash,
                fhe_proof,
            )
        }

        FutarchyInstructionV3::SettleMarketV3 {
            market_id,
            outcome,
            decrypted_pool_yes,
            decrypted_pool_no,
            threshold_signatures,
        } => {
            msg!("Instruction V3: SettleMarketV3");
            process_settle_market_v3(
                program_id,
                accounts,
                market_id,
                outcome,
                decrypted_pool_yes,
                decrypted_pool_no,
                threshold_signatures,
            )
        }

        FutarchyInstructionV3::ClaimV3 {
            market_id,
            bet_ciphertext_hash,
            bet_secret_commitment,
            proof,
            public_inputs,
            new_balance_commitment,
            circuit_type,
        } => {
            msg!("Instruction V3: ClaimV3");
            process_claim_v3(
                program_id,
                accounts,
                market_id,
                bet_ciphertext_hash,
                bet_secret_commitment,
                proof,
                public_inputs,
                new_balance_commitment,
                circuit_type,
            )
        }

        FutarchyInstructionV3::ExecuteGovernanceActionV3 { market_id } => {
            msg!("Instruction V3: ExecuteGovernanceActionV3");
            process_execute_governance_action_v3(program_id, accounts, market_id)
        }

        FutarchyInstructionV3::CancelGovernanceActionV3 { market_id } => {
            msg!("Instruction V3: CancelGovernanceActionV3");
            process_cancel_governance_action_v3(program_id, accounts, market_id)
        }
    }
}

/// Check if instruction data is V3 format
pub fn is_v3_instruction(instruction_data: &[u8]) -> bool {
    if instruction_data.is_empty() {
        return false;
    }
    FutarchyInstructionV3::unpack(instruction_data).is_ok()
}
