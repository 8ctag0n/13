//! Instruction processors

use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

pub mod create_market;
pub mod create_market_with_governance;
pub mod place_bet;
pub mod settle_market;
pub mod claim_payout;
pub mod cancel_market;
pub mod update_pool;
pub mod register_user;
pub mod execute_governance_action;
pub mod cancel_governance_action;
pub mod deposit_to_market;
pub mod withdraw_from_escrow;

use crate::instruction::FutarchyInstruction;

pub use create_market::process_create_market;
pub use create_market_with_governance::process_create_market_with_governance;
pub use place_bet::process_place_bet;
pub use settle_market::process_settle_market;
pub use claim_payout::process_claim_payout;
pub use cancel_market::process_cancel_market;
pub use update_pool::process_update_pool;
pub use register_user::process_register_user;
pub use execute_governance_action::process_execute_governance_action;
pub use cancel_governance_action::process_cancel_governance_action;
pub use deposit_to_market::process_deposit_to_market;
pub use withdraw_from_escrow::process_withdraw_from_escrow;

/// Main processor entry point
pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = FutarchyInstruction::unpack(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    match instruction {
        FutarchyInstruction::CreateMarket {
            market_id,
            question_hash,
            end_time,
            max_bet,
        } => {
            msg!("Instruction: CreateMarket");
            process_create_market(program_id, accounts, market_id, question_hash, end_time, max_bet)
        }
        FutarchyInstruction::PlaceBet {
            market_id,
            bet_commitment,
            proof,
            public_inputs,
            amount,
            circuit_type,
            encrypted_bet_amount,
            side,
        } => {
            msg!("Instruction: PlaceBet");
            process_place_bet(
                program_id,
                accounts,
                market_id,
                bet_commitment,
                proof,
                public_inputs,
                amount,
                circuit_type,
                encrypted_bet_amount,
                side,
            )
        }
        FutarchyInstruction::SettleMarket { market_id, outcome } => {
            msg!("Instruction: SettleMarket");
            process_settle_market(program_id, accounts, market_id, outcome)
        }
        FutarchyInstruction::ClaimPayout {
            market_id,
            claim_nullifier,
            proof,
            public_inputs,
            payout_amount,
        } => {
            msg!("Instruction: ClaimPayout");
            process_claim_payout(
                program_id,
                accounts,
                market_id,
                claim_nullifier,
                proof,
                public_inputs,
                payout_amount,
            )
        }
        FutarchyInstruction::CancelMarket { market_id } => {
            msg!("Instruction: CancelMarket");
            process_cancel_market(program_id, accounts, market_id)
        }
        FutarchyInstruction::UpdatePool {
            market_id,
            fhe_job_id,
            encrypted_result,
            side,
        } => {
            msg!("Instruction: UpdatePool");
            process_update_pool(program_id, accounts, market_id, fhe_job_id, encrypted_result, side)
        }
        FutarchyInstruction::RegisterUser {
            proof,
            public_inputs,
            blacklist_root,
            blacklist_version,
        } => {
            msg!("Instruction: RegisterUser");
            process_register_user(
                program_id,
                accounts,
                proof,
                public_inputs,
                blacklist_root,
                blacklist_version,
            )
        }
        FutarchyInstruction::CreateMarketWithGovernance {
            market_id,
            question_hash,
            end_time,
            max_bet,
            executable_action,
            execution_threshold,
            timelock_duration,
        } => {
            msg!("Instruction: CreateMarketWithGovernance");
            process_create_market_with_governance(
                program_id,
                accounts,
                market_id,
                question_hash,
                end_time,
                max_bet,
                executable_action,
                execution_threshold,
                timelock_duration,
            )
        }
        FutarchyInstruction::ExecuteGovernanceAction { market_id } => {
            msg!("Instruction: ExecuteGovernanceAction");
            process_execute_governance_action(program_id, accounts, market_id)
        }
        FutarchyInstruction::CancelGovernanceAction { market_id } => {
            msg!("Instruction: CancelGovernanceAction");
            process_cancel_governance_action(program_id, accounts, market_id)
        }
        FutarchyInstruction::DepositToMarket { market_id, amount } => {
            msg!("Instruction: DepositToMarket");
            process_deposit_to_market(program_id, accounts, market_id, amount)
        }
        FutarchyInstruction::WithdrawFromEscrow { market_id, amount } => {
            msg!("Instruction: WithdrawFromEscrow");
            process_withdraw_from_escrow(program_id, accounts, market_id, amount)
        }
    }
}
