//! ExecuteGovernanceActionV2 - Execute governance action after settlement and timelock
//!
//! Requirements:
//! 1. Market is settled
//! 2. Resolution was YES (threshold met)
//! 3. Timelock has expired
//! 4. Action not already executed

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

use crate::error::FutarchyError;
use crate::state::{
    ExecutableAction, GovernanceConfig, MarketV2,
    GOVERNANCE_CONFIG_SEED, MARKET_V2_SEED,
};

/// Process ExecuteGovernanceActionV2 instruction
///
/// Executes the governance action after all conditions are met
///
/// Accounts expected:
/// 0. `[writable, signer]` Executor (anyone can call)
/// 1. `[]` MarketV2 PDA
/// 2. `[writable]` GovernanceConfig PDA
/// 3. `[]` Clock sysvar
/// ... Additional accounts depend on ExecutableAction type
pub fn process_execute_governance_action_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    market_id: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let executor_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let governance_info = next_account_info(account_info_iter)?;
    let clock_sysvar_info = next_account_info(account_info_iter)?;

    // Verify executor is signer (anyone can execute)
    if !executor_info.is_signer {
        msg!("Executor must be signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let clock = Clock::from_account_info(clock_sysvar_info)?;
    let current_time = clock.unix_timestamp;
    let market_id_bytes = market_id.to_le_bytes();

    // Verify MarketV2 PDA
    let (market_pda, _) = Pubkey::find_program_address(
        &[MARKET_V2_SEED, &market_id_bytes],
        program_id,
    );
    if market_pda != *market_info.key {
        msg!("Invalid MarketV2 PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Load market
    let market_data = market_info.try_borrow_data()?;
    let market = MarketV2::deserialize(&mut &market_data[..])?;
    drop(market_data);

    // Verify market has governance
    if !market.has_governance {
        msg!("Market does not have governance");
        return Err(FutarchyError::NotGovernanceMarket.into());
    }

    // Verify market is settled
    if !market.is_settled() {
        msg!("Market is not settled");
        return Err(FutarchyError::MarketNotSettled.into());
    }

    // Verify resolution was YES
    let resolution = market.resolution.ok_or(FutarchyError::MarketNotSettled)?;
    if !resolution {
        msg!("Market resolution was NO - action not executable");
        return Err(FutarchyError::ThresholdNotMet.into());
    }

    // Verify GovernanceConfig PDA
    let (governance_pda, governance_bump) = Pubkey::find_program_address(
        &[GOVERNANCE_CONFIG_SEED, &market_id_bytes],
        program_id,
    );
    if governance_pda != *governance_info.key {
        msg!("Invalid GovernanceConfig PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // Load governance config
    let governance_data = governance_info.try_borrow_data()?;
    let mut governance = GovernanceConfig::deserialize(&mut &governance_data[..])?;
    drop(governance_data);

    // Verify action not already executed
    if governance.action_executed {
        msg!("Governance action already executed");
        return Err(FutarchyError::ActionAlreadyExecuted.into());
    }

    // Verify timelock has expired
    if !governance.timelock_expired(current_time) {
        msg!("Timelock has not expired yet");
        msg!("  Current time: {}", current_time);
        msg!("  Expires at: {:?}", governance.timelock_expires_at);
        return Err(FutarchyError::TimelockNotExpired.into());
    }

    // Execute the action based on type
    match &governance.executable_action {
        ExecutableAction::None => {
            msg!("No action to execute");
        }

        ExecutableAction::TransferTokens { token_mint, from_treasury, to, amount } => {
            msg!("Executing TransferTokens action");
            msg!("  Token mint: {}", token_mint);
            msg!("  From treasury: {}", from_treasury);
            msg!("  To: {}", to);
            msg!("  Amount: {}", amount);

            // Get remaining accounts for token transfer
            let from_token_info = next_account_info(account_info_iter)?;
            let to_token_info = next_account_info(account_info_iter)?;
            let mint_info = next_account_info(account_info_iter)?;
            let token_program_info = next_account_info(account_info_iter)?;
            let authority_info = next_account_info(account_info_iter)?;

            // Verify accounts match
            if from_token_info.key != from_treasury {
                msg!("From account mismatch");
                return Err(ProgramError::InvalidAccountData);
            }
            if to_token_info.key != to {
                msg!("To account mismatch");
                return Err(ProgramError::InvalidAccountData);
            }
            if mint_info.key != token_mint {
                msg!("Mint account mismatch");
                return Err(ProgramError::InvalidAccountData);
            }

            // Build SPL Token transfer instruction data manually
            // Instruction layout: [3 (transfer discriminator), amount as u64 LE]
            let mut transfer_data = vec![3u8]; // Transfer instruction
            transfer_data.extend_from_slice(&amount.to_le_bytes());

            let transfer_accounts = vec![
                AccountMeta::new(*from_treasury, false),
                AccountMeta::new(*to, false),
                AccountMeta::new_readonly(*authority_info.key, true),
            ];

            let transfer_ix = Instruction {
                program_id: *token_program_info.key,
                accounts: transfer_accounts,
                data: transfer_data,
            };

            invoke(
                &transfer_ix,
                &[
                    from_token_info.clone(),
                    to_token_info.clone(),
                    authority_info.clone(),
                    token_program_info.clone(),
                ],
            )?;

            msg!("Token transfer executed");
        }

        ExecutableAction::UpdateParameter {
            target_program,
            parameter_key,
            parameter_value,
        } => {
            msg!("Executing UpdateParameter action");
            msg!("  Target program: {}", target_program);
            msg!("  Parameter key size: {} bytes", parameter_key.len());
            msg!("  Parameter value size: {} bytes", parameter_value.len());

            // Note: This is a simplified version.
            // In production, you'd need a proper interface with the target program.
            msg!("UpdateParameter execution requires target program interface");
        }

        ExecutableAction::CustomInstruction {
            program_id: target_program,
            data: instruction_data,
            accounts: action_accounts,
        } => {
            msg!("Executing CustomInstruction action");
            msg!("  Target program: {}", target_program);
            msg!("  Instruction data size: {} bytes", instruction_data.len());
            msg!("  Number of accounts: {}", action_accounts.len());

            // Build account metas from the remaining accounts
            let mut account_metas = Vec::new();
            let mut account_infos = Vec::new();

            for action_account in action_accounts.iter() {
                let account_info = next_account_info(account_info_iter)?;

                // Verify account matches
                if account_info.key != &action_account.pubkey {
                    msg!("Account mismatch: expected {}, got {}",
                         action_account.pubkey, account_info.key);
                    return Err(ProgramError::InvalidAccountData);
                }

                account_metas.push(AccountMeta {
                    pubkey: action_account.pubkey,
                    is_signer: action_account.is_signer,
                    is_writable: action_account.is_writable,
                });
                account_infos.push(account_info.clone());
            }

            // Add target program to account infos
            let target_program_info = next_account_info(account_info_iter)?;
            if target_program_info.key != target_program {
                msg!("Target program mismatch");
                return Err(ProgramError::InvalidAccountData);
            }
            account_infos.push(target_program_info.clone());

            let instruction = Instruction {
                program_id: *target_program,
                accounts: account_metas,
                data: instruction_data.clone(),
            };

            invoke(&instruction, &account_infos)?;

            msg!("Custom instruction executed");
        }
    }

    // Mark action as executed
    governance.mark_executed();

    let mut governance_data = governance_info.try_borrow_mut_data()?;
    governance.serialize(&mut &mut governance_data[..])?;

    msg!("Governance action executed successfully");
    msg!("  Market ID: {}", market_id);

    Ok(())
}
