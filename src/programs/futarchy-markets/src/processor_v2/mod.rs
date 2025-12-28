//! V2 Processors for PrivateBalance architecture
//!
//! These processors handle instructions using:
//! - PrivateBalance accounts (FHE + Poseidon commitments)
//! - Protocol/Market Vaults (lamport-only PDAs)
//! - Anonymous PositionV2 (no user field)
//! - Optional GovernanceConfig PDAs

use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

// V2 processor modules
pub mod initialize_protocol;
pub mod create_private_balance;
pub mod deposit_v2;
pub mod withdraw_v2;
pub mod create_market_v2;
pub mod place_bet_v2;
pub mod claim_v2;
pub mod settle_market_v2;
pub mod update_pool_state;
pub mod execute_governance_action_v2;
pub mod cancel_governance_action_v2;

use crate::instruction_v2::FutarchyInstructionV2;

// Re-exports
pub use initialize_protocol::process_initialize_protocol;
pub use create_private_balance::process_create_private_balance;
pub use deposit_v2::process_deposit_v2;
pub use withdraw_v2::process_withdraw_v2;
pub use create_market_v2::process_create_market_v2;
pub use place_bet_v2::process_place_bet_v2;
pub use claim_v2::process_claim_v2;
pub use settle_market_v2::process_settle_market_v2;
pub use update_pool_state::process_update_pool_state;
pub use execute_governance_action_v2::process_execute_governance_action_v2;
pub use cancel_governance_action_v2::process_cancel_governance_action_v2;

/// Process V2 instruction
pub fn process_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = FutarchyInstructionV2::unpack(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    match instruction {
        FutarchyInstructionV2::InitializeProtocol => {
            msg!("Instruction V2: InitializeProtocol");
            process_initialize_protocol(program_id, accounts)
        }

        FutarchyInstructionV2::CreatePrivateBalance {
            initial_encrypted_balance,
            initial_commitment,
        } => {
            msg!("Instruction V2: CreatePrivateBalance");
            process_create_private_balance(
                program_id,
                accounts,
                initial_encrypted_balance,
                initial_commitment,
            )
        }

        FutarchyInstructionV2::Deposit { amount, new_commitment } => {
            msg!("Instruction V2: Deposit");
            process_deposit_v2(program_id, accounts, amount, new_commitment)
        }

        FutarchyInstructionV2::Withdraw {
            amount,
            proof,
            public_inputs,
            new_balance_commitment,
        } => {
            msg!("Instruction V2: Withdraw");
            process_withdraw_v2(
                program_id,
                accounts,
                amount,
                proof,
                public_inputs,
                new_balance_commitment,
            )
        }

        FutarchyInstructionV2::CreateMarketV2 {
            market_id,
            question_hash,
            end_time,
            max_bet,
            has_governance,
            executable_action,
            execution_threshold,
            timelock_duration,
        } => {
            msg!("Instruction V2: CreateMarketV2");
            process_create_market_v2(
                program_id,
                accounts,
                market_id,
                question_hash,
                end_time,
                max_bet,
                has_governance,
                executable_action,
                execution_threshold,
                timelock_duration,
            )
        }

        FutarchyInstructionV2::PlaceBetV2 {
            market_id,
            bet_commitment,
            bet_side,
            proof,
            public_inputs,
            new_balance_commitment,
            encrypted_bet,
            circuit_type,
        } => {
            msg!("Instruction V2: PlaceBetV2");
            process_place_bet_v2(
                program_id,
                accounts,
                market_id,
                bet_commitment,
                bet_side,
                proof,
                public_inputs,
                new_balance_commitment,
                encrypted_bet,
                circuit_type,
            )
        }

        FutarchyInstructionV2::ClaimV2 {
            market_id,
            nullifier_hash,
            proof,
            public_inputs,
            new_balance_commitment,
            encrypted_payout,
            bet_commitment,
            circuit_type,
        } => {
            msg!("Instruction V2: ClaimV2");
            process_claim_v2(
                program_id,
                accounts,
                market_id,
                nullifier_hash,
                proof,
                public_inputs,
                new_balance_commitment,
                encrypted_payout,
                bet_commitment,
                circuit_type,
            )
        }

        FutarchyInstructionV2::SettleMarketV2 { market_id, outcome } => {
            msg!("Instruction V2: SettleMarketV2");
            process_settle_market_v2(program_id, accounts, market_id, outcome)
        }

        FutarchyInstructionV2::UpdatePoolState {
            market_id,
            new_pool_state_root,
            consensus_proof,
        } => {
            msg!("Instruction V2: UpdatePoolState");
            process_update_pool_state(
                program_id,
                accounts,
                market_id,
                new_pool_state_root,
                consensus_proof,
            )
        }

        FutarchyInstructionV2::ExecuteGovernanceActionV2 { market_id } => {
            msg!("Instruction V2: ExecuteGovernanceActionV2");
            process_execute_governance_action_v2(program_id, accounts, market_id)
        }

        FutarchyInstructionV2::CancelGovernanceActionV2 { market_id } => {
            msg!("Instruction V2: CancelGovernanceActionV2");
            process_cancel_governance_action_v2(program_id, accounts, market_id)
        }
    }
}

/// Check if instruction data is V2 format
/// V2 instructions start with specific discriminants from borsh serialization
pub fn is_v2_instruction(instruction_data: &[u8]) -> bool {
    if instruction_data.is_empty() {
        return false;
    }
    // V2 instructions are detected by trying to parse them
    // If borsh can deserialize as FutarchyInstructionV2, it's V2
    FutarchyInstructionV2::unpack(instruction_data).is_ok()
}
